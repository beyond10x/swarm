//! Running the coordinator, and turning what it says into a verdict.
//!
//! The loop is `[loop] -> [coordinator] -> [goal]`. The trigger drives the first arrow and the goal
//! lifecycle closes the third; this is the middle one, and without it a goal reaches `Pursuing` and
//! stays there, because nothing else in the system is allowed to say whether a goal is met.
//!
//! The coordinator is a Claude Code run, driven by `metaharness run claude`. The runtime builds the
//! prompt, starts the run in the swarm's own work directory, reads the event stream as it comes,
//! and takes the verdict from the model's own last words:
//!
//! ```text
//!   VERDICT {"reached": false, "note": "still three tests short"}
//! ```
//!
//! Every `metaharness.event/1` record is passed on to every watcher and written to the turn's file
//! under `data/swarms/<slug>/turns/`, so a run can be watched live and read back later. Token
//! counts are summed from the `usage` events and the cost is read from `session.ended`; neither is
//! computed here.
//!
//! `SWARM_COORDINATOR` chooses: `metaharness` for the real thing, a program path for one that
//! satisfies the older stdin→stdout contract (`examples/coordinator-manual.sh` prints a verdict and
//! nothing else), unset for no coordinator at all — so a restart never spends money on its own.
//!
//! What this must never do is decide the verdict itself. A runtime that answered "reached" on a
//! coordinator's behalf — on a timeout, on a crash, on a budget — would be reporting work nobody
//! did. Every failure here leaves the goal in `Pursuing`, where a person can see it waiting.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::swarm::{Swarm, What};

/// What the coordinator is asked.
#[derive(Debug, Serialize)]
struct Asked<'a> {
    /// The goal, in the words it was set in.
    goal: &'a str,
    /// Which turn of the loop this is.
    iterations: u64,
    /// Which swarm it belongs to.
    swarm: &'a str,
    /// Where it may work: a directory of the swarm's own, kept between turns.
    work: String,
    /// Its unread mail, rendered for the prompt. Empty when there is none.
    #[serde(skip)]
    mail: String,
}

/// What the coordinator answers.
#[derive(Debug, Deserialize)]
pub struct Verdict {
    /// Whether the goal is met. The one thing only the coordinator may say.
    pub reached: bool,
    /// Why, for a person reading the log. Not acted on.
    #[serde(default)]
    pub note: Option<String>,
}

/// What a turn cost, summed from what the vendor reported and never computed here.
///
/// Token counts are summed over `usage` events, one per request: Claude Code reports the same
/// figures once per content block of a request, so a request is counted once. Where the run ends
/// with a `session.ended.usage`, that total replaces the sum — it is the vendor's own figure for
/// the run, and it counts output the per-request records do not (2026-09-12: 1645 output tokens
/// in the terminal record against 11 summed). The cost is `session.ended.total_cost_usd`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Spent {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    /// `None` until the run ended, and `None` for a coordinator that reports no cost.
    pub cost_usd: Option<f64>,
    pub requests: u64,
    pub tool_calls: u64,
    pub model: Option<String>,
    pub duration_ms: Option<u64>,
    /// Billed thinking tokens, where the vendor breaks them out.
    pub thinking_tokens: Option<u64>,
    /// Requests already counted, so a repeated `usage` record is not counted twice.
    #[serde(skip)]
    seen: std::collections::HashSet<String>,
}

impl Spent {
    /// Reads one event's figures into the running total.
    fn absorb(&mut self, event: &Json) {
        match event.get("event").and_then(Json::as_str) {
            Some("usage") => {
                if let Some(request) = event.get("request_id").and_then(Json::as_str)
                    && !self.seen.insert(request.to_owned())
                {
                    return;
                }
                self.requests += 1;
                if let Some(model) = event.get("model").and_then(Json::as_str) {
                    self.model = Some(model.to_owned());
                }
                let usage = event.get("usage");
                let count = |name: &str| {
                    usage
                        .and_then(|usage| usage.get(name))
                        .and_then(Json::as_u64)
                        .unwrap_or_default()
                };
                self.input_tokens += count("input_tokens");
                self.output_tokens += count("output_tokens");
                self.cache_read_tokens += count("cache_read_input_tokens");
                self.cache_write_tokens += count("cache_creation_input_tokens");
            }
            Some("tool.requested") => self.tool_calls += 1,
            Some("session.ended") => {
                self.cost_usd = event.get("total_cost_usd").and_then(Json::as_f64);
                self.duration_ms = event.get("duration_ms").and_then(Json::as_u64);
                // The vendor's own total for the run replaces the sum.
                if let Some(usage) = event.get("usage").filter(|usage| usage.is_object()) {
                    let count = |name: &str| usage.get(name).and_then(Json::as_u64);
                    if let Some(n) = count("input_tokens") {
                        self.input_tokens = n;
                    }
                    if let Some(n) = count("output_tokens") {
                        self.output_tokens = n;
                    }
                    if let Some(n) = count("cache_read_input_tokens") {
                        self.cache_read_tokens = n;
                    }
                    if let Some(n) = count("cache_creation_input_tokens") {
                        self.cache_write_tokens = n;
                    }
                    self.thinking_tokens = count("thinking_tokens");
                }
            }
            _ => {}
        }
    }

    /// Adds another turn's figures to this total.
    pub fn add(&mut self, other: &Spent) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.requests += other.requests;
        self.tool_calls += other.tool_calls;
        if let Some(cost) = other.cost_usd {
            self.cost_usd = Some(self.cost_usd.unwrap_or_default() + cost);
        }
        if let Some(ms) = other.duration_ms {
            self.duration_ms = Some(self.duration_ms.unwrap_or_default() + ms);
        }
        if let Some(n) = other.thinking_tokens {
            self.thinking_tokens = Some(self.thinking_tokens.unwrap_or_default() + n);
        }
        if other.model.is_some() {
            self.model.clone_from(&other.model);
        }
    }
}

/// A turn, answered.
#[derive(Debug)]
pub struct Answer {
    pub verdict: Verdict,
    pub spent: Spent,
}

/// Why a turn could not be finished.
#[derive(Debug)]
pub enum Unfinished {
    /// No coordinator is configured. The goal waits, which is the honest state.
    NoCoordinator,
    /// The program could not be started or did not finish.
    Unrunnable { why: String, spent: Spent },
    /// It ran, and what it said was not a verdict.
    Unreadable {
        output: String,
        why: String,
        spent: Spent,
    },
    /// It gave a verdict, and reporting it was refused.
    Unreportable { why: String, spent: Spent },
}

impl Unfinished {
    /// What the turn cost before it failed, when anything was spent.
    pub fn spent(&self) -> Option<&Spent> {
        match self {
            Self::NoCoordinator => None,
            Self::Unrunnable { spent, .. }
            | Self::Unreadable { spent, .. }
            | Self::Unreportable { spent, .. } => Some(spent),
        }
    }
}

impl std::fmt::Display for Unfinished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCoordinator => f.write_str(
                "no coordinator is configured, so nothing can say whether the goal is met",
            ),
            Self::Unrunnable { why, .. } => write!(f, "the coordinator could not be run: {why}"),
            Self::Unreadable { output, why, .. } => {
                write!(
                    f,
                    "the coordinator did not answer with a verdict ({why}): {output}"
                )
            }
            Self::Unreportable { why, .. } => write!(f, "the verdict could not be reported: {why}"),
        }
    }
}

/// How a swarm's coordinator is launched.
#[derive(Clone, Debug)]
pub enum Launch {
    /// `metaharness run claude`, built here.
    Metaharness,
    /// A program satisfying the stdin→stdout contract.
    Program(Vec<String>),
}

impl Launch {
    /// One line naming it, for a status reader.
    pub fn describe(&self) -> String {
        match self {
            Self::Metaharness => format!(
                "metaharness run claude{}",
                std::env::var("SWARM_COORDINATOR_MODEL")
                    .map(|model| format!(" --model {model}"))
                    .unwrap_or_default()
            ),
            Self::Program(parts) => parts.join(" "),
        }
    }
}

/// Which coordinator a swarm runs.
///
/// Read from the environment for now rather than from the swarm's `Config` record. That is a
/// shortcut and it is worth naming: `swarm.config.HarnessLaunch` already declares `binary`,
/// `model`, `args`, `tool_surface` and `allow_program`, and this should read them once a swarm
/// reliably has an activated config. Until then one setting keeps a demonstration honest rather
/// than inventing a config nobody drafted.
pub fn configured() -> Option<Launch> {
    let raw = std::env::var("SWARM_COORDINATOR").ok()?;
    if raw.trim() == "metaharness" {
        return Some(Launch::Metaharness);
    }
    let parts: Vec<String> = raw.split_whitespace().map(ToOwned::to_owned).collect();
    (!parts.is_empty()).then_some(Launch::Program(parts))
}

/// The settings the `swarm` CLI reads, written into the work directory before every turn.
///
/// A file and not an environment variable, because the hermetic launch builds the child's
/// environment from a seven-key allowlist and nothing of ours is in it.
fn write_settings(
    work: &std::path::Path,
    swarm: &str,
    agent: &str,
    url: &str,
) -> std::io::Result<()> {
    let directory = work.join(".swarm");
    std::fs::create_dir_all(&directory)?;
    let settings = json!({
        "url": url,
        "swarm": swarm,
        "agent": agent,
        "mailbox": "main",
    });
    std::fs::write(
        directory.join("config.json"),
        format!("{}\n", serde_json::to_string_pretty(&settings)?),
    )
}

/// The unread mail, as the prompt carries it.
///
/// Verbatim, and with the id, because the next thing an agent does with a message is read it, act
/// on it and ack it — and it cannot ack what it cannot name. Nothing here marks anything read: the
/// agent does that, or it does not, and a runtime that marked mail read on delivery would be
/// answering for a reader the way it must never answer for a coordinator.
fn mail_section(unread: &[Map<String, Json>]) -> String {
    if unread.is_empty() {
        return String::new();
    }
    let mut text = format!(
        "\n## Your mail — {} unread\n\nRead them with `swarm read <id>`, and `swarm ack <id>` once \
you have dealt with one.\n",
        unread.len()
    );
    for row in unread {
        let field = |name: &str| row.get(name).and_then(Json::as_str).unwrap_or_default();
        text.push_str(&format!(
            "\n- **{}** from `{}` (id `{}`){}\n\n      {}\n",
            field("subject"),
            field("sender_id"),
            field("message_id"),
            if row.get("broadcast_id").and_then(Json::as_str).is_some() {
                " — a broadcast"
            } else {
                ""
            },
            field("body").replace('\n', "\n      "),
        ));
    }
    text
}

/// The prompt a turn starts with.
fn prompt_for(asked: &Asked<'_>) -> String {
    format!(
        "You are the coordinator of the swarm `{swarm}`.\n\n\
The swarm exists to reach one goal:\n\n    {goal}\n\n\
This is turn {turn} of a loop. Each turn you are started fresh in the work directory `{work}`, \
with no memory of earlier turns except what is on disk there. Keep a `NOTES.md` in it: read it \
first, and update it before you finish, so the next turn knows what was done and what is left.\n\n\
Do as much toward the goal as one turn allows. Then decide, honestly, whether the goal as written \
is met. Do not claim it is met to end the loop; the loop only ends when it is.\n\n\
## What you can reach\n\n\
`swarm` is on your PATH and talks to the runtime that started you. It is how you do anything to \
this swarm beyond writing files:\n\n\
      swarm inbox                     what has been said to you\n\
      swarm read <id>                 read one, and mark it read\n\
      swarm ack <id> --note \"...\"     say you have dealt with it\n\
      swarm send --to <agent> --subject S --body B\n\
      swarm broadcast --subject S --body B\n\
      swarm canvas                    what the swarm holds\n\
      swarm view <name>               any view the specification declares\n\
      swarm spec                      every command, and what it takes\n\
      swarm do <command> --input '{{...}}'\n\n\
`swarm do` reaches every command, so you can spawn agents, draw boxes on the canvas and wire \
connections between them. `swarm spec` tells you what those are called; read it before you guess. \
What you may do is decided by the specification, not by the CLI — a refusal comes back naming the \
rule.\n\
{mail}\n\
## Finishing\n\n\
You have a bounded number of steps in this turn, and a turn that runs out before it writes the \
line below produced nothing that could be reported — the goal stays where it was and the whole turn \
is spent again. Leave yourself room: write `NOTES.md` and the verdict before you run out.\n\n\
End your reply with exactly one line, and nothing after it:\n\n\
VERDICT {{\"reached\": true or false, \"note\": \"one sentence on where things stand\"}}\n",
        swarm = asked.swarm,
        goal = asked.goal,
        turn = asked.iterations,
        work = asked.work,
        mail = asked.mail,
    )
}

/// The argv for a metaharness-driven turn.
///
/// `--hermetic` with the native tool surface and `--decisions observe`: the model has Claude Code's
/// own tools inside the work directory, and every call is recorded. The owned surface refuses
/// `observe` by construction (metaharness V4); a frame-narrowed run is the next step, not this one.
fn metaharness_argv(asked: &Asked<'_>) -> Vec<String> {
    let knob =
        |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_owned());
    let mut argv: Vec<String> = [
        "metaharness",
        "run",
        "claude",
        "--hermetic",
        "--tool-surface",
        "native",
        "--decisions",
        "observe",
        "--max-turns",
        &knob("SWARM_COORDINATOR_MAX_TURNS", "30"),
        "--max-budget-usd",
        &knob("SWARM_COORDINATOR_BUDGET_USD", "1.00"),
        "--cwd",
        &asked.work,
        "-p",
        &prompt_for(asked),
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();
    if let Ok(model) = std::env::var("SWARM_COORDINATOR_MODEL") {
        argv.extend(["--model".to_owned(), model]);
    }
    if let Ok(effort) = std::env::var("SWARM_COORDINATOR_EFFORT") {
        argv.extend(["--effort".to_owned(), effort]);
    }
    argv
}

/// The verdict in the model's last words, if it wrote one.
///
/// `VERDICT {...}` on its own line at the end. Anything else is not a verdict, and the turn is
/// left unfinished rather than read charitably.
fn verdict_in(text: &str) -> Option<Verdict> {
    let at = text.rfind("VERDICT")?;
    let rest = text[at + "VERDICT".len()..].trim();
    let start = rest.find('{')?;
    let end = rest.rfind('}')?;
    serde_json::from_str(&rest[start..=end]).ok()
}

/// Where the runtime answers, as the agent must address it.
///
/// The port the server bound, not a guess: `SWARM_PORT` is read the same way `main` reads it.
fn server_url() -> String {
    let port = std::env::var("SWARM_PORT").unwrap_or_else(|_| "5000".to_owned());
    format!("http://localhost:{port}")
}

/// The file one attempt at one turn is written to.
///
/// The attempt is in the name because a turn that fails is retried with the SAME number — the goal
/// did not move — and a name keyed only on the turn would have the retry overwrite the record of
/// what went wrong. Observed 2026-09-12: a first attempt read its mail, ran out of vendor turns
/// before writing a verdict, and its whole transcript was replaced by the attempt that followed.
pub fn turn_file(swarm: &Swarm, goal_id: &str, iterations: u64) -> PathBuf {
    let short: String = goal_id.chars().take(8).collect();
    let directory = swarm.dir().join("turns");
    let mut attempt = 1;
    while directory
        .join(format!("{iterations:04}-{attempt:02}-{short}.jsonl"))
        .exists()
    {
        attempt += 1;
    }
    directory.join(format!("{iterations:04}-{attempt:02}-{short}.jsonl"))
}

/// Asks the coordinator about one goal and reports what it says.
///
/// The goal must already be in `Pursuing`: this answers a turn that the loop started, and a verdict
/// on a goal nobody is pursuing is refused by the specification's own `wrong-state` branch.
///
/// The error carries what the failed turn cost, which is why it is large: a run that spent money
/// and then could not answer must not lose the figure with the verdict.
#[allow(clippy::result_large_err)]
pub async fn take_a_turn(
    swarm: &Arc<Swarm>,
    actor: &str,
    goal_id: &str,
    goal: &str,
    iterations: u64,
) -> Result<Answer, Unfinished> {
    let launch = configured().ok_or(Unfinished::NoCoordinator)?;
    let mut spent = Spent::default();

    let work = swarm.dir().join("work");
    std::fs::create_dir_all(&work).map_err(|why| Unfinished::Unrunnable {
        why: format!("{}: {why}", work.display()),
        spent: spent.clone(),
    })?;

    // Mail is read at the start of a turn. There is no poller and no wake: at the declared cadence
    // a message is never more than one period from being seen, and an interrupt would be a second
    // way into a loop that has one.
    let unread = swarm.unread_for(actor).await;
    if !unread.is_empty() {
        tracing::info!(swarm = %swarm.slug(), agent = actor, unread = unread.len(),
                       "the coordinator has mail");
    }

    let asked = Asked {
        goal,
        iterations,
        swarm: swarm.slug(),
        work: work.display().to_string(),
        mail: mail_section(&unread),
    };

    // What the CLI reads to know which runtime and which swarm it is talking to.
    if let Err(why) = write_settings(&work, swarm.slug(), actor, &server_url()) {
        tracing::warn!(swarm = %swarm.slug(), error = %why,
                       "the agent's settings could not be written; `swarm` will not find them");
    }
    let program = match &launch {
        Launch::Metaharness => metaharness_argv(&asked),
        Launch::Program(parts) => parts.clone(),
    };

    let mut child = Command::new(&program[0])
        .args(&program[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| Unfinished::Unrunnable {
            why: format!("{}: {why}", program[0]),
            spent: spent.clone(),
        })?;

    // A program is handed the question on stdin; metaharness was handed it in the prompt.
    if let Some(mut stdin) = child.stdin.take() {
        if matches!(launch, Launch::Program(_)) {
            let text = serde_json::to_string(&asked).map_err(|why| Unfinished::Unrunnable {
                why: why.to_string(),
                spent: spent.clone(),
            })?;
            let _ = stdin.write_all(text.as_bytes()).await;
        }
        let _ = stdin.shutdown().await;
    }

    // The record of the run, written as it arrives so a crash mid-turn leaves what was seen.
    let record_at = turn_file(swarm, goal_id, iterations);
    let _ = std::fs::create_dir_all(record_at.parent().expect("a parent"));
    let mut record = tokio::fs::File::create(&record_at).await.ok();

    // stderr is drained on its own so a chatty program cannot fill the pipe and stall.
    let stderr = child.stderr.take();
    let drained = tokio::spawn(async move {
        let mut text = String::new();
        if let Some(stderr) = stderr {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if text.len() < 8_000 {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
        }
        text
    });

    let mut last_line = String::new();
    let mut last_text = String::new();
    let mut seq = 0u64;
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            last_line = line.clone();
            let Ok(parsed) = serde_json::from_str::<Json>(&line) else {
                continue;
            };
            if parsed.get("event").is_none() {
                continue;
            }
            seq += 1;
            spent.absorb(&parsed);
            if parsed.get("event").and_then(Json::as_str) == Some("text")
                && let Some(text) = parsed.get("text").and_then(Json::as_str)
            {
                last_text = text.to_owned();
            }
            if let Some(file) = record.as_mut() {
                let _ = file.write_all(line.as_bytes()).await;
                let _ = file.write_all(b"\n").await;
            }
            swarm.announce(What::Agent {
                goal: goal_id.to_owned(),
                iterations,
                seq,
                spent: spent.clone(),
                event: parsed,
            });
        }
    }
    if let Some(mut file) = record.take() {
        let _ = file.flush().await;
    }
    if seq == 0 {
        // Nothing was streamed, so there is no record worth keeping: the manual coordinator's turn
        // is a verdict and nothing else.
        let _ = std::fs::remove_file(&record_at);
    }

    let status = child.wait().await.map_err(|why| Unfinished::Unrunnable {
        why: why.to_string(),
        spent: spent.clone(),
    })?;
    let stderr = drained.await.unwrap_or_default();

    if !status.success() {
        // A coordinator that failed has not reported anything, and must not be read as "not yet":
        // that would be this runtime answering on its behalf.
        return Err(Unfinished::Unrunnable {
            why: format!("exited {status}: {}", stderr.trim()),
            spent,
        });
    }

    let verdict: Verdict = match &launch {
        Launch::Metaharness => verdict_in(&last_text).ok_or_else(|| Unfinished::Unreadable {
            output: last_text.chars().take(300).collect(),
            why: "no VERDICT line in the model's last words".to_owned(),
            spent: spent.clone(),
        })?,
        Launch::Program(_) => {
            serde_json::from_str(last_line.trim()).map_err(|why| Unfinished::Unreadable {
                output: last_line.chars().take(300).collect(),
                why: why.to_string(),
                spent: spent.clone(),
            })?
        }
    };

    if let Some(note) = &verdict.note {
        tracing::info!(swarm = %swarm.slug(), goal = %goal_id, reached = verdict.reached, note = %note,
                       cost = ?spent.cost_usd, "the coordinator reported");
    }

    swarm.record_spend(
        goal_id,
        iterations,
        &spent,
        Some(verdict.reached),
        verdict.note.as_deref(),
    );

    let input = json!({ "goal_id": goal_id, "reached": verdict.reached })
        .as_object()
        .cloned()
        .expect("an object");

    swarm
        .issue(
            Some("swarm.goal.Coordinator"),
            "swarm.goal.Evaluate",
            input,
            &format!("verdict:{goal_id}:{iterations}"),
        )
        .await
        .map_err(|why| Unfinished::Unreportable {
            why: why.to_string(),
            spent: spent.clone(),
        })?;

    Ok(Answer { verdict, spent })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verdict_is_read_from_the_last_line() {
        let text = "Did the work.\n\nVERDICT {\"reached\": false, \"note\": \"two tests left\"}\n";
        let verdict = verdict_in(text).expect("a verdict");
        assert!(!verdict.reached);
        assert_eq!(verdict.note.as_deref(), Some("two tests left"));
    }

    #[test]
    fn the_last_verdict_wins_and_prose_is_not_one() {
        let text =
            "VERDICT {\"reached\": false}\nlater: VERDICT {\"reached\": true, \"note\": \"done\"}";
        assert!(verdict_in(text).expect("a verdict").reached);
        assert!(verdict_in("I think the verdict is that we are done.").is_none());
        assert!(verdict_in("VERDICT {not json}").is_none());
    }

    #[test]
    fn usage_is_summed_and_cost_is_read_not_computed() {
        let mut spent = Spent::default();
        spent.absorb(&serde_json::json!({"event": "usage", "model": "m", "usage": {
            "input_tokens": 2, "output_tokens": 17, "cache_read_input_tokens": 100, "cache_creation_input_tokens": 50}}));
        spent.absorb(&serde_json::json!({"event": "usage", "usage": {"input_tokens": 3, "output_tokens": 1}}));
        spent.absorb(&serde_json::json!({"event": "tool.requested"}));
        assert_eq!(
            (
                spent.input_tokens,
                spent.output_tokens,
                spent.cache_read_tokens,
                spent.cache_write_tokens
            ),
            (5, 18, 100, 50)
        );
        assert_eq!((spent.requests, spent.tool_calls), (2, 1));
        assert_eq!(spent.cost_usd, None);
        spent.absorb(&serde_json::json!({"event": "session.ended", "total_cost_usd": 0.145, "duration_ms": 3000}));
        assert_eq!(spent.cost_usd, Some(0.145));
    }
}

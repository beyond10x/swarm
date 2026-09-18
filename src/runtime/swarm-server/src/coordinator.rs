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
//! Which coordinator runs is resolved in three steps, in this order — see [`resolve`]:
//!
//! ```text
//!   1. the swarm's activated `Config`   swarm.config.HarnessLaunch
//!   2. SWARM_COORDINATOR                a program path, or the word `metaharness`
//!   3. Launch::Metaharness              the default
//! ```
//!
//! The default is `metaharness` and the reason is the frame. It builds `metaharness run claude
//! --hermetic --tool-surface native --decisions frame --frame <file> --max-turns 30
//! --max-budget-usd 1.00`, and the frame is sealed before the process exists: a turn whose frame
//! could not be sealed is not launched, and there is no path from here that falls back to
//! `--decisions observe`, which allowed every call and recorded it.
//!
//! That is refusal at the decision seam and it is **not** containment. `--substrate`,
//! `--substrate-embedded`, `--cgroup-root` and `--write-scope` are refused by name for this arm, so
//! `AGENTS.md`'s standing rule — nothing confines a coordinator — still holds. See
//! [`crate::frame`].
//!
//! [`Launch::Program`] runs arbitrary argv with none of it, which makes
//! `examples/coordinator-manual.sh` the right example and the wrong default.
//!
//! Until 2026-09-12 only step 2 existed, and the cost of that was not the missing steps but the
//! silence: a swarm whose `Config` named a coordinator was told there was none, in the same words
//! as a swarm that had never been given one. Those two are different facts and
//! [`Unfinished::Unusable`] now says which.
//!
//! Step 1 refuses rather than guesses when a swarm has more than one Active config, which is what
//! a swarm has as soon as its operator activates a replacement — `ActivateConfig` does not
//! supersede the row it replaces, whatever `config.yaml` says about it. See `from_config`.
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

/// What a member was posted, as `swarm.agent.OpenAssignments` holds it.
///
/// The three things `swarm/AGENTS.md` §4 step 4 says an assignment post names — "the one outcome,
/// the sign-off it is running under (verbatim, from Timo), and what it may not do" — plus the
/// artifact the work comes from. Carried whole rather than as an id, because every one of them is
/// in the prompt and in the frame, and a turn that had to go and look them up could be given a
/// different answer than the one it was posted.
#[derive(Clone, Debug, Serialize)]
pub struct Assignment {
    pub assignment_id: String,
    pub agent_id: String,
    pub outcome: String,
    pub signoff: String,
    pub forbidden: String,
    pub artifact_ref: Option<String>,
}

impl Assignment {
    /// One row of `swarm.agent.OpenAssignments`, or nothing when the row is not one.
    ///
    /// `None` rather than defaults: an assignment missing its sign-off or its prohibition is not
    /// an assignment with an empty one, and a member launched against a half-read row would be
    /// working to a brief nobody wrote.
    #[must_use]
    pub fn from_row(row: &Map<String, Json>) -> Option<Self> {
        let text = |name: &str| row.get(name).and_then(Json::as_str).map(ToOwned::to_owned);
        Some(Self {
            assignment_id: text("assignment_id")?,
            agent_id: text("agent_id")?,
            outcome: text("outcome")?,
            signoff: text("signoff")?,
            forbidden: text("forbidden")?,
            artifact_ref: text("artifact_ref"),
        })
    }
}

/// What an agent is asked.
#[derive(Debug, Serialize)]
struct Asked<'a> {
    /// The work, in the words it was set in: the goal, or the assignment's one outcome.
    goal: &'a str,
    /// Which turn of the loop this is.
    iterations: u64,
    /// Which swarm it belongs to.
    swarm: &'a str,
    /// Where it may work: a directory of its OWN, kept between turns. Per agent since correction
    /// round 1 of the 2026-09-13a wave — see `run_turn`.
    work: String,
    /// The swarm's shared work directory, which every agent may read and none may write.
    shared: String,
    /// Its unread mail, rendered for the prompt. Empty when there is none.
    #[serde(skip)]
    mail: String,
    /// What the frame admits, by metaharness's own operation names.
    ///
    /// In the prompt as well as in the frame, and that is not redundancy. Measured 2026-09-13: a
    /// framed run still offers all 26 vendor tools and reports `withheld: null`, so a coordinator
    /// that is not told reaches for `Bash`, is refused at the seam, and has spent a billed turn
    /// learning what a sentence could have said. metaharness's flag for saying it up front,
    /// `--scope-announce`, is `b10x` only.
    #[serde(skip)]
    admitted: Vec<String>,
    /// Who is being asked. On the record as well as in the prompt, because two agents in one swarm
    /// answer the same way and a turn that does not say whose it was cannot be attributed.
    agent: &'a str,
    /// The assignment this turn works, when the turn is a member's rather than the coordinator's.
    assignment: Option<&'a Assignment>,
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
    /// Calls refused at the decision seam, of the `tool_calls` that were asked for.
    ///
    /// `tool_calls` counts what the model reached for, which under `--decisions observe` was the
    /// same as what it did. Under a frame those are different numbers, and the difference is the
    /// only figure that says the frame did anything. `serde(default)` because every spend row
    /// written before 2026-09-13 has no such key, and a row that predates a field is not a row
    /// with a zero in it — but zero is the honest reading here: nothing refused it.
    #[serde(default)]
    pub refused: u64,
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
            // The denial record, which metaharness emits for every call in every mode — `allow`
            // as well as `deny`, so this must read the decision rather than count the event.
            Some("tool.decided") => {
                if event
                    .get("decision")
                    .and_then(|decision| decision.get("decision"))
                    .and_then(Json::as_str)
                    == Some("deny")
                {
                    self.refused += 1;
                }
            }
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
        self.refused += other.refused;
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
    /// The swarm's `Config` names a coordinator this host cannot launch.
    ///
    /// Distinct from having none configured, which is no longer a state: resolution ends at
    /// [`Launch::Metaharness`]. Being quietly given the default when your config said something
    /// else is the failure this variant exists to make loud.
    Unusable { why: String },
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
            Self::Unusable { .. } => None,
            Self::Unrunnable { spent, .. }
            | Self::Unreadable { spent, .. }
            | Self::Unreportable { spent, .. } => Some(spent),
        }
    }
}

impl std::fmt::Display for Unfinished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unusable { why } => write!(
                f,
                "the swarm's config names a coordinator that cannot be launched ({why}), and the \
                 default was NOT used in its place"
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
    /// `metaharness run claude`, built here, under a frame admitting these operations.
    ///
    /// The admitted set travels with the launch because it is resolved from the same place the
    /// launch is — the swarm's activated config, or the default when it says nothing. What does
    /// **not** travel with it is where those operations may act: see [`crate::frame::scope`], which
    /// derives that from the swarm's own work directory and takes no input from any config.
    Metaharness { admitted: Vec<String> },
    /// A program satisfying the stdin→stdout contract.
    Program(Vec<String>),
}

impl Launch {
    /// `metaharness run claude` with the default admitted set.
    #[must_use]
    pub fn metaharness() -> Self {
        Self::Metaharness {
            admitted: crate::frame::ADMITTED
                .iter()
                .map(|op| (*op).to_owned())
                .collect(),
        }
    }

    /// One line naming it, for a reader that does not know whose turn it is.
    ///
    /// No `--model`, and that absence is the fix for a finding: this read
    /// `SWARM_COORDINATOR_MODEL` unconditionally, `run_turn` logged it for every turn including a
    /// member's, and `/status` published it as `coordinator.program`. Since members got their own
    /// knobs the argv has been right and this sentence has not, so an operator who set the
    /// coordinator's model was TOLD each member ran `--model <it>` while `metaharness_argv` passed
    /// them none. The knob never leaked into the launch; it leaked into the only account of the
    /// launch anybody sees, which is worse, because nothing contradicts it.
    ///
    /// [`Self::describe_for`] is for a caller that knows which set of knobs the turn reads.
    pub fn describe(&self) -> String {
        match self {
            Self::Metaharness { admitted } => format!(
                "metaharness run claude --decisions frame ({} operations admitted)",
                admitted.len()
            ),
            Self::Program(parts) => parts.join(" "),
        }
    }

    /// The same line, with the model the named knobs ask for.
    ///
    /// `knobs` is `SWARM_COORDINATOR` or `SWARM_MEMBER` — see [`knobs_for`], which is the one
    /// place that decides which a turn reads.
    pub fn describe_for(&self, knobs: &str) -> String {
        let described = self.describe();
        match self {
            Self::Metaharness { .. } => match std::env::var(format!("{knobs}_MODEL")) {
                Ok(model) => format!("{described} --model {model}"),
                Err(_) => described,
            },
            Self::Program(_) => described,
        }
    }
}

/// Where a resolved launch came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The swarm's activated `swarm.config.Config`.
    Config,
    /// `SWARM_COORDINATOR`.
    Environment,
    /// Nothing said, so the contained default.
    Default,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Config => "the swarm's activated config",
            Self::Environment => "SWARM_COORDINATOR",
            Self::Default => "the default",
        })
    }
}

/// A launch, and which of the three sources it came from.
#[derive(Clone, Debug)]
pub struct Resolution {
    pub launch: Launch,
    pub source: Source,
}

impl Resolution {
    /// One line a status reader can show: what will run, and on whose say-so.
    pub fn describe(&self) -> String {
        format!("{} (from {})", self.launch.describe(), self.source)
    }
}

/// Which coordinator this swarm runs: its config, then the environment, then the default.
///
/// A swarm is needed because the first source is a record in its world, and that is the whole
/// reason this is not the free function it used to be. It reads, and decides nothing else.
///
/// The error is the reason a config could not be used, which [`take_a_turn`] wraps in
/// [`Unfinished::Unusable`]. It is a `String` and not an `Unfinished` because the other variants of
/// that enum each carry a `Spent`, and resolution happens before anything has been spent.
pub async fn resolve(swarm: &Swarm) -> Result<Resolution, String> {
    match from_config(swarm).await {
        Ok(Some(launch)) => {
            return Ok(Resolution {
                launch,
                source: Source::Config,
            });
        }
        Ok(None) => {}
        Err(why) => return Err(why),
    }
    Ok(configured().map_or(
        Resolution {
            launch: Launch::metaharness(),
            source: Source::Default,
        },
        |launch| Resolution {
            launch,
            source: Source::Environment,
        },
    ))
}

/// What resolution says when there is no swarm to ask — the environment, then the default.
///
/// For a status reader that reports one figure for the whole server. `state.rs:263` still calls
/// [`configured`] and so still reports `configured: false` for a server that will in fact run
/// metaharness; moving it here is a one-line change in a file this wave does not own.
pub fn fallback() -> Resolution {
    configured().map_or(
        Resolution {
            launch: Launch::metaharness(),
            source: Source::Default,
        },
        |launch| Resolution {
            launch,
            source: Source::Environment,
        },
    )
}

/// The launch `SWARM_COORDINATOR` names, if it names one.
///
/// Step 2 of [`resolve`], and the only step that existed before 2026-09-12.
pub fn configured() -> Option<Launch> {
    let raw = std::env::var("SWARM_COORDINATOR").ok()?;
    if raw.trim() == "metaharness" {
        return Some(Launch::metaharness());
    }
    let parts: Vec<String> = raw.split_whitespace().map(ToOwned::to_owned).collect();
    (!parts.is_empty()).then_some(Launch::Program(parts))
}

/// The launch the swarm's activated `Config` names, if it names one.
///
/// `Ok(None)` is "this config says nothing about a coordinator", which falls through to the next
/// source. `Err` is "it says something this host cannot do", which must NOT fall through — a
/// config silently replaced by the default is the bug this whole path exists to end.
async fn from_config(swarm: &Swarm) -> Result<Option<Launch>, String> {
    let active: Vec<Json> = swarm
        .instances("swarm.config.Config")
        .await
        .into_iter()
        .filter(|config| config.get("state").and_then(Json::as_str) == Some("Active"))
        .collect();

    // `config.yaml:9-12` says one config is Active per swarm at a time, and `ActivateConfig`'s own
    // summary says the previous row is superseded in the same transaction. Neither is true: the
    // same paragraph records that no binding can do it, and nothing else does it either, so
    // activating a replacement — the documented way to change a launch line — leaves BOTH rows
    // Active.
    //
    // This used to be `.find(state == "Active")`. Over a map keyed by generated UUID that is a coin
    // flip, and what it was flipping was which coordinator a swarm launches: a replaced config won
    // about half the time, per swarm, for the life of the process.
    //
    // Ordering by `activated_at` would be the kinder answer and is not available: the `activated`
    // outcome writes no `writes:` and `ConfigActivated` carries no timestamp, so `activated_at` is
    // `null` on every row that has ever existed. There is nothing in the log that says which
    // activation came last.
    //
    // So it refuses. A runtime that picked one would be answering a question the model says cannot
    // be asked, and answering it wrong half the time. The refusal names the rows and the fix, and
    // a swarm with one Active config — which is every swarm whose config was activated once — is
    // untouched by any of this.
    if active.len() > 1 {
        let ids: Vec<&str> = active
            .iter()
            .filter_map(|config| config.get("id").and_then(Json::as_str))
            .collect();
        return Err(format!(
            "{} configs are Active at once ({}), and nothing records which was activated last —              `activated_at` is never written. swarm.config.ActivateConfig does not supersede the              row it replaces, though config.yaml says it does. Supersede all but one with              swarm.config.SupersedeConfig, or set SWARM_COORDINATOR to say which coordinator to run",
            active.len(),
            ids.join(", ")
        ));
    }

    let Some(launch) = active
        .first()
        .and_then(|config| config.get("fields"))
        .and_then(|fields| fields.get("launch"))
    else {
        return Ok(None);
    };

    let text = |name: &str| {
        launch
            .get(name)
            .and_then(Json::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    };

    // A named binary is the operator's explicit choice, and it wins over the harness kind: it is
    // the only way a config can ask for something metaharness does not run.
    if let Some(binary) = text("binary") {
        let mut argv = vec![binary];
        if let Some(args) = launch.get("args").and_then(Json::as_array) {
            argv.extend(args.iter().filter_map(Json::as_str).map(ToOwned::to_owned));
        }
        return Ok(Some(Launch::Program(argv)));
    }

    // What the config narrows the frame to, if it narrows it — and NARROWING IS ALL IT CAN DO,
    // because the list is intersected with `frame::ADMITTED` right here.
    //
    // Without that intersection this field was a confinement the confined party could widen, which
    // is the one property `story:a-turn-is-confined-by-a-frame` exists to deny. `prompt_for` tells
    // every coordinator that `swarm do` reaches every command; `swarm.config.DraftConfig` and
    // `ActivateConfig` are commands; the frame admits `shell`, which is how `swarm do` is called.
    // So a coordinator could draft itself a config naming all ten of `frame::VOCABULARY` and seal
    // itself a frame admitting `subagent.spawn`, `task.todo` and `web.read` — two of which
    // `frame::ADMITTED`'s own doc says are deliberately absent. Measured by the adversary of
    // correction round 2, which resolved exactly those three.
    //
    // The same argument decided in round 1 that the subject scope is derived by the runtime and is
    // not a config field at all. It simply did not reach this field, and the difference is that
    // an operation set CAN be in a config safely — as long as the config can only ever take names
    // away.
    //
    // A name the intersection drops is warned about rather than refused: dropping is narrowing,
    // narrowing is safe, and an operator who asked for `web.read` should be told it was not given
    // rather than have the whole swarm stop. A config whose whole set is dropped is a different
    // thing and is refused below, because a turn admitted nothing cannot write its own verdict and
    // silently handing it the default is the substitution this file exists to end.
    let stated: Option<Vec<String>> = launch
        .get("admitted_operations")
        .and_then(Json::as_array)
        .map(|operations| {
            operations
                .iter()
                .filter_map(Json::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .filter(|operations: &Vec<String>| !operations.is_empty());

    let admitted: Vec<String> = match stated {
        None => crate::frame::ADMITTED
            .iter()
            .map(|operation| (*operation).to_owned())
            .collect(),
        Some(stated) => {
            let (kept, dropped): (Vec<String>, Vec<String>) = stated
                .into_iter()
                .partition(|operation| crate::frame::ADMITTED.contains(&operation.as_str()));
            if !dropped.is_empty() {
                tracing::warn!(
                    swarm = %swarm.slug(), dropped = ?dropped, admitted = ?kept,
                    "the swarm's config named operations this runtime does not admit; they were \
                     dropped, because a config may narrow the frame and may never widen it"
                );
            }
            if kept.is_empty() {
                return Err(format!(
                    "the config admits {dropped:?}, and this runtime admits none of them: the \
                     frame it would seal admits nothing, and a turn that may not read or write \
                     cannot even record its own verdict. The operations a config may name are \
                     {:?}; a config that names none of them is refused rather than quietly given \
                     the default",
                    crate::frame::ADMITTED
                ));
            }
            kept
        }
    };

    match text("harness").as_deref() {
        // `metaharness run claude` is what this host launches, and Claude is what it launches.
        Some("Claude") => Ok(Some(Launch::Metaharness { admitted })),
        Some(other) => Err(format!(
            "harness {other} has no launcher here; only Claude does, through metaharness"
        )),
        None => Ok(None),
    }
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
///
/// It states the frame, and that is not a courtesy. The frame refuses what is **attempted**, not
/// what is offered: a run under a frame admitting one operation still listed all 26 vendor tools
/// with `withheld: null` (measured 2026-09-13). A coordinator that is not told reaches for a tool
/// it can see, is refused at the seam, and has bought that sentence at the price of a turn.
fn prompt_for(asked: &Asked<'_>) -> String {
    if let Some(posted) = asked.assignment {
        return prompt_for_member(asked, posted);
    }
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
rule.\n\n\
## What this turn admits\n\n\
This turn runs under a sealed frame. Your tool list will show you more than the frame admits — \
the list is the vendor's and the frame is not applied to it — so read this rather than the list. \
These operations are admitted, and every other one is refused when you attempt it:\n\n\
      {admitted}\n\n\
They are admitted inside `{work}`, which is yours alone, and that is where you write. The swarm's \
shared directory `{shared}` you may READ and not write: what the other agents of this swarm have \
left is under it, one directory each, and a swarm older than this arrangement may also have files \
loose at its root — an earlier `NOTES.md` among them — left when every agent shared one directory. Every other path is refused — a read, a write or an edit \
outside those two is refused even though the operation itself is admitted. A refusal names the \
operation and the rule, and it is the answer rather than an obstacle: do not route around one.\n\
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
        shared = asked.shared,
        admitted = asked.admitted.join("  "),
        mail = asked.mail,
    )
}

/// The prompt a member's turn starts with.
///
/// A member is not a small coordinator. It is given one outcome, the sign-off that outcome runs
/// under and what it may not do — the three things an assignment post carries — and it answers
/// about THAT, not about the swarm's goal. `swarm.goal.Evaluate` is not reachable from this turn
/// and the prompt does not invite it: a member that reported the goal met would be answering a
/// question only the coordinator may answer.
///
/// The sign-off and the prohibition are rendered verbatim. A runtime that summarised either would
/// be the only place that summary existed, and the member would be working to it.
fn prompt_for_member(asked: &Asked<'_>, posted: &Assignment) -> String {
    format!(
        "You are `{agent}`, a member of the swarm `{swarm}`.\n\n\
You have one assignment, and this is the whole of it:\n\n    {outcome}\n\n\
You are running under this sign-off, verbatim: {signoff}\n\n\
What you may not do: {forbidden}\n\
{artifact}\n\
This is turn {turn} at this assignment. Each turn you are started fresh in the work directory \
`{work}`, with no memory of earlier turns except what is on disk there. Keep a `NOTES.md` in it: \
read it first, and update it before you finish.\n\n\
You are not the coordinator. Whether the SWARM's goal is met is not yours to say and you cannot \
say it; what you report is whether this assignment's one outcome is reached.\n\n\
## What you can reach\n\n\
`swarm` is on your PATH and talks to the runtime that started you:\n\n\
      swarm inbox                     what has been said to you\n\
      swarm read <id>                 read one, and mark it read\n\
      swarm ack <id> --note \"...\"     say you have dealt with it\n\
      swarm send --to <agent> --subject S --body B\n\
      swarm view <name>               any view the specification declares\n\
      swarm do <command> --input '{{...}}'\n\n\
## What this turn admits\n\n\
This turn runs under a sealed frame. Your tool list will show you more than the frame admits — the \
list is the vendor's and the frame is not applied to it — so read this rather than the list. These \
operations are admitted, and every other one is refused when you attempt it:\n\n\
      {admitted}\n\n\
They are admitted inside `{work}`, which is yours alone, and that is where you write. The swarm's \
shared directory `{shared}` you may READ and not write: what the other agents of this swarm have \
left is under it, one directory each, and a swarm older than this arrangement may also have files \
loose at its root — an earlier `NOTES.md` among them — left when every agent shared one directory. Every other path is refused — a read, a write or an edit \
outside those two is refused even though the operation itself is admitted. A refusal names the \
operation and the rule, and it is the answer rather than an obstacle: do not route around one.\n\
{mail}\n\
## Finishing\n\n\
End your reply with exactly one line, and nothing after it:\n\n\
VERDICT {{\"reached\": true or false, \"note\": \"one sentence on where this assignment stands\"}}\n",
        agent = asked.agent,
        swarm = asked.swarm,
        outcome = posted.outcome,
        signoff = posted.signoff,
        forbidden = posted.forbidden,
        artifact = posted
            .artifact_ref
            .as_deref()
            .map(|reference| format!("\nThe work comes from: {reference}\n"))
            .unwrap_or_default(),
        turn = asked.iterations,
        work = asked.work,
        shared = asked.shared,
        admitted = asked.admitted.join("  "),
        mail = asked.mail,
    )
}

/// Which set of environment knobs bounds, prices and models this turn.
///
/// `SWARM_COORDINATOR_*` for the coordinator and `SWARM_MEMBER_*` for a member, and **no fallback
/// from one to the other**. `story:spawn-a-second-agent`'s own Scope calls the coordinator's four
/// knobs "not reusable… because a second agent needs its own", and the reason is money: an operator
/// who sets `SWARM_COORDINATOR_MODEL` for a coordinator that takes one turn a period was, while
/// these were shared, setting it for every member of every swarm at every member's per-turn price.
/// `--max-budget-usd` does not bound that before the fact — measured 2026-09-13, a run launched at
/// `--max-budget-usd 0.01` ended at `total_cost_usd: 0.0710`, because the vendor stops once its own
/// estimate crosses the number rather than before.
///
/// A fallback would have been the convenient answer and it is the defect in another shape: set the
/// coordinator's model, get it for every member, find out on the invoice. So a member with nothing
/// set gets the defaults — `--max-turns 30`, `--max-budget-usd 1.00`, the vendor's default model
/// and no `--effort` — and an operator who wants otherwise for members says so by name.
pub fn knobs_for(assignment: Option<&Assignment>) -> &'static str {
    match assignment {
        None => "SWARM_COORDINATOR",
        Some(_) => "SWARM_MEMBER",
    }
}

/// The argv for a metaharness-driven turn.
///
/// `--hermetic` with the native tool surface and `--decisions frame`: the model holds Claude Code's
/// own tools, and each call is decided from the sealed frame at the decision seam, with no round
/// trip. Until 2026-09-13 this said `--decisions observe`, which metaharness's own help calls "the
/// capture mode, and nothing else" — every call allowed, and recorded.
///
/// # There is no argv without a frame
///
/// `frame` is an `Option` and `None` is an error rather than a fallback, which is the whole of
/// acceptance clause 5. A missing frame that quietly fell back to `observe` would be the failure
/// this story exists to prevent, and it is how `observe` came to be the status quo: by being the
/// thing nobody had to ask for. The refusal is a `String` because it happens before anything has
/// been spent, which is the same reason [`resolve`] returns one.
///
/// # Errors
///
/// When no frame was sealed for this turn.
fn metaharness_argv(
    asked: &Asked<'_>,
    frame: Option<&std::path::Path>,
) -> Result<Vec<String>, String> {
    let Some(frame) = frame else {
        return Err(
            "no frame was sealed for this turn, and a turn with no frame is not launched: \
             `--decisions observe` would allow every call and record it, which is what the frame \
             replaced"
                .to_owned(),
        );
    };
    let frame = frame.display().to_string();
    let knobs = knobs_for(asked.assignment);
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
        "frame",
        "--frame",
        &frame,
        "--max-turns",
        &knob(&format!("{knobs}_MAX_TURNS"), "30"),
        "--max-budget-usd",
        &knob(&format!("{knobs}_BUDGET_USD"), "1.00"),
        "--cwd",
        &asked.work,
        "-p",
        &prompt_for(asked),
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();
    if let Ok(model) = std::env::var(format!("{knobs}_MODEL")) {
        argv.extend(["--model".to_owned(), model]);
    }
    if let Ok(effort) = std::env::var(format!("{knobs}_EFFORT")) {
        argv.extend(["--effort".to_owned(), effort]);
    }
    Ok(argv)
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
///
/// The AGENT is in the name for the same reason, one level up. While a swarm had one agent the
/// name did not need to say which; two agents working one goal would have written the same name,
/// and the second would have replaced the first's transcript exactly as that retry did. A
/// transcript nobody can attribute is the thing this system exists to keep.
pub fn turn_file(swarm: &Swarm, agent: &str, unit_id: &str, iterations: u64) -> PathBuf {
    turn_file_attempt(swarm, agent, unit_id, iterations).0
}

/// The same file, and which attempt it is.
///
/// The number is not a detail of the name: the sealed frame states it (`step.attempt`) and
/// metaharness renders it into the instruction the model is shown, so a runtime that computed it
/// for the file name and then wrote `1` into the frame told every retry it was a first attempt.
/// [`turn_file`] is the projection that drops it, and stays because callers outside this module
/// want the path alone.
fn turn_file_attempt(swarm: &Swarm, agent: &str, unit_id: &str, iterations: u64) -> (PathBuf, u32) {
    let short: String = unit_id.chars().take(8).collect();
    let agent = agent_directory(agent);
    let directory = swarm.dir().join("turns");
    let name = |attempt: u32| format!("{iterations:04}-{attempt:02}-{agent}-{short}.jsonl");
    let mut attempt = 1;
    while directory.join(name(attempt)).exists() {
        attempt += 1;
    }
    (directory.join(name(attempt)), attempt)
}

/// An agent's slug as a single path segment: safe, and never the same segment for two slugs.
///
/// A slug is a role name — `improver`, `cv2-aep`, `disk-warden` — and nothing in the specification
/// forbids one holding a `/` or a `..`, because `agent.yaml`'s identity is a `String` chosen by
/// whoever issues `Spawn`, and a coordinator issues `Spawn` through `swarm do`. So the slug is
/// reduced here, at the one place a slug becomes a path: a work directory named by an agent that
/// could climb out of the swarm's own directory would be a confinement its subject writes.
///
/// `[A-Za-z0-9_-]` survives unchanged, which is every slug in all eleven event logs. Anything else
/// is reduced to `-` AND the segment gains eight hex characters of the raw slug's digest, because
/// reduction alone is not injective: `qa/1`, `qa.1` and `qa 1` all reduce to `qa-1` and would have
/// shared one directory — which is the defect the per-agent split exists to end, reappearing for
/// names nobody has used yet. `_` and `-` are kept apart rather than folded together for the same
/// reason.
///
/// The digest is over the slug as given, so the mapping is stable across restarts and two distinct
/// slugs cannot collide: equal segments imply equal digests imply equal slugs, up to SHA-256.
fn agent_directory(agent: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if !agent.is_empty() && agent.chars().all(plain) && agent != "-" && agent != "_" {
        return agent.to_owned();
    }
    let reduced: String = agent
        .chars()
        .map(|c| if plain(c) { c } else { '-' })
        .collect();
    let digest: String = crate::frame::digest_of_bytes(agent.as_bytes())
        .chars()
        .take(8)
        .collect();
    if reduced.is_empty() {
        format!("agent-{digest}")
    } else {
        format!("{reduced}-{digest}")
    }
}

/// The frame that turn ran under, beside its transcript.
///
/// Kept rather than written to a scratch directory and forgotten: what a turn was admitted to do
/// is as much a part of its record as what it did, and a reader asking "why was that refused"
/// needs the document that refused it.
///
/// Private, and driven rather than called: `the_frame_a_turn_ran_under_is_named_by_the_runtime…`
/// runs the metaharness arm against a stand-in, reads the `--frame` the process was given, and
/// asserts the NAME relates to the transcript's. Until correction round 1 this had no test caller
/// at all and could be pointed anywhere with the suite staying green; a case that called it and
/// compared would have had the same hole, because both sides move together.
fn frame_file(record_at: &std::path::Path) -> PathBuf {
    record_at.with_extension("frame.json")
}

/// Persists usage already observed if the owning future is dropped rather than returning normally.
struct AbandonedTurn<'a> {
    swarm: &'a Swarm,
    actor: &'a str,
    unit: &'a str,
    iteration: u64,
    observed: Option<Spent>,
    returned: bool,
}
impl Drop for AbandonedTurn<'_> {
    fn drop(&mut self) {
        if !self.returned
            && let Some(spent) = &self.observed
        {
            self.swarm.record_spend(
                self.actor,
                self.unit,
                self.iteration,
                spent,
                None,
                Some("turn task dropped before completion"),
            );
        }
    }
}
/// One agent's turn at one unit of work: the launch, the stream, the record and the verdict.
///
/// Everything the coordinator's turn and a member's turn have in common is here, and everything
/// they do not is in [`Asked`] — the prompt — and in what the caller does with the answer. They
/// were one function while a swarm had one agent, and the parts that differ are small: which
/// workflow the frame names, what the turn is about, and which command the verdict becomes.
///
/// What it does NOT do is issue anything to the specification. A turn that reports its own verdict
/// AND acts on it would be two decisions in one place; the callers below each make exactly one.
///
/// The error carries what the failed turn cost, which is why it is large: a run that spent money
/// and then could not answer must not lose the figure with the verdict.
#[allow(clippy::result_large_err)]
async fn run_turn(
    swarm: &Arc<Swarm>,
    actor: &str,
    unit_id: &str,
    iterations: u64,
    goal: &str,
    assignment: Option<&Assignment>,
) -> Result<Answer, Unfinished> {
    let mut evidence = AbandonedTurn {
        swarm,
        actor,
        unit: unit_id,
        iteration: iterations,
        observed: None,
        returned: false,
    };
    let result = run_turn_inner(
        swarm,
        actor,
        unit_id,
        iterations,
        goal,
        assignment,
        &mut evidence.observed,
    )
    .await;
    evidence.returned = true;
    result
}
#[allow(clippy::result_large_err)]
async fn run_turn_inner(
    swarm: &Arc<Swarm>,
    actor: &str,
    unit_id: &str,
    iterations: u64,
    goal: &str,
    assignment: Option<&Assignment>,
    observed: &mut Option<Spent>,
) -> Result<Answer, Unfinished> {
    let resolved = resolve(swarm)
        .await
        .map_err(|why| Unfinished::Unusable { why })?;
    tracing::info!(swarm = %swarm.slug(), unit = %unit_id, agent = %actor,
                   source = %resolved.source,
                   launch = %resolved.launch.describe_for(knobs_for(assignment)),
                   "the session was resolved");
    let launch = resolved.launch;
    let mut spent = Spent::default();

    // One directory per AGENT, inside the swarm's shared one. The prompt tells every agent to keep
    // a `NOTES.md` in its work directory and to read it first; while every agent was given the
    // swarm's, two members of one swarm wrote over each other's memory of what they had done —
    // in a wave whose entire point is that there are two members. The shared directory stays, and
    // stays readable (`frame::scope`), so nothing a coordinator left is lost to a member.
    let shared = swarm.dir().join("work");
    let work = shared.join(agent_directory(actor));
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
                       "the agent has mail");
    }

    let asked = Asked {
        goal,
        iterations,
        swarm: swarm.slug(),
        agent: actor,
        work: work.display().to_string(),
        shared: shared.display().to_string(),
        mail: mail_section(&unread),
        admitted: match &launch {
            Launch::Metaharness { admitted } => admitted.clone(),
            Launch::Program(_) => Vec::new(),
        },
        assignment,
    };

    // What the CLI reads to know which runtime and which swarm it is talking to.
    if let Err(why) = write_settings(&work, swarm.slug(), actor, &server_url()) {
        tracing::warn!(swarm = %swarm.slug(), error = %why,
                       "the agent's settings could not be written; `swarm` will not find them");
    }

    // The record of the run, named before the run so the frame can be named after it — and the
    // attempt, which the frame states and which the runtime has known all along.
    let (record_at, attempt) = turn_file_attempt(swarm, actor, unit_id, iterations);
    let _ = std::fs::create_dir_all(record_at.parent().expect("a parent"));

    let program = match &launch {
        // The frame is sealed BEFORE the process exists, and a turn whose frame could not be
        // sealed does not become a process. That is acceptance clause 5: there is no path from
        // here to a launch with no frame, and in particular none that falls back to `observe`.
        Launch::Metaharness { admitted } => {
            let frame_at = crate::frame::write(
                &frame_file(&record_at),
                &crate::frame::Turn {
                    workflow: match assignment {
                        None => "swarm/coordinator",
                        Some(_) => "swarm/assignment",
                    },
                    state: match assignment {
                        None => "pursuing",
                        Some(_) => "working",
                    },
                    index: u32::try_from(iterations).unwrap_or(u32::MAX),
                    // Reported, not invented. metaharness renders this into the instruction the
                    // model is shown — `Frame::render_instruction`: "step {index} attempt
                    // {attempt}" — so a frame that always said 1 told every retry it was a first
                    // attempt. `turn_file` has counted the attempts on disk since 2026-09-12,
                    // after a first attempt ran out of vendor turns and its transcript was
                    // replaced by the retry's.
                    attempt,
                    obligations: match assignment {
                        None => vec![
                            "end the reply with exactly one VERDICT line and nothing after it"
                                .to_owned(),
                            "do not claim the goal is met to end the loop".to_owned(),
                        ],
                        // Verbatim, and never summarised: the sign-off and the prohibition are the
                        // two things an assignment post carries besides the outcome, and this is
                        // the only place either of them is written down for the turn.
                        Some(posted) => vec![
                            "end the reply with exactly one VERDICT line and nothing after it"
                                .to_owned(),
                            format!("the sign-off this runs under: {}", posted.signoff),
                            format!("what it may not do: {}", posted.forbidden),
                        ],
                    },
                    reaching: vec![match assignment {
                        None => "to finish: the goal as written is met".to_owned(),
                        Some(posted) => format!("to finish: {}", posted.outcome),
                    }],
                    operations: admitted.clone(),
                    work: &work,
                    shared: &shared,
                },
            )
            .map_err(|why| Unfinished::Unusable {
                why: why.to_string(),
            })?;
            metaharness_argv(&asked, Some(&frame_at)).map_err(|why| Unfinished::Unusable { why })?
        }
        Launch::Program(parts) => parts.clone(),
    };

    let mut command = Command::new(&program[0]);
    command
        .args(&program[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = swarm
        .control
        .spawn(&mut command)
        .map_err(|why| Unfinished::Unrunnable {
            why: format!("{}: {why}", program[0]),
            spent: spent.clone(),
        })?;

    *observed = Some(spent.clone());
    let stdin = child.stdin.take();
    let stderr = child.stderr.take();
    let stdout = child.stdout.take();
    let reaped = swarm.control.supervise(child);
    // A program is handed the question on stdin; metaharness was handed it in the prompt.
    if let Some(mut stdin) = stdin {
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
    let mut record = tokio::fs::File::create(&record_at).await.ok();

    // stderr is drained on its own so a chatty program cannot fill the pipe and stall.
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
    if let Some(stdout) = stdout {
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
            *observed = Some(spent.clone());
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
                agent: actor.to_owned(),
                goal: unit_id.to_owned(),
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

    let status = reaped
        .await
        .map_err(|why| Unfinished::Unrunnable {
            why: why.to_string(),
            spent: spent.clone(),
        })?
        .map_err(|why| Unfinished::Unrunnable {
            why: why.to_string(),
            spent: spent.clone(),
        })?;
    let stderr = drained.await.unwrap_or_default();

    if let Err(why) = swarm.control.check() {
        return Err(Unfinished::Unrunnable { why, spent });
    }
    if !status.success() {
        // A coordinator that failed has not reported anything, and must not be read as "not yet":
        // that would be this runtime answering on its behalf.
        return Err(Unfinished::Unrunnable {
            why: format!("exited {status}: {}", stderr.trim()),
            spent,
        });
    }

    let verdict: Verdict = match &launch {
        Launch::Metaharness { .. } => {
            verdict_in(&last_text).ok_or_else(|| Unfinished::Unreadable {
                output: last_text.chars().take(300).collect(),
                why: "no VERDICT line in the model's last words".to_owned(),
                spent: spent.clone(),
            })?
        }
        Launch::Program(_) => {
            serde_json::from_str(last_line.trim()).map_err(|why| Unfinished::Unreadable {
                output: last_line.chars().take(300).collect(),
                why: why.to_string(),
                spent: spent.clone(),
            })?
        }
    };

    if let Some(note) = &verdict.note {
        tracing::info!(swarm = %swarm.slug(), unit = %unit_id, agent = %actor,
                       reached = verdict.reached, note = %note,
                       cost = ?spent.cost_usd, "the agent reported");
    }

    // The answered turn is recorded by the same attributed door as the unfinished one
    // (`trigger.rs`). It was not, for a few hours, and every successful turn wrote `"agent": null`.
    swarm.record_spend(
        actor,
        unit_id,
        iterations,
        &spent,
        Some(verdict.reached),
        verdict.note.as_deref(),
    );

    Ok(Answer { verdict, spent })
}

/// Asks the coordinator about one goal and reports what it says.
///
/// The goal must already be in `Pursuing`: this answers a turn that the loop started, and a verdict
/// on a goal nobody is pursuing is refused by the specification's own `wrong-state` branch.
#[allow(clippy::result_large_err)]
pub async fn take_a_turn(
    swarm: &Arc<Swarm>,
    actor: &str,
    goal_id: &str,
    goal: &str,
    iterations: u64,
) -> Result<Answer, Unfinished> {
    if crate::control::Control::in_turn() {
        return take_a_turn_claimed(swarm, actor, goal_id, goal, iterations).await;
    }
    let claim = swarm
        .control
        .admit(format!("goal:{}", goal_id))
        .ok_or_else(|| Unfinished::Unusable {
            why: "swarm is not running or unit already has a turn".to_owned(),
        })?;
    claim
        .scope(take_a_turn_claimed(swarm, actor, goal_id, goal, iterations))
        .await
}

#[allow(clippy::result_large_err)]
async fn take_a_turn_claimed(
    swarm: &Arc<Swarm>,
    actor: &str,
    goal_id: &str,
    goal: &str,
    iterations: u64,
) -> Result<Answer, Unfinished> {
    let answer = run_turn(swarm, actor, goal_id, iterations, goal, None).await?;

    let input = json!({ "goal_id": goal_id, "reached": answer.verdict.reached })
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
            spent: answer.spent.clone(),
        })?;

    Ok(answer)
}

/// Runs one member against the assignment it was posted, and reports what it says.
///
/// The member must already have TAKEN the assignment — `swarm.agent.TakeAssignment`, which moves it
/// to `Working` — and that is the caller's to issue, before this. The order is not arbitrary: a
/// session that ran without a `take` is work with no record that it started, and the domain's own
/// lifecycle would never leave `Assigned`.
///
/// A member's verdict is about its assignment and about nothing else. It does not reach
/// `swarm.goal.Evaluate`: whether the swarm's GOAL is met is the coordinator's to say, and a member
/// that could answer it would be the runtime's own "reported work nobody did" in a new shape.
#[allow(clippy::result_large_err)]
pub async fn work_an_assignment(
    swarm: &Arc<Swarm>,
    assignment: &Assignment,
    iterations: u64,
) -> Result<Answer, Unfinished> {
    if crate::control::Control::in_turn() {
        return work_an_assignment_claimed(swarm, assignment, iterations).await;
    }
    let claim = swarm
        .control
        .admit(format!("assignment:{}", assignment.assignment_id))
        .ok_or_else(|| Unfinished::Unusable {
            why: "swarm is not running or unit already has a turn".to_owned(),
        })?;
    claim
        .scope(work_an_assignment_claimed(swarm, assignment, iterations))
        .await
}

#[allow(clippy::result_large_err)]
async fn work_an_assignment_claimed(
    swarm: &Arc<Swarm>,
    assignment: &Assignment,
    iterations: u64,
) -> Result<Answer, Unfinished> {
    let answer = run_turn(
        swarm,
        &assignment.agent_id,
        &assignment.assignment_id,
        iterations,
        &assignment.outcome,
        Some(assignment),
    )
    .await?;

    // Only a member that says it is finished finishes the record. A verdict of `false` leaves the
    // assignment Assigned and the member Working, so the next period continues it rather than
    // taking it again — which is what `TakeAssignment`'s own `wrong-state` branch would refuse.
    if answer.verdict.reached {
        #[cfg(test)]
        swarm.control.before_result().await;
        // Finishing the assignment and returning its worker to Idle are one logical result.
        // Pause/stop must precede both commands or follow both; a Done assignment is no longer
        // eligible for dispatch and cannot repair a worker stranded between these writes.
        let _publication =
            swarm
                .control
                .publish()
                .await
                .map_err(|why| Unfinished::Unreportable {
                    why,
                    spent: answer.spent.clone(),
                })?;
        let input = json!({"assignment_id": assignment.assignment_id,
                           "note": answer.verdict.note})
        .as_object()
        .cloned()
        .expect("an object");
        swarm
            .issue(
                Some("swarm.agent.Worker"),
                "swarm.agent.FinishAssignment",
                input,
                &format!("finish:{}:{iterations}", assignment.assignment_id),
            )
            .await
            .map_err(|why| Unfinished::Unreportable {
                why: why.to_string(),
                spent: answer.spent.clone(),
            })?;

        // And the member itself goes Idle. `FinishAssignment` moves the ASSIGNMENT and leaves the
        // agent Working, and `Assign` runs from `Spawned` or `Idle` only — so a member that
        // finished and stayed Working could never be retasked, and "retasking a member is a new
        // post, not an edit" would be unreachable for every member that ever finished anything.
        // eventlog.md: "your task is done or your brief has nothing left. Emit this explicitly."
        let idle = json!({"agent_id": assignment.agent_id,
                          "msg": format!("finished: {}", assignment.outcome)})
        .as_object()
        .cloned()
        .expect("an object");
        if let Err(why) = swarm
            .issue(
                Some("swarm.agent.Worker"),
                "swarm.agent.GoIdle",
                idle,
                &format!("idle:{}:{iterations}", assignment.assignment_id),
            )
            .await
        {
            tracing::warn!(swarm = %swarm.slug(), agent = %assignment.agent_id, error = %why,
                           "the member finished its assignment and could not go idle, so it \
                            cannot be retasked until something moves it");
        }
    }

    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One turn's question, with the frame's admitted set on it.
    fn asked() -> Asked<'static> {
        Asked {
            goal: "ship the thing",
            iterations: 3,
            swarm: "s",
            work: "/data/swarms/s/work/coordinator".to_owned(),
            shared: "/data/swarms/s/work".to_owned(),
            agent: "coordinator",
            mail: String::new(),
            admitted: crate::frame::ADMITTED
                .iter()
                .map(|op| (*op).to_owned())
                .collect(),
            assignment: None,
        }
    }

    /// One posted assignment, for the cases that ask as a member rather than as the coordinator.
    pub(super) fn an_assignment() -> Assignment {
        Assignment {
            assignment_id: "aaaaaaaa-0000-0000-0000-000000000001".to_owned(),
            agent_id: "builder".to_owned(),
            outcome: "make the suite green".to_owned(),
            signoff: "Timo, verbatim".to_owned(),
            forbidden: "do not touch the planning store".to_owned(),
            artifact_ref: Some("story:spawn-a-second-agent".to_owned()),
        }
    }

    /// The same turn, asked of a member working that assignment.
    fn asked_as_member<'a>(posted: &'a Assignment) -> Asked<'a> {
        Asked {
            agent: "builder",
            work: "/data/swarms/s/work/builder".to_owned(),
            shared: "/data/swarms/s/work".to_owned(),
            assignment: Some(posted),
            ..asked()
        }
    }

    /// `#[test]` without a runtime, for the one case that needs the environment lock.
    fn futures_lite_block<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime")
            .block_on(future)
    }

    /// The value a flag was given, by the flag's name.
    fn value_of<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
        argv.iter()
            .position(|part| part == flag)
            .and_then(|at| argv.get(at + 1))
            .map(String::as_str)
    }

    /// Frame clause 1: the turn launches narrowed, and `observe` is nowhere in it.
    ///
    /// `--decisions observe` is metaharness's capture mode — every call allowed and recorded — and
    /// it was this runtime's launch line until 2026-09-13. The second half of the assertion is the
    /// one that matters in a year: a frame that is passed *and* an `observe` left beside it is the
    /// capture mode with extra steps.
    #[test]
    fn a_turn_launches_under_a_frame_and_never_under_observe() {
        let frame = std::path::Path::new("/data/swarms/s/turns/0003-01-abcd.frame.json");
        let argv = metaharness_argv(&asked(), Some(frame)).expect("a framed turn builds an argv");

        assert_eq!(
            value_of(&argv, "--frame"),
            Some("/data/swarms/s/turns/0003-01-abcd.frame.json"),
            "the sealed document the adapter decides from: {argv:?}"
        );
        assert_eq!(
            value_of(&argv, "--decisions"),
            Some("frame"),
            "decided from the frame, with no round trip: {argv:?}"
        );
        assert!(
            !argv.iter().any(|part| part == "observe"),
            "`observe` allows every call and records it; it must appear nowhere: {argv:?}"
        );
    }

    /// Frame clause 5: there is no argv for a metaharness turn without a frame.
    ///
    /// Separate from the digest check because the failure this guards is not a bad frame but a
    /// missing one silently falling back to `observe` — which is how `observe` became the status
    /// quo in the first place: by being the thing nobody had to ask for.
    #[test]
    fn a_metaharness_turn_with_no_frame_has_no_argv_at_all() {
        let why =
            metaharness_argv(&asked(), None).expect_err("a turn with no frame does not launch");
        assert!(
            why.contains("frame"),
            "the refusal says what is missing: {why}"
        );
    }

    /// The frame refuses what is attempted, not what is offered, so the prompt has to say it.
    ///
    /// Measured: under a frame admitting one operation the model was still offered all 26 vendor
    /// tools, reached for `Bash`, and was refused. At roughly a dollar a turn, a coordinator
    /// discovering its own boundary by being refused is a turn bought for nothing.
    #[test]
    fn the_prompt_states_what_the_frame_admits_and_where() {
        let asked = asked();
        let prompt = prompt_for(&asked);
        for operation in &asked.admitted {
            assert!(
                prompt.contains(operation.as_str()),
                "the prompt names {operation}, because the tool list will not: {prompt}"
            );
        }
        assert!(
            prompt.contains("/data/swarms/s/work"),
            "and where those operations may act"
        );
        assert!(
            prompt.contains("refused"),
            "and that a call outside them is refused rather than merely discouraged"
        );
    }

    /// Every agent has its own work directory, and the prompt has to say which is which.
    ///
    /// The scope admits writes in the agent's own and reads in the swarm's shared one. An agent
    /// told only "you may work in X" would never look in the shared one, and an agent told nothing
    /// about the difference would try to write there and spend a turn being refused. Both prompts
    /// carry it, because both agents are under the same scope.
    #[test]
    fn both_prompts_say_where_an_agent_writes_and_where_it_may_only_read() {
        let posted = an_assignment();
        for (who, prompt) in [
            ("the coordinator", prompt_for(&asked())),
            ("a member", prompt_for(&asked_as_member(&posted))),
        ] {
            let own = if who == "a member" {
                "/data/swarms/s/work/builder"
            } else {
                "/data/swarms/s/work/coordinator"
            };
            assert!(
                prompt.contains(own),
                "{who} is told its OWN directory, which is where it writes: {prompt}"
            );
            assert!(
                prompt.contains("/data/swarms/s/work`"),
                "{who} is told the shared one, which it may read: {prompt}"
            );
            assert!(
                prompt.contains("read") && prompt.contains("not write"),
                "{who} is told which of the two it may only read: {prompt}"
            );
            assert!(
                !prompt.contains("and nowhere else"),
                "{who} is not told its own directory is the whole of what it can reach, because \
                 it is not: {prompt}"
            );
        }
    }

    /// Frame clause 2, the half that is this runtime's: a refusal at the seam is counted, not just
    /// streamed past.
    ///
    /// metaharness decides every call and emits `tool.decided` for it whatever the decision, so a
    /// refused call is already in the turn's file. What a reader of the turn's FIGURES had was
    /// `tool_calls`, which counts what was asked for and says nothing about what was allowed — a
    /// turn that asked for thirty tools and was refused all thirty read the same as one that ran
    /// them. The count is what makes "it was refused" answerable without reading the transcript.
    #[test]
    fn a_refusal_at_the_seam_is_counted_and_not_merely_streamed_past() {
        let mut spent = Spent::default();
        spent.absorb(&json!({"event": "tool.requested", "call_id": "t1", "tool_name": "Bash"}));
        spent.absorb(
            &json!({"event": "tool.decided", "call_id": "t1", "decided_by": "frame",
                             "seam": "hook", "decision": {"decision": "deny",
                             "reason": "this step admits [\"file.read\"] and Bash is [\"shell\"], \
                                        which it does not"}}),
        );
        assert_eq!(
            (spent.tool_calls, spent.refused),
            (1, 1),
            "one call asked for, one refused"
        );

        spent.absorb(
            &json!({"event": "tool.decided", "call_id": "t2", "decided_by": "frame",
                             "seam": "hook", "decision": {"decision": "allow"}}),
        );
        assert_eq!(
            spent.refused, 1,
            "an allowed call is not a refusal, whoever decided it"
        );

        let mut turn = Spent::default();
        turn.add(&spent);
        assert_eq!(
            turn.refused, 1,
            "and a total over turns carries it, like every other figure here"
        );
    }

    /// A member is not priced, bounded or modelled by the coordinator's knobs.
    ///
    /// `SWARM_COORDINATOR_MODEL` names the coordinator, and while there was one agent it was also
    /// the only agent. An operator who set it for a coordinator that takes one turn a period was
    /// silently setting it for every member of every swarm at every member's per-turn price — and
    /// `--max-budget-usd` is not a pre-spend bound (measured 2026-09-13: a run launched at $0.01
    /// ended at $0.0710, seven times the cap), so the multiplier is real money.
    ///
    /// There is deliberately NO fallback from `SWARM_MEMBER_*` to `SWARM_COORDINATOR_*`. A
    /// fallback would reproduce the defect exactly: set the coordinator's model, get it for every
    /// member, discover it on the invoice.
    #[test]
    fn a_member_is_bounded_by_its_own_knobs_and_never_by_the_coordinators() {
        let _guard = futures_lite_block(crate::ENVIRONMENT.lock());
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_COORDINATOR_MAX_TURNS", "99");
            std::env::set_var("SWARM_COORDINATOR_BUDGET_USD", "50.00");
            std::env::set_var("SWARM_COORDINATOR_MODEL", "an-expensive-model");
            std::env::set_var("SWARM_COORDINATOR_EFFORT", "high");
            std::env::remove_var("SWARM_MEMBER_MAX_TURNS");
            std::env::remove_var("SWARM_MEMBER_BUDGET_USD");
            std::env::remove_var("SWARM_MEMBER_MODEL");
            std::env::remove_var("SWARM_MEMBER_EFFORT");
        }
        let frame = std::path::Path::new("/f.json");

        let coordinator = metaharness_argv(&asked(), Some(frame)).expect("an argv");
        assert_eq!(value_of(&coordinator, "--max-turns"), Some("99"));
        assert_eq!(value_of(&coordinator, "--max-budget-usd"), Some("50.00"));
        assert_eq!(
            value_of(&coordinator, "--model"),
            Some("an-expensive-model"),
            "the coordinator's own knobs still reach the coordinator"
        );
        assert_eq!(value_of(&coordinator, "--effort"), Some("high"));

        let posted = an_assignment();
        let member = metaharness_argv(&asked_as_member(&posted), Some(frame)).expect("an argv");
        assert_eq!(
            value_of(&member, "--max-turns"),
            Some("30"),
            "and none of them reaches a member: it gets the default: {member:?}"
        );
        assert_eq!(value_of(&member, "--max-budget-usd"), Some("1.00"));
        assert_eq!(
            value_of(&member, "--model"),
            None,
            "no model, rather than the coordinator's: {member:?}"
        );
        assert_eq!(value_of(&member, "--effort"), None);

        // SAFETY: as above.
        unsafe {
            std::env::set_var("SWARM_MEMBER_MAX_TURNS", "7");
            std::env::set_var("SWARM_MEMBER_BUDGET_USD", "0.25");
            std::env::set_var("SWARM_MEMBER_MODEL", "a-cheaper-model");
            std::env::set_var("SWARM_MEMBER_EFFORT", "low");
        }
        let member = metaharness_argv(&asked_as_member(&posted), Some(frame)).expect("an argv");
        assert_eq!(value_of(&member, "--max-turns"), Some("7"));
        assert_eq!(value_of(&member, "--max-budget-usd"), Some("0.25"));
        assert_eq!(value_of(&member, "--model"), Some("a-cheaper-model"));
        assert_eq!(value_of(&member, "--effort"), Some("low"));
        // And the coordinator is not moved by the member's, either way round.
        let coordinator = metaharness_argv(&asked(), Some(frame)).expect("an argv");
        assert_eq!(value_of(&coordinator, "--max-turns"), Some("99"));
        assert_eq!(
            value_of(&coordinator, "--model"),
            Some("an-expensive-model")
        );

        // SAFETY: as above.
        unsafe {
            for knob in [
                "SWARM_COORDINATOR_MAX_TURNS",
                "SWARM_COORDINATOR_BUDGET_USD",
                "SWARM_COORDINATOR_MODEL",
                "SWARM_COORDINATOR_EFFORT",
                "SWARM_MEMBER_MAX_TURNS",
                "SWARM_MEMBER_BUDGET_USD",
                "SWARM_MEMBER_MODEL",
                "SWARM_MEMBER_EFFORT",
            ] {
                std::env::remove_var(knob);
            }
        }
    }

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

#[cfg(test)]
mod resolution {
    use super::*;
    use ess_runtime::Spec;

    async fn a_swarm(data: &tempdir::TempDir, slug: &str) -> (Arc<Swarm>, String) {
        let kernel = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Arc::new(
            Swarm::open(spec, data.path(), slug)
                .await
                .expect("a swarm opens"),
        );
        issue(
            &swarm,
            "swarm.manager.CreateSwarm",
            json!({"display_name": slug, "tmux_session": slug, "home": "/tmp/x",
                   "created_at": "2026-09-12T10:00:00Z"}),
        )
        .await;
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        (swarm, swarm_id)
    }

    async fn issue(swarm: &Arc<Swarm>, command: &str, input: Json) {
        swarm
            .issue(
                None,
                command,
                input.as_object().expect("an object").clone(),
                command,
            )
            .await
            .expect("the command applies");
    }

    /// Drafts a config with this launch and activates it.
    async fn activate(swarm: &Arc<Swarm>, swarm_id: &str, launch: Json) {
        issue(
            swarm,
            "swarm.config.DraftConfig",
            json!({"swarm_id": swarm_id, "paths": {}, "launch": launch,
                   "schedules": {}, "budgets": {}, "board": {}}),
        )
        .await;
        let config_id = swarm.instances("swarm.config.Config").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        issue(
            swarm,
            "swarm.config.ActivateConfig",
            json!({"config_id": config_id, "swarm_id": swarm_id}),
        )
        .await;
    }

    /// Where the admitted set lives in the specification, and where the scope deliberately does
    /// not.
    ///
    /// `swarm.config.HarnessLaunch` already carried launch policy — `tool_surface`,
    /// `allow_program` — so the operations a frame admits belong there and not in a new entity.
    /// The subject scope does not, and that is the decision this case pins: a scope a config can
    /// state is a scope a coordinator can widen, because `swarm do swarm.config.DraftConfig` is a
    /// command like any other. It is derived from the swarm's own work directory instead
    /// ([`crate::frame::scope`]), where nothing a turn can issue reaches it.
    #[tokio::test]
    async fn the_config_declares_what_a_frame_admits_and_never_where() {
        let kernel = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Spec::load(kernel).expect("the kernel resolves");
        let types = serde_json::to_value(spec.ir().types()).expect("the IR serialises");
        let launch = &types["swarm.config.HarnessLaunch"];
        let fields: Vec<&str> = launch["body"]["fields"]
            .as_array()
            .expect("a struct has fields")
            .iter()
            .filter_map(|field| field["name"].as_str())
            .collect();

        assert!(
            fields.contains(&"admitted_operations"),
            "an operator may narrow what a turn admits: {fields:?}"
        );
        for forbidden in ["subjects", "subject_scope", "scope", "write_scope"] {
            assert!(
                !fields.contains(&forbidden),
                "and may not say where they act — a confinement the confined party can edit \
                 confines nothing: {fields:?}"
            );
        }
    }

    /// A config may narrow the frame and may never widen it, and the two halves of that.
    ///
    /// The adversary of correction round 2 drove the widening half: a config naming all ten of
    /// `frame::VOCABULARY` resolved to all ten, so a coordinator — which reaches `DraftConfig` and
    /// `ActivateConfig` through `swarm do`, as its own prompt tells it — could seal itself a frame
    /// admitting `subagent.spawn`, `task.todo` and `web.read`. This case holds the two branches
    /// that fix left behind: a narrowing config is honoured exactly, and a config that names
    /// nothing this runtime admits is refused rather than quietly handed the default.
    #[tokio::test]
    async fn a_config_narrows_the_frame_and_is_refused_when_it_would_leave_nothing() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe { std::env::remove_var("SWARM_COORDINATOR") };
        let data = tempdir::TempDir::new("swarm-narrowing").expect("a scratch directory");
        let (swarm, swarm_id) = a_swarm(&data, "narrowing").await;

        // Narrowing, with one name this runtime does not admit mixed in: the admitted ones are
        // kept, the other is dropped rather than honoured, and the turn still runs.
        activate(
            &swarm,
            &swarm_id,
            json!({"harness": "Claude", "binary": "", "model": "", "args": [],
                   "skip_permissions": false, "tool_surface": "Owned", "allow_program": [],
                   "admitted_operations": ["file.read", "dir.list", "web.read"]}),
        )
        .await;
        let resolved = resolve(&swarm).await.expect("the config resolves");
        match &resolved.launch {
            Launch::Metaharness { admitted } => assert_eq!(
                admitted,
                &["file.read".to_owned(), "dir.list".to_owned()],
                "what the config asked for and this runtime admits, in the config's own order, \
                 and `web.read` dropped rather than granted"
            ),
            other => panic!("the config names no binary, so this is the frame arm: {other:?}"),
        }

        // And a config that names ONLY operations this runtime does not admit leaves nothing. It
        // is refused, loudly, rather than falling through to the default — the same rule the rest
        // of this resolution follows, and the reason `Unfinished::Unusable` exists.
        let data = tempdir::TempDir::new("swarm-nothing-left").expect("a scratch directory");
        let (swarm, swarm_id) = a_swarm(&data, "nothing-left").await;
        activate(
            &swarm,
            &swarm_id,
            json!({"harness": "Claude", "binary": "", "model": "", "args": [],
                   "skip_permissions": false, "tool_surface": "Owned", "allow_program": [],
                   "admitted_operations": ["web.read", "task.todo"]}),
        )
        .await;
        let why = match resolve(&swarm).await {
            Err(why) => why,
            Ok(other) => panic!(
                "a config admitting nothing this runtime admits is refused, not replaced: {:?}",
                other.launch
            ),
        };
        assert!(
            why.contains("web.read") && why.contains("task.todo"),
            "the refusal names what was asked for: {why}"
        );
        assert!(
            why.contains("file.read"),
            "and what there is to ask for: {why}"
        );
    }

    /// The account of a launch names the model the launch will actually use, or no model at all.
    ///
    /// `describe` read `SWARM_COORDINATOR_MODEL` whoever was running, so a member's turn was
    /// logged — and `/status` published — with a model `metaharness_argv` never passed it.
    #[tokio::test]
    async fn the_account_of_a_launch_names_only_the_model_that_launch_will_use() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: as above.
        unsafe {
            std::env::set_var("SWARM_COORDINATOR_MODEL", "the-coordinators-model");
            std::env::set_var("SWARM_MEMBER_MODEL", "the-members-model");
        }
        let launch = Launch::metaharness();

        assert!(
            !launch.describe().contains("model"),
            "asked without a turn, it names no model, because it cannot know whose turn it is: {}",
            launch.describe()
        );
        assert!(
            launch
                .describe_for(knobs_for(None))
                .contains("the-coordinators-model"),
            "asked for a coordinator's turn, the coordinator's: {}",
            launch.describe_for(knobs_for(None))
        );
        let posted = super::tests::an_assignment();
        let for_member = launch.describe_for(knobs_for(Some(&posted)));
        assert!(
            for_member.contains("the-members-model"),
            "asked for a member's turn, the member's: {for_member}"
        );
        assert!(
            !for_member.contains("the-coordinators-model"),
            "and never the coordinator's on a member's line: {for_member}"
        );

        // SAFETY: as above.
        unsafe {
            std::env::remove_var("SWARM_COORDINATOR_MODEL");
            std::env::remove_var("SWARM_MEMBER_MODEL");
        }
    }

    /// Two slugs are never one directory, whatever characters they are spelled with.
    ///
    /// The reduction to a safe path segment was not injective: `qa/1`, `qa.1` and `qa 1` all became
    /// `qa-1` and shared a work directory with each other and with the literal `qa-1` — which is
    /// the collision the per-agent split exists to end, for names nobody has used yet.
    #[test]
    fn two_agents_never_share_a_directory_however_their_slugs_are_spelled() {
        for plain in [
            "coordinator",
            "cv2-aep",
            "qa_1",
            "qa-1",
            "disk-warden",
            "a1",
        ] {
            assert_eq!(
                agent_directory(plain),
                plain,
                "a slug that is already a safe segment is left exactly as it is"
            );
        }

        let colliding = ["qa/1", "qa.1", "qa 1", "qa-1", "qa_1", "qa\\1"];
        let directories: std::collections::BTreeSet<String> =
            colliding.iter().map(|slug| agent_directory(slug)).collect();
        assert_eq!(
            directories.len(),
            colliding.len(),
            "six slugs, six directories: {directories:?}"
        );

        for climbing in ["../../etc", "..", ".", "/absolute", ""] {
            let directory = agent_directory(climbing);
            assert!(
                !directory.contains('/') && !directory.contains(".."),
                "and nothing that could climb out of the swarm's own directory: {directory}"
            );
            assert!(!directory.is_empty(), "nor an empty segment");
        }

        assert_eq!(
            agent_directory("qa/1"),
            agent_directory("qa/1"),
            "and the mapping is stable, so an agent finds its own notes next turn"
        );
    }

    /// The operator's bug: a swarm whose config names a coordinator was told there was none.
    ///
    /// The config wins over the environment, which is set here to something else entirely so that
    /// a resolution which ignored the config would be visible rather than coincidentally right.
    #[tokio::test]
    async fn the_swarms_config_names_the_coordinator_that_runs() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe { std::env::set_var("SWARM_COORDINATOR", "/not/this/one") };
        let data = tempdir::TempDir::new("swarm-resolve-config").expect("a scratch directory");
        let (swarm, swarm_id) = a_swarm(&data, "from-config").await;
        activate(
            &swarm,
            &swarm_id,
            json!({"harness": "Claude", "binary": "/usr/local/bin/answerer",
                   "model": "", "args": ["--verdict"], "skip_permissions": false,
                   "tool_surface": "Owned", "allow_program": []}),
        )
        .await;

        let resolved = resolve(&swarm).await.expect("the config resolves");
        assert_eq!(resolved.source, Source::Config);
        assert!(
            matches!(&resolved.launch, Launch::Program(argv)
                     if argv == &["/usr/local/bin/answerer", "--verdict"]),
            "the config's own launch line, args and all: {:?}",
            resolved.launch
        );
    }

    /// A config that names Claude asks for the contained launcher, not for a program.
    #[tokio::test]
    async fn a_config_naming_claude_resolves_to_metaharness() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: as above.
        unsafe { std::env::remove_var("SWARM_COORDINATOR") };
        let data = tempdir::TempDir::new("swarm-resolve-claude").expect("a scratch directory");
        let (swarm, swarm_id) = a_swarm(&data, "claude").await;
        activate(
            &swarm,
            &swarm_id,
            json!({"harness": "Claude", "binary": "", "model": "", "args": [],
                   "skip_permissions": false, "tool_surface": "Owned", "allow_program": []}),
        )
        .await;

        let resolved = resolve(&swarm).await.expect("the config resolves");
        assert_eq!(resolved.source, Source::Config);
        assert!(matches!(resolved.launch, Launch::Metaharness { .. }));
    }

    /// With nothing configured anywhere, the answer is metaharness — and it says so.
    ///
    /// Not `Program`: `Launch::Program` runs arbitrary argv with none of `--hermetic`,
    /// `--tool-surface native`, `--decisions frame`, `--frame`, `--max-turns 30` or
    /// `--max-budget-usd 1.00`. The default is the narrowed one, which is the whole reason there is
    /// a default at all.
    #[tokio::test]
    async fn with_no_config_and_no_variable_the_default_is_metaharness_and_says_so() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: as above.
        unsafe { std::env::remove_var("SWARM_COORDINATOR") };
        let data = tempdir::TempDir::new("swarm-resolve-default").expect("a scratch directory");
        let (swarm, _) = a_swarm(&data, "default").await;

        let resolved = resolve(&swarm).await.expect("the default always resolves");
        assert_eq!(resolved.source, Source::Default);
        assert!(matches!(resolved.launch, Launch::Metaharness { .. }));
        assert!(
            resolved.describe().contains("from the default"),
            "a reader is told which of the three sources answered: {}",
            resolved.describe()
        );
        // And the same, for a status reader that has no swarm to ask.
        assert_eq!(fallback().source, Source::Default);
    }

    /// `SWARM_COORDINATOR` still answers when no config does, and says that it did.
    #[tokio::test]
    async fn the_environment_answers_when_no_config_does() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: as above.
        unsafe { std::env::set_var("SWARM_COORDINATOR", "/usr/local/bin/from-the-environment") };
        let data = tempdir::TempDir::new("swarm-resolve-env").expect("a scratch directory");
        let (swarm, _) = a_swarm(&data, "from-env").await;

        let resolved = resolve(&swarm).await.expect("the environment resolves");
        assert_eq!(resolved.source, Source::Environment);
        assert!(matches!(&resolved.launch, Launch::Program(argv)
                         if argv == &["/usr/local/bin/from-the-environment"]));
    }

    /// The distinction the old refusal conflated: your config named one, and it was ignored.
    ///
    /// A config naming Codex must not be quietly given metaharness-with-Claude. Falling through to
    /// the default here would run a different model than the operator asked for and report success,
    /// which is worse than refusing.
    #[tokio::test]
    async fn a_config_this_host_cannot_launch_is_refused_rather_than_replaced() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // SAFETY: as above.
        unsafe { std::env::remove_var("SWARM_COORDINATOR") };
        let data = tempdir::TempDir::new("swarm-resolve-codex").expect("a scratch directory");
        let (swarm, swarm_id) = a_swarm(&data, "codex").await;
        activate(
            &swarm,
            &swarm_id,
            json!({"harness": "Codex", "binary": "", "model": "", "args": [],
                   "skip_permissions": false, "tool_surface": "Owned", "allow_program": []}),
        )
        .await;

        let why = match resolve(&swarm).await {
            Err(why) => why,
            Ok(other) => panic!(
                "a config this host cannot launch is refused, not replaced: {:?} from {}",
                other.launch, other.source
            ),
        };
        assert!(why.contains("Codex"), "and the refusal names it: {why}");
        assert!(
            Unfinished::Unusable { why }
                .to_string()
                .contains("the default was NOT used"),
            "the two cases the old refusal conflated are told apart in words"
        );
    }
}

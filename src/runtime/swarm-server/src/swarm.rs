//! One running swarm: its log, its world, and the broadcast that tells the UI something changed.
//!
//! The world is held in memory and written through to the log, rather than re-folded on every read.
//! Both are true at once because the fold is deterministic — the in-memory copy is a cache of the
//! replay, and [`Swarm::reload`] throws it away and rebuilds when that needs proving.
//!
//! Three methods apply a command and write the log, and there are three rather than one because a
//! command arrives here three ways: [`Swarm::issue`] for one a caller asked for, [`Swarm::tick`]
//! for one a period caused, and [`Swarm::redeliver`] for one whose first delivery failed. All
//! three run the pump over what the command emitted, so a cascade does not depend on which door
//! the command came in by — that difference is what a fourth path into the world would really be,
//! and it is the thing to keep out rather than the count of methods.
//!
//! Until 2026-09-12 this said `issue` was the only one of the three. The sentence survived being
//! false of `tick` from the day it was written, because nothing reads a module doc for a
//! contradiction — so when the claim above was first corrected it was corrected into a second
//! false sentence, `tick` having no pump at all. It is true now because `tick` was changed, not
//! because the sentence was.
//!
//! Two call sites serve the three doors, and the difference between them is worth knowing before
//! reading further: `issue` routes and commits inline, because the cascade of a command a caller
//! asked for is part of that request and a refusal in it is the caller's answer. `tick` and
//! `redeliver` both go through [`Swarm::pump_cascade`], because nobody is waiting on either — a
//! failure there is logged and owed, and there is no request to fail.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Map, Value as Json};
use tokio::sync::{Mutex, broadcast};

use crate::budget::Reached;
use crate::coordinator::Spent;
use ess_runtime::apply::Emitted;
use ess_runtime::{
    Issuer, Recorded, Spec, Store, World, apply, pump, route::Occurrence, route::Routed,
    route::needs_redelivery, route::tick, view,
};

/// How many events a slow reader may fall behind before it is dropped and told to resync.
const BACKLOG: usize = 256;

/// How many times a failed `at_least_once` delivery is attempted in total, the first included.
///
/// Neither number is the specification's. ESS declares `delivery` and `on_failure` and says nothing
/// about how many attempts or how far apart, so the bound is the host's and is written here rather
/// than inferred: an unbounded retry of a delivery that fails for a permanent reason — a command
/// whose subject was never created — is a queue that grows for as long as the process runs.
///
/// Three, because the failure this serves is a stale read losing a race with a write, and a
/// delivery still failing on its third attempt is failing for a reason another attempt will not
/// change. [`RETRY_DELAY`] is `PT30S` — the same period `components.yaml` gives `turn-the-loop`,
/// because the trigger's loop is what drains this queue and a second cadence would be a second
/// thing to keep in step with the first.
///
/// It is a FLOOR and not a period. Nothing is re-attempted before it, and the actual wait is this
/// plus however far the delivery was queued from the next drain — between one and two trigger
/// periods, 30 to 60 seconds as the kernel stands. `trigger.rs` says what widens it further.
pub const MAX_ATTEMPTS: u32 = 3;

/// How long a failed delivery waits before it is attempted again. See [`MAX_ATTEMPTS`].
pub const RETRY_DELAY: Duration = Duration::from_secs(30);

/// Something that happened, as the UI hears about it.
///
/// Stamped with when the server saw it, so a reader can show "3s ago" without a clock of its own,
/// and so two readers that connected at different times agree on the order.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    /// When, RFC 3339.
    pub at: String,
    #[serde(flatten)]
    pub what: What,
}

/// What kind of thing happened.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum What {
    /// A command was applied and these events were appended.
    Applied {
        command: String,
        outcome: String,
        actor: Option<String>,
        events: Vec<Record>,
        /// The instance as it now stands, when the outcome acted on one.
        instance: Option<Json>,
    },
    /// A binding carried an event to another command.
    Routed {
        binding: String,
        command: String,
        outcome: String,
        instance: Option<Json>,
    },
    /// The loop turned for one goal.
    Ticked {
        binding: String,
        command: String,
        goal: String,
        iterations: u64,
        instance: Option<Json>,
    },
    /// An agent was asked, or answered, or could not.
    Turn {
        /// Whose turn it is.
        ///
        /// Added in correction round 1 of the 2026-09-13a wave, and it is the same defect as
        /// `turn_file`'s: while a swarm had one agent the unit identified the turn, and two members
        /// working at once publish into one channel where no reader can tell whose events are
        /// whose. A field and not a new variant, because it is the same event about a different
        /// agent.
        agent: String,
        /// The unit of work — a goal id, or an assignment id.
        goal: String,
        iterations: u64,
        phase: TurnPhase,
        reached: Option<bool>,
        note: Option<String>,
        error: Option<String>,
        took_ms: Option<u64>,
        /// What the turn cost, once anything is known.
        spent: Option<Spent>,
    },
    /// One event of the coordinator's run, as it happened.
    ///
    /// `event` is a `metaharness.event/1` record passed through whole: a text, a tool call, a
    /// usage figure. `spent` is the running total up to and including it.
    Agent {
        /// Whose session this event is from. See [`What::Turn::agent`].
        agent: String,
        /// The unit of work — a goal id, or an assignment id.
        goal: String,
        iterations: u64,
        seq: u64,
        spent: Spent,
        event: Json,
    },
    /// A unit of work used up a cap, so the loop stopped asking. Nothing in the model changed.
    Capped {
        /// Whose turn was refused. See [`What::Turn::agent`] — this is the fourth variant of the
        /// same fix, and it was missing: it published an assignment UUID in a field named `goal`
        /// with nothing saying whose turn it was, live, to every reader of the stream.
        agent: String,
        /// Which kind of unit [`Self::Capped::goal`] identifies: `"goal"` or `"assignment"`.
        unit: String,
        /// The unit of work — a goal id, or an assignment id. Still called `goal` because
        /// `src/web/src/runtime.ts` reads it under that name.
        goal: String,
        turns: u64,
        spent_usd: Option<f64>,
        reached: Reached,
        why: String,
    },
    /// A binding that asked to be retried was given up on after [`MAX_ATTEMPTS`].
    ///
    /// The one thing a watcher must be told: the delivery the specification promised did not
    /// happen. Silence here is the defect this variant was added to end.
    Undelivered {
        binding: String,
        command: String,
        attempts: u32,
        why: String,
    },
    /// The in-memory world was thrown away and rebuilt from the log.
    Reloaded,
}

/// Where a coordinator's turn is.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    Asking,
    Answered,
    Unfinished,
}

/// Now, as the wire carries it.
pub fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// One fired bound, as `turns/capped.jsonl` records it.
///
/// A struct and not seven arguments, because six of them are strings and numbers that would read
/// identically in any order at the call site.
#[derive(Debug)]
pub struct CappedRecord<'a> {
    /// Whose turn was refused. Always known: something was about to be asked when this fired.
    pub agent: &'a str,
    /// Which fold the caps were applied to: `"unit"` for the unit's own record, `"agent"` for the
    /// agent's whole record across every unit it works, `"swarm"` for the whole swarm's record —
    /// every agent in it, every unit they work. The third value arrived with
    /// `SWARM_MAX_TOTAL_SPEND_USD`, which refuses a swarm whose members are each still inside
    /// their own bounds.
    pub bound: &'a str,
    /// The unit of work the turn was refused at — the goal, or the assignment.
    pub unit: &'a str,
    /// Turns on the record that fired.
    pub turns: u64,
    /// Dollars on it, where anything on it was priced.
    pub spent_usd: Option<f64>,
    /// The environment variable that sets the cap reached, so a reader is not left to find it.
    pub cap: &'a str,
    /// The sentence a person is given, verbatim from `Capped::why`.
    pub why: &'a str,
}

/// What a caller wants said, before it is addressed.
#[derive(Debug)]
pub struct Outgoing<'a> {
    /// `agent` or `agent/mailbox`. Ignored by a broadcast.
    pub to: &'a str,
    /// Who the message SAYS it is from. A claim in the payload, like `TakeAssignment`'s
    /// `agent_id`, and free to disagree with `issuer` — which is the point of having both.
    pub sender: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
    /// The message this answers, when it answers one.
    pub reply_to: Option<&'a str>,
    /// Who issued it. Mail is the second door that writes events, and an operator can reach it
    /// exactly as an agent can, so it names its issuer for the same reason the command door does.
    pub issuer: Issuer,
}

/// What posting produced.
#[derive(Debug, Serialize)]
pub struct Posted {
    /// Present on a broadcast; the id every copy shares.
    pub broadcast_id: Option<String>,
    pub messages: Vec<String>,
}

/// Where one swarm is, in one row.
#[derive(Debug, Serialize)]
pub struct Summary {
    pub slug: String,
    pub display_name: Option<String>,
    /// The `swarm.manager.Swarm` state, or `None` before `CreateSwarm`.
    pub state: Option<String>,
    pub goal: Option<GoalSummary>,
    /// Instances held, by entity.
    pub instances: BTreeMap<String, usize>,
    /// Readers on the stream right now.
    pub watchers: usize,
    /// What every finished coordinator turn has cost, summed.
    pub spent: Spent,
    /// How many turns that sum covers.
    pub turns_recorded: u64,
}

/// One recorded coordinator run.
#[derive(Debug, Serialize)]
pub struct TurnRecord {
    pub name: String,
    pub iterations: u64,
    /// Which attempt at this turn number. A turn that failed is retried with the same number.
    pub attempt: u64,
    /// The first eight characters of the goal's identity.
    pub goal: String,
    pub bytes: u64,
    /// The verdict, when the turn finished with one.
    pub reached: Option<bool>,
    pub note: Option<String>,
    pub ended_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GoalSummary {
    pub id: String,
    pub state: String,
    pub text: String,
    pub iterations: u64,
}

/// One appended event, flattened for a reader.
#[derive(Clone, Debug, Serialize)]
pub struct Record {
    pub name: String,
    pub fields: Map<String, Json>,
}

impl From<&Emitted> for Record {
    fn from(emitted: &Emitted) -> Self {
        Self {
            name: emitted.name.clone(),
            fields: emitted.fields.clone(),
        }
    }
}

/// What issuing a command produced.
#[derive(Debug, Serialize)]
pub struct Issued {
    /// Which outcome the specification selected.
    pub outcome: String,
    /// The error it reports, when it refused.
    pub error: Option<String>,
    /// The events appended, including any a binding caused.
    pub events: Vec<Record>,
}

/// Why a request could not be served.
#[derive(Debug)]
pub enum Refused {
    /// The specification refused it: no such command, not permitted, a missing input.
    Command(String),
    /// A binding could not be carried out.
    Routing(String),
    /// The log refused it.
    Store(String),
    /// No such view.
    View(String),
    /// The domain may have committed, but process-host cleanup failed.
    Host(String),
}

impl std::error::Error for Refused {}

impl From<ess_runtime::LoadError> for Refused {
    fn from(why: ess_runtime::LoadError) -> Self {
        Self::Command(why.to_string())
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(why)
            | Self::Routing(why)
            | Self::Store(why)
            | Self::View(why)
            | Self::Host(why) => f.write_str(why),
        }
    }
}

/// A delivery a binding asked for, that failed, and that is owed another attempt.
///
/// **Where this lives is a decision the story left open, and it is here rather than in the
/// eventlog.** `store.rs` holds what the specification declares: an event is appended because an
/// outcome emitted it, and `bin/check-sets-are-emitted.py` refuses a spec whose outcome writes a
/// field no event carries. There is no ESS event for "a delivery was attempted and failed" — the
/// nearest thing, `swarm.agent.AssignmentUndelivered`, is an escalation the model declares for one
/// binding — so putting attempts in the log would mean the runtime inventing a record the
/// specification does not declare. That is the rule this repository exists to keep: a fact the
/// model does not name is not written to the model'"'"'s log. An attempt is a fact about this
/// process'"'"'s effort to deliver, not about the swarm, and it is held per-server accordingly.
///
/// **A pending delivery is in memory, so it does not survive the process, and "at least once"
/// therefore holds only for as long as the server does.** This is a real limit, not a rounding:
/// a crash between the failed attempt and the last retry loses the delivery with no trace but a
/// log line. Closing it needs the queue to be durable BEFORE the first attempt is made — a table
/// beside the eventlog in `data/swarms/<slug>/`, written under the same request key used for the
/// commit, drained at `Swarm::open`. That is a store schema and a boot path, which is a larger
/// change than this one and belongs to whoever decides the log gets a second table.
#[derive(Clone, Debug, Serialize)]
pub struct Delivery {
    /// The binding whose delivery failed.
    pub binding: String,
    /// The command it was carrying the event to.
    pub command: String,
    /// How many attempts this delivery has had, the first one included.
    pub attempts: u32,
    /// Why the last attempt failed.
    pub why: String,
    /// The input the binding'"'"'s mapping built. Not on the wire; a reader wants the count and the
    /// reason, and the payload is the pump'"'"'s business.
    #[serde(skip)]
    input: Map<String, Json>,
    /// The key the successful attempt commits under. The same one on every attempt.
    ///
    /// **It is not what stops a delivery landing twice, and an earlier version of this comment said
    /// it was.** `store.rs` scopes an idempotency key to the INSTANCE'S STREAM, so it can only
    /// refuse a second append to a stream it was already spent on. That covers a command that
    /// `updates` an existing instance — `AdoptConfig` and `Note`, two of the three bindings this
    /// queue serves — and does nothing at all for one that `creates`: `RecordAssignment` mints a
    /// fresh `Assignment` id per attempt, so every attempt writes to a stream where the key has
    /// never been spent and nothing refuses anything. Measured, not reasoned:
    /// `tests/redelivery_under_attack.rs` spends one key twice on `DraftConfig` and gets two
    /// Configs. Filed as `story:request-key-is-not-idempotency`.
    ///
    /// What actually stops a double delivery here is this queue: a delivery is held once, attempted
    /// once per drain, and removed on the first success. That is a property of THIS type, held in
    /// THIS process — so it is worth exactly as much as the process is, which is the same limit the
    /// doc on this struct states. Cross-stream idempotency is a `Claim` in the log, which `store.rs`
    /// says is not used here; making the key mean what the old comment claimed is that change.
    #[serde(skip)]
    request: String,
    /// When the next attempt may be made.
    #[serde(skip)]
    due: Instant,
}

/// One swarm, running.
pub struct Swarm {
    pub(crate) control: Arc<crate::control::Control>,
    spec: Arc<Spec>,
    store: Store,
    world: Mutex<World>,
    changes: broadcast::Sender<Change>,
    slug: String,
    /// `data/swarms/<slug>`: the log, the work directory, the turns.
    dir: PathBuf,
    /// Failed deliveries of bindings that declared `at_least_once` with `retry`, awaiting another
    /// attempt. See [`Delivery`] for why this is here and not in the log.
    owed: Mutex<Vec<Delivery>>,
    /// The bindings this swarm keeps failures for, read off the specification once at open.
    ///
    /// `Routed` carries the binding'"'"'s name and not its policy, so the host cross-references the
    /// pump'"'"'s own answer rather than re-deciding what `at_least_once` means.
    redelivers: Vec<String>,
}

impl Swarm {
    /// Opens a swarm's log and rebuilds its world from it.
    ///
    /// A swarm that has never run comes back empty, which is not a failure: a bare swarm has done
    /// nothing yet, and that is the state it is supposed to start in.
    pub async fn open(spec: Arc<Spec>, root: &Path, slug: &str) -> Result<Self, Refused> {
        let store = Store::open(root, slug)
            .await
            .map_err(|why| Refused::Store(why.to_string()))?;
        let world = store
            .world(spec.ir())
            .await
            .map_err(|why| Refused::Store(why.to_string()))?;
        let (changes, _) = broadcast::channel(BACKLOG);

        let redelivers = needs_redelivery(spec.ir());

        Ok(Self {
            spec,
            store,
            control: crate::control::Control::new(
                world
                    .values()
                    .any(|i| i.entity == "swarm.manager.Swarm" && i.state == "Running"),
            ),
            world: Mutex::new(world),
            changes,
            slug: slug.to_owned(),
            dir: root.join("swarms").join(slug),
            owed: Mutex::new(Vec::new()),
            redelivers,
        })
    }

    /// Which swarm this is.
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Where this swarm's files live.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Appends one attempt's figures to `turns/spend.jsonl`, naming the agent that spent them.
    ///
    /// A small file beside the transcripts, so a total can be read without reading every run.
    /// Appends one ATTEMPT's figures, whether or not it produced a verdict.
    ///
    /// A turn that was cut off before it answered still ran and still cost money, and until
    /// 2026-09-12 only answered turns were recorded — so a coordinator that never wrote a verdict
    /// spent without limit while both caps read zero.
    ///
    /// `agent` is required, and there is deliberately no second door that omits it. There was one
    /// for a few hours — `record_spend_by` beside an unattributed `record_spend` — and the
    /// answered-turn path stayed on the unattributed one, so every SUCCESSFUL turn wrote
    /// `"agent": null` while a test asserting the opposite passed over rows it had written itself.
    /// A hand-kept rule that every caller should attribute is the defect; a signature that cannot
    /// express the alternative is the fix.
    ///
    /// `goal` alone cannot bound anything once more than one agent works one goal: every row lands
    /// in the same bucket, so the figures say what the goal cost and can never say what an agent
    /// cost. A ceiling can only be written against a figure that names who ran it up.
    pub fn record_spend(
        &self,
        agent: &str,
        goal_id: &str,
        iterations: u64,
        spent: &Spent,
        reached: Option<bool>,
        note: Option<&str>,
    ) {
        let dir = self.dir.join("turns");
        let _ = std::fs::create_dir_all(&dir);
        let line = serde_json::json!({
            "at": now(),
            "agent": agent,
            "goal": goal_id,
            "iterations": iterations,
            "reached": reached,
            "note": note,
            "finished": reached.is_some(),
            "spent": spent,
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("spend.jsonl"))
        {
            use std::io::Write;
            // One write per row, not many. `writeln!` issues several small writes, so two members
            // recording a turn at the same moment interleave *inside* a row; `spend_where` then
            // drops the shredded line with `let Ok(row) = ... else { continue }` and the turn and
            // its dollars vanish from every cap fold. A single `write_all` under `O_APPEND` is
            // atomic. Unreachable while one writer could not race itself; SWARM_MAX_IN_FLIGHT
            // makes it reachable.
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
    }

    /// Appends one fired bound to `turns/capped.jsonl`, beside the spend it was measured on.
    ///
    /// A cap that refuses a turn was published to the watch stream and to the tracing log, and both
    /// of those are gone with the process. So a finished swarm's own records could not answer
    /// "was a turn ever refused here" — the first question a reader of a stopped loop asks, and the
    /// question nobody could put to the $11.35 run afterwards.
    ///
    /// Written from `report_capped`, which is idempotent per unit, so a bound that holds for a
    /// hundred periods leaves one row rather than a hundred.
    pub fn record_capped(&self, capped: &CappedRecord<'_>) {
        let dir = self.dir.join("turns");
        let _ = std::fs::create_dir_all(&dir);
        let line = serde_json::json!({
            "at": now(),
            "agent": capped.agent,
            "unit": capped.unit,
            "bound": capped.bound,
            "turns": capped.turns,
            "spent_usd": capped.spent_usd,
            "cap": capped.cap,
            "why": capped.why,
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("capped.jsonl"))
        {
            use std::io::Write;
            // One write per row, not many. `writeln!` issues several small writes, so two members
            // recording a turn at the same moment interleave *inside* a row; `spend_where` then
            // drops the shredded line with `let Ok(row) = ... else { continue }` and the turn and
            // its dollars vanish from every cap fold. A single `write_all` under `O_APPEND` is
            // atomic. Unreachable while one writer could not race itself; SWARM_MAX_IN_FLIGHT
            // makes it reachable.
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
    }

    /// What every finished turn has cost, summed, and how many there were.
    pub fn spend(&self) -> (Spent, u64) {
        self.spend_where(|_| true)
    }

    /// The same, for one goal, counting ATTEMPTS rather than answered turns.
    ///
    /// This is what a cap is measured against, and it must not be the goal's own `iterations`: a
    /// turn that fails leaves the goal where it was, so the next attempt carries the same number
    /// and a goal that never answers would sit at turn 1 for ever while the money went out.
    pub fn spend_on(&self, goal_id: &str) -> (Spent, u64) {
        self.spend_where(|row| row.get("goal").and_then(Json::as_str) == Some(goal_id))
    }

    /// The same, for one agent on one goal.
    ///
    /// An unattributed row belongs to no agent: it is in the goal's total and in nobody's share,
    /// which is the honest reading of a record that never said who spent it.
    pub fn spend_by(&self, goal_id: &str, agent: &str) -> (Spent, u64) {
        self.spend_where(|row| {
            row.get("goal").and_then(Json::as_str) == Some(goal_id)
                && row.get("agent").and_then(Json::as_str) == Some(agent)
        })
    }

    /// Everything one agent has run, across every goal it worked.
    ///
    /// The fold that keys on nothing but the agent. A cap keyed on a goal is bounded by the number
    /// of goals a swarm has rather than by anything an operator chose: one agent spending $1.00 a
    /// turn over two goals reaches $6.00 against a $5.00 cap and is refused nowhere, because each
    /// goal holds only $3.00. Measured, and pinned in
    /// `tests/the_turn_cap_under_attack.rs`.
    ///
    /// An unattributed row belongs to no agent here as in [`Swarm::spend_by`], so a ceiling read
    /// from this cannot be evaded by a row that never said who spent it — such a row is in no
    /// agent's total, and the rows that name an agent are still all of that agent's.
    pub fn spend_by_agent(&self, agent: &str) -> (Spent, u64) {
        self.spend_where(|row| row.get("agent").and_then(Json::as_str) == Some(agent))
    }

    /// Folds the spend record, keeping the rows a caller wants.
    fn spend_where(&self, keep: impl Fn(&Json) -> bool) -> (Spent, u64) {
        let mut total = Spent::default();
        let mut turns = 0;
        if let Ok(text) = std::fs::read_to_string(self.dir.join("turns").join("spend.jsonl")) {
            for line in text.lines() {
                let Ok(row) = serde_json::from_str::<Json>(line) else {
                    continue;
                };
                if !keep(&row) {
                    continue;
                }
                if let Some(spent) = row
                    .get("spent")
                    .and_then(|spent| serde_json::from_value::<Spent>(spent.clone()).ok())
                {
                    total.add(&spent);
                    turns += 1;
                }
            }
        }
        (total, turns)
    }

    /// Every recorded turn, newest first: the file name, the turn number, the goal, the verdict.
    pub fn turns(&self) -> Vec<TurnRecord> {
        // Verdicts live in spend.jsonl, not in the transcript: the model's words are the run's
        // record and the verdict is what the runtime made of them.
        let mut verdicts: BTreeMap<(u64, String), (bool, Option<String>, String)> = BTreeMap::new();
        if let Ok(text) = std::fs::read_to_string(self.dir.join("turns").join("spend.jsonl")) {
            for line in text.lines() {
                let Ok(row) = serde_json::from_str::<Json>(line) else {
                    continue;
                };
                let (Some(turn), Some(goal)) = (
                    row.get("iterations").and_then(Json::as_u64),
                    row.get("goal").and_then(Json::as_str),
                ) else {
                    continue;
                };
                verdicts.insert(
                    (turn, goal.chars().take(8).collect()),
                    (
                        row.get("reached").and_then(Json::as_bool).unwrap_or(false),
                        row.get("note")
                            .and_then(Json::as_str)
                            .map(ToOwned::to_owned),
                        row.get("at")
                            .and_then(Json::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    ),
                );
            }
        }
        let mut found = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.dir.join("turns")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let Some(stem) = name.strip_suffix(".jsonl") else {
                    continue;
                };
                let mut parts = stem.splitn(3, '-');
                let (Some(turn), Some(attempt), Some(goal)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                let goal = goal.to_owned();
                let (Ok(iterations), Ok(attempt)) = (turn.parse::<u64>(), attempt.parse::<u64>())
                else {
                    continue;
                };
                let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or_default();
                let verdict = verdicts.get(&(iterations, goal.clone()));
                found.push(TurnRecord {
                    name,
                    iterations,
                    attempt,
                    reached: verdict.map(|(reached, _, _)| *reached),
                    note: verdict.and_then(|(_, note, _)| note.clone()),
                    ended_at: verdict.map(|(_, _, at)| at.clone()),
                    goal,
                    bytes,
                });
            }
        }
        found.sort_by(|a, b| {
            b.iterations
                .cmp(&a.iterations)
                .then(b.attempt.cmp(&a.attempt))
        });
        found
    }

    /// One recorded turn, every event.
    pub fn turn(&self, name: &str) -> Result<Vec<Json>, Refused> {
        // A name is a file in ONE directory; anything that could reach another is refused.
        if name.contains('/') || name.contains("..") || !name.ends_with(".jsonl") {
            return Err(Refused::View(format!("no turn `{name}`")));
        }
        let text = std::fs::read_to_string(self.dir.join("turns").join(name))
            .map_err(|_| Refused::View(format!("no turn `{name}`")))?;
        Ok(text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }

    /// A reader of everything that happens from now on.
    pub fn watch(&self) -> broadcast::Receiver<Change> {
        self.changes.subscribe()
    }

    /// Tells every watcher something happened. Nobody listening is not an error.
    pub fn announce(&self, what: What) {
        let _ = self.changes.send(Change { at: now(), what });
    }

    /// The most recent events in the log, oldest first.
    pub async fn history(&self, limit: usize) -> Result<Vec<Recorded>, Refused> {
        self.store
            .history(limit)
            .await
            .map_err(|why| Refused::Store(why.to_string()))
    }

    /// How many events the log holds.
    pub async fn count(&self) -> Result<u64, Refused> {
        self.store
            .count()
            .await
            .map_err(|why| Refused::Store(why.to_string()))
    }

    /// How many watchers are connected.
    pub fn watchers(&self) -> usize {
        self.changes.receiver_count()
    }

    /// A one-line account of where the swarm is: its record, its goal, and what it holds.
    pub async fn summary(&self) -> Summary {
        let world = self.world.lock().await;
        let mut instances = BTreeMap::new();
        let mut state = None;
        let mut display_name = None;
        let mut goal = None;
        for instance in world.values() {
            *instances.entry(instance.entity.clone()).or_insert(0usize) += 1;
            match instance.entity.as_str() {
                "swarm.manager.Swarm" => {
                    state = Some(instance.state.clone());
                    display_name = instance
                        .fields
                        .get("display_name")
                        .and_then(|field| field.known())
                        .and_then(Json::as_str)
                        .map(ToOwned::to_owned);
                }
                "swarm.goal.Goal" => {
                    goal = Some(GoalSummary {
                        id: instance.id.clone(),
                        state: instance.state.clone(),
                        text: instance
                            .fields
                            .get("text")
                            .and_then(|field| field.known())
                            .and_then(Json::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        iterations: instance
                            .fields
                            .get("iterations")
                            .and_then(|field| field.known())
                            .and_then(Json::as_u64)
                            .unwrap_or_default(),
                    });
                }
                _ => {}
            }
        }
        let (spent, turns_recorded) = self.spend();
        Summary {
            slug: self.slug.clone(),
            display_name,
            state,
            goal,
            instances,
            watchers: self.watchers(),
            spent,
            turns_recorded,
        }
    }

    /// Applies one command the runtime issued to itself.
    ///
    /// **Every in-process caller of this is the runtime**, and that is what the record now says:
    /// the trigger, the turn machinery, `ensure_agent`, `reload`. A command from outside arrives
    /// through a door, and a door calls [`Swarm::issue_as`] with what the caller said about
    /// itself — the command door, and the mail door through [`Swarm::deliver`]. There is no third
    /// way in.
    pub async fn issue(
        &self,
        actor: Option<&str>,
        command: &str,
        input: Map<String, Json>,
        request: &str,
    ) -> Result<Issued, Refused> {
        self.issue_as(&Issuer::Runtime, actor, command, input, request)
            .await
    }

    /// Applies one command, appends what it produced, and runs the bindings it set off.
    ///
    /// The lock is held across the whole of it on purpose. Applying reads the world, deciding the
    /// outcome depends on what it read, and the append records that decision — a reader that got in
    /// between would see a world no command had produced.
    ///
    /// `issuer` is who is issuing it; `actor` is which of the specification's actor types they are
    /// claiming to act as. Both go in the record and neither replaces the other: the actor type is
    /// what `permitted` checks, and it cannot tell two members of a swarm apart because it was
    /// never asked to.
    pub async fn issue_as(
        &self,
        issuer: &Issuer,
        actor: Option<&str>,
        command: &str,
        input: Map<String, Json>,
        request: &str,
    ) -> Result<Issued, Refused> {
        #[cfg(test)]
        if crate::control::Control::in_turn() && command == "swarm.goal.Evaluate" {
            self.control.before_result().await;
        }
        let lifecycle = command.starts_with("swarm.manager.");
        let _lifecycle = if lifecycle {
            Some(self.control.lifecycle.lock().await)
        } else {
            None
        };
        let mut publication = if lifecycle {
            Some(self.control.publication.lock().await)
        } else {
            None
        };
        let mut world = self.world.lock().await;
        self.control.check().map_err(Refused::Command)?;

        let mut done = apply(
            self.spec.ir(),
            &world,
            actor,
            command,
            &input,
            Some(&mint()),
        )
        .map_err(|why| Refused::Command(why.to_string()))?;

        if lifecycle
            && matches!(done.outcome.as_str(), "started" | "resumed")
            && !self.control.running()
        {
            // Only a valid opening transition retries cleanup. Wrong-state stays the domain's
            // answer even while an earlier cancellation is waiting for a cleanup retry.
            drop(world);
            // A cancelled result may be queued for publication. Let it acquire the lock and
            // reject its old generation, so its claim can retire while cleanup waits for it.
            drop(publication.take());
            self.control.quiesce().await.map_err(Refused::Host)?;
            publication = Some(self.control.publication.lock().await);
            world = self.world.lock().await;
            done = apply(
                self.spec.ir(),
                &world,
                actor,
                command,
                &input,
                Some(&mint()),
            )
            .map_err(|why| Refused::Command(why.to_string()))?;
        }
        let opening = matches!(done.outcome.as_str(), "started" | "resumed") && lifecycle;
        self.store
            .commit(&done, actor, issuer, request)
            .await
            .map_err(|why| Refused::Store(why.to_string()))?;

        if let Some(instance) = done.instance.clone() {
            world.insert((instance.entity.clone(), instance.id.clone()), instance);
        }

        let closing =
            lifecycle && matches!(done.outcome.as_str(), "paused" | "stopped" | "deleted");
        if closing {
            self.control.close();
        }
        if opening {
            self.control.open();
        }
        let mut appended: Vec<Record> = done.events.iter().map(Record::from).collect();
        self.announce(What::Applied {
            command: command.to_owned(),
            outcome: done.outcome.clone(),
            actor: actor.map(ToOwned::to_owned),
            events: appended.clone(),
            instance: done
                .instance
                .as_ref()
                .and_then(|instance| serde_json::to_value(instance).ok()),
        });

        // Whatever the bindings carry onward is part of the same request.
        let caused = pump(
            self.spec.ir(),
            &mut world,
            done.events.clone(),
            &mut mint_each(),
        )
        .map_err(|why| Refused::Routing(why.to_string()))?;

        appended.extend(self.commit_caused(&caused, request).await?);

        drop(world);
        drop(publication);
        if closing {
            self.control.quiesce().await.map_err(Refused::Host)?;
        }
        Ok(Issued {
            outcome: done.outcome,
            error: done.error,
            events: appended,
        })
    }

    /// The key one binding's delivery commits under, whichever attempt lands.
    ///
    /// It distinguishes the deliveries of one request from each other and from the request's own
    /// command. What it does not do is make a second landing impossible — see [`Delivery::request`]
    /// and `story:request-key-is-not-idempotency`.
    fn delivery_key(&self, request: &str, binding: &str) -> String {
        format!("{request}:{binding}")
    }

    /// Commits everything the bindings caused, and keeps the failures they asked to have kept.
    ///
    /// One method for every path that runs the pump, so that what happens to a caused command does
    /// not depend on which of them ran it. Before this existed the `Ok` arm was written out at each
    /// site and the `Err` arm at only one, which is how a failed delivery came to be dropped.
    ///
    /// **Every binding in `caused` is dealt with, whichever way each one went.** The first version
    /// of this used `?` on the commit, which returned from the middle of the loop: in the one
    /// fan-out this kernel has — `AssignmentPosted` to `record-the-assignment` AND
    /// `note-the-assignment`, both `at_least_once` — a store failure on the first left the second
    /// unexamined and its delivery neither owed nor announced. A delivery is not lost because
    /// another delivery failed. The refusal is still returned, because the caller asked for a
    /// command and part of what it asked for did not land; it is returned after the loop, not
    /// instead of it, and the first one is the one reported.
    ///
    /// A commit failure is owed like an apply failure. The command applied against the world in
    /// memory and only the log refused, so the retry re-applies and re-commits under the same key —
    /// which is the one case where that key does what a key is for, because the stream it names is
    /// the one the failed append was for.
    async fn commit_caused(
        &self,
        caused: &[Routed],
        request: &str,
    ) -> Result<Vec<Record>, Refused> {
        let mut appended = Vec::new();
        let mut refused: Option<Refused> = None;

        for routed in caused {
            match &routed.result {
                Ok(applied) => {
                    match self
                        .store
                        .commit(
                            applied,
                            None,
                            &Issuer::Runtime,
                            &self.delivery_key(request, &routed.binding),
                        )
                        .await
                    {
                        Ok(()) => {
                            appended.extend(applied.events.iter().map(Record::from));
                            self.announce(What::Routed {
                                binding: routed.binding.clone(),
                                command: routed.command.clone(),
                                outcome: applied.outcome.clone(),
                                instance: applied
                                    .instance
                                    .as_ref()
                                    .and_then(|instance| serde_json::to_value(instance).ok()),
                            });
                        }
                        Err(why) => {
                            self.owe(routed, &why.to_string(), request).await;
                            refused.get_or_insert(Refused::Store(why.to_string()));
                        }
                    }
                }
                // The failure the pump hands out. Until 2026-09-12 nothing read this arm, so every
                // failed delivery of the three bindings that declare `at_least_once`/`retry` was
                // discarded — the promise the specification makes and nothing kept. A binding that
                // declares any other policy is the pump's business and not this one's: `Escalate`
                // has already published its event and `Drop` means what it says.
                Err(why) => self.owe(routed, &why.to_string(), request).await,
            }
        }

        match refused {
            Some(why) => Err(why),
            None => Ok(appended),
        }
    }

    /// Stops trying, and says so.
    ///
    /// Every arm that abandons a delivery goes through here. There are two — the attempt that could
    /// not be applied and the one that could not be committed — and until 2026-09-12 only the first
    /// announced anything, so a delivery lost to a store failure was lost in the silence this
    /// variant exists to end. A third arm that forgets is the failure mode; there is one place to
    /// forget in now.
    fn give_up(&self, delivery: &Delivery) {
        tracing::error!(swarm = %self.slug, binding = %delivery.binding,
                        command = %delivery.command, attempts = delivery.attempts,
                        error = %delivery.why, "a delivery was given up on");
        self.announce(What::Undelivered {
            binding: delivery.binding.clone(),
            command: delivery.command.clone(),
            attempts: delivery.attempts,
            why: delivery.why.clone(),
        });
    }

    /// Keeps a failed delivery that its binding asked to have retried.
    ///
    /// A binding that did not ask is not kept. The list comes from `route::needs_redelivery`, so
    /// "which bindings asked" is answered by the specification through the pump rather than by a
    /// second reading of the model here.
    async fn owe(&self, routed: &Routed, why: &str, request: &str) {
        if !self.redelivers.iter().any(|name| name == &routed.binding) {
            tracing::debug!(swarm = %self.slug, binding = %routed.binding, error = %why,
                            "a delivery failed and its binding asked for no retry");
            return;
        }

        tracing::warn!(swarm = %self.slug, binding = %routed.binding, command = %routed.command,
                       error = %why, "a delivery failed and is owed another attempt");
        self.owed.lock().await.push(Delivery {
            binding: routed.binding.clone(),
            command: routed.command.clone(),
            // The attempt that just failed. A caller reading `attempts` wants how many times the
            // delivery has been tried, not how many times this queue has tried it.
            attempts: 1,
            why: why.to_owned(),
            input: routed.input.clone(),
            request: self.delivery_key(request, &routed.binding),
            due: Instant::now() + RETRY_DELAY,
        });
    }

    /// Takes every delivery due at `now` out of the queue, leaving the rest.
    ///
    /// One of the four places the implementation locks `self.owed` — `owe`, this, `keep_owed`,
    /// `deliveries` — and in none of them does the guard outlive the statements that read and write
    /// the vector. **That is the rule this type keeps: an `owed` guard is never held across a
    /// call.** It is not checkable by a lint, so it is checkable by reading four short methods,
    /// which is why there are four of them and why `self.owed` is named nowhere else.
    /// (`grep 'self.owed' swarm.rs` is the audit; the unit tests seed the queue through `swarm.owed`
    /// and take no guard across anything either.) `owe` takes the same lock, `commit_caused` calls `owe`, and
    /// `redeliver` calls `commit_caused` — so a drain that held the guard for its duration
    /// deadlocked on itself the moment a cascade failed, and did it while holding `world`, which
    /// hangs every `issue`, `view` and `instances` on that swarm for ever. `tokio::sync::Mutex` is
    /// not reentrant and gives no warning; the only defence is that no guard outlives its
    /// statement, which is why `redeliver` no longer names this field at all.
    async fn take_due(&self, now: Instant) -> Vec<Delivery> {
        let mut owed = self.owed.lock().await;
        let (due, keep) = owed.drain(..).partition(|delivery| delivery.due <= now);
        *owed = keep;
        due
    }

    /// Puts deliveries back, keeping whatever arrived while they were out.
    ///
    /// Extends, never assigns. A drain that assigned the vector it computed before running the
    /// cascade overwrote the delivery the cascade had just owed — lost with no `give_up`, no
    /// `What::Undelivered` and no log line, which is the exact silence this queue exists to end.
    async fn keep_owed(&self, deliveries: Vec<Delivery>) {
        self.owed.lock().await.extend(deliveries);
    }

    /// Every delivery still owed, and how many attempts each has had.
    pub async fn deliveries(&self) -> Vec<Delivery> {
        self.owed.lock().await.clone()
    }

    /// Attempts every delivery that is due at `now`, and returns how many were attempted.
    ///
    /// `now` is passed in rather than read here for the reason every other timestamp in this system
    /// is passed in: the caller owns the clock. The trigger hands it `Instant::now()`; a test hands
    /// it a later instant and does not wait thirty seconds to find out whether the bound holds.
    ///
    /// A delivery that succeeds is pumped like any other command — see the module doc. Nothing in
    /// the kernel this repository ships binds an event that these three commands emit, so the
    /// cascade is empty as things stand; running it anyway is what keeps that a fact about
    /// `components.yaml` rather than a difference between this method and `issue`.
    ///
    /// A binding added tomorrow on one of those events needs no change here — which is a claim the
    /// same sentence made before that binding hung the swarm, so it is now written down as what it
    /// rests on: the queue is not locked while the cascade runs (see [`Swarm::take_due`]), and a
    /// delivery the cascade owes is added to the queue rather than overwritten by it (see
    /// [`Swarm::keep_owed`]). `tests/the_cascade_of_a_caused_command.rs` adds exactly such a
    /// binding to a copy of the kernel and drives it.
    ///
    /// What this does NOT rest on is the idempotency key refusing a second landing. See
    /// [`Delivery::request`]: the key is stream-scoped and a creating command mints a new stream per
    /// attempt. A delivery lands at most once because it is held once and dropped on success, in
    /// this process, and for as long as this process lives.
    pub async fn redeliver(&self, now: Instant) -> usize {
        // Taken before the world is locked, and put back after it is released. Nothing is held
        // across the attempts but `world`, which the commands need.
        let due = self.take_due(now).await;
        if due.is_empty() {
            return 0;
        }
        let attempted = due.len();
        let mut keep = Vec::new();

        {
            let mut world = self.world.lock().await;

            for mut delivery in due {
                delivery.attempts += 1;
                let minted = mint();
                // No actor, for the reason the pump has none: a binding is the system acting on
                // itself.
                let result = apply(
                    self.spec.ir(),
                    &world,
                    None,
                    &delivery.command,
                    &delivery.input,
                    Some(&minted),
                );

                let applied = match result {
                    Ok(applied) => applied,
                    Err(why) => {
                        self.defer_or_give_up(&mut delivery, why.to_string(), now, &mut keep);
                        continue;
                    }
                };

                // The command applied and the log would not take it. Keeping the delivery is the
                // only answer that does not lose the write, and the world is left untouched so that
                // what is in memory still matches what is on disk.
                if let Err(why) = self
                    .store
                    .commit(&applied, None, &Issuer::Runtime, &delivery.request)
                    .await
                {
                    self.defer_or_give_up(&mut delivery, why.to_string(), now, &mut keep);
                    continue;
                }

                if let Some(instance) = applied.instance.clone() {
                    world.insert((instance.entity.clone(), instance.id.clone()), instance);
                }
                tracing::info!(swarm = %self.slug, binding = %delivery.binding,
                               attempts = delivery.attempts, "a delivery was made at last");
                self.announce(What::Routed {
                    binding: delivery.binding.clone(),
                    command: delivery.command.clone(),
                    outcome: applied.outcome.clone(),
                    instance: applied
                        .instance
                        .as_ref()
                        .and_then(|instance| serde_json::to_value(instance).ok()),
                });

                // Whatever the redelivered command set off. A failure in the cascade is owed the
                // same way any other failed delivery is, under a key of its own so it cannot
                // collide with the delivery that caused it.
                self.pump_cascade(
                    &mut world,
                    &applied.events,
                    &format!("{}:cascade", delivery.request),
                    &delivery.binding,
                )
                .await;
            }
        }

        self.keep_owed(keep).await;
        attempted
    }

    /// Routes what a command emitted, and commits what that caused.
    ///
    /// The three doors — `issue`, `tick`, `redeliver` — differ in how a command arrives and in
    /// nothing after it has been applied, which is what this method is for. `what` names the thing
    /// being pumped for the log line, because a cascade that cannot be routed is worth attributing.
    async fn pump_cascade(&self, world: &mut World, events: &[Emitted], request: &str, what: &str) {
        match pump(self.spec.ir(), world, events.to_vec(), &mut mint_each()) {
            Ok(caused) => {
                if let Err(why) = self.commit_caused(&caused, request).await {
                    tracing::error!(swarm = %self.slug, what, error = %why,
                                    "a caused command's cascade could not be committed");
                }
            }
            Err(why) => {
                tracing::error!(swarm = %self.slug, what, error = %why,
                                "a command's events could not be routed");
            }
        }
    }

    /// Sets a failed attempt aside for later, or stops trying and says so.
    ///
    /// The two failure arms of a drain — could not apply, could not commit — decided this
    /// identically and wrote it out twice, which is how one of them came to be silent at the bound.
    /// One place to decide it in, and `keep` is the drain's own list so the queue is not touched.
    fn defer_or_give_up(
        &self,
        delivery: &mut Delivery,
        why: String,
        now: Instant,
        keep: &mut Vec<Delivery>,
    ) {
        delivery.why = why;
        delivery.due = now + RETRY_DELAY;
        if delivery.attempts >= MAX_ATTEMPTS {
            self.give_up(delivery);
        } else {
            keep.push(delivery.clone());
        }
    }

    /// Runs one occurrence of a periodic binding.
    ///
    /// Returns `false` when the host judged the occurrence ineligible — a swarm that is not running
    /// does not turn its loop, and that is a quiet non-event rather than a refusal.
    pub async fn tick(&self, occurrence: Occurrence) -> Result<bool, Refused> {
        let mut world = self.world.lock().await;
        if !self.control.running() {
            return Ok(false);
        }
        let goal = occurrence
            .context
            .get("goal_id")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();
        let iterations = occurrence
            .read
            .get("iterations")
            .and_then(Json::as_u64)
            .unwrap_or_default();

        let Some(turned) = tick(self.spec.ir(), &mut world, &occurrence, &mut mint_each())
            .map_err(|why| Refused::Routing(why.to_string()))?
        else {
            return Ok(false);
        };

        let request = format!("tick:{}", mint());
        match &turned.result {
            Ok(applied) => {
                self.store
                    .commit(applied, None, &Issuer::Runtime, &request)
                    .await
                    .map_err(|why| Refused::Store(why.to_string()))?;
                self.announce(What::Ticked {
                    binding: turned.binding.clone(),
                    command: turned.command.clone(),
                    goal,
                    iterations,
                    instance: applied
                        .instance
                        .as_ref()
                        .and_then(|instance| serde_json::to_value(instance).ok()),
                });

                // A command a period caused causes its bindings, exactly as one a caller asked for
                // does. This was missing until 2026-09-12 while the module doc said otherwise, and
                // adding it is a real change of behaviour: an event a periodic command emits now
                // reaches every binding that names it. Nothing in the kernel this repository ships
                // names one — `turn-the-loop` invokes `Pursue`, and no binding's `when` is
                // `GoalPursued` — so the change is visible only to a specification that adds one,
                // which is the whole point of the specification being the system.
                self.pump_cascade(
                    &mut world,
                    &applied.events,
                    &format!("{request}:cascade"),
                    &turned.binding,
                )
                .await;
            }
            // A periodic binding's own failure, handled the way an event-caused one is. The kernel's
            // only periodic binding declares `at_most_once`/`drop`, so `owe` will decline it and say
            // so — but the arm is written rather than assumed, because which policy a binding
            // declares is the specification's to change and not this method's to know.
            Err(why) => self.owe(&turned, &why.to_string(), &request).await,
        }
        Ok(true)
    }

    /// Makes sure one agent is a record and has somewhere to be written to.
    ///
    /// The coordinator was a process and nothing else: `swarm.agent.Agent` held no instances, so
    /// the `[coordinator]` of the birth graph was drawn by the UI and was not a thing the model
    /// knew about. Nothing could address it, and a swarm's own agent could not be sent anything.
    ///
    /// Done by the host and not by a binding, for the reason everything else here is: `Spawn` needs
    /// a swarm id and a role, and no event carries both at the moment a swarm starts.
    ///
    /// Idempotent, and called before every turn rather than once at start, so a swarm created
    /// before this existed gets its coordinator the next time the loop turns.
    ///
    /// `role` and `display_name` are arguments and not constants because a swarm has more than one
    /// kind of member as of 2026-09-13. There is deliberately no second door that hardcodes
    /// `Coordinator` — that is the shape [`Swarm::record_spend`] records above as the defect: a
    /// hand-kept rule that every caller should pass the right role is what produced eleven logs in
    /// which four differently-named agents all carry the coordinator's.
    pub async fn ensure_agent(
        &self,
        agent_id: &str,
        role: &str,
        display_name: &str,
    ) -> Result<(), Refused> {
        let Some(swarm_id) = self.swarm_id().await else {
            return Ok(());
        };

        let known = {
            let world = self.world.lock().await;
            world.contains_key(&("swarm.agent.Agent".to_owned(), agent_id.to_owned()))
        };
        if !known {
            let mut input = Map::new();
            input.insert("agent_id".into(), Json::String(agent_id.to_owned()));
            input.insert("swarm_id".into(), Json::String(swarm_id.clone()));
            input.insert("role".into(), Json::String(role.to_owned()));
            // `Claude`, because `swarm.config.Harness` declares `[Claude, Codex, B10x]` and
            // nothing else. This line said `"ClaudeCode"` until 2026-09-18 — not a variant, and so
            // not a value `coordinator.rs`'s `Some("Claude")` could ever route: the harness every
            // agent in this repository's history was spawned on was unreadable by the only code
            // that reads one. The interpreter refuses it now
            // (`ess-runtime/src/apply.rs::require_declared_variants`), so this is checked rather
            // than remembered.
            input.insert("harness".into(), Json::String("Claude".into()));
            input.insert("display_name".into(), Json::String(display_name.to_owned()));
            input.insert("host".into(), Json::Object(Map::new()));
            self.issue(
                None,
                "swarm.agent.Spawn",
                input,
                &format!("spawn:{agent_id}"),
            )
            .await?;
        }

        // One mailbox, named `main`, created by the host. Any further one the agent opens itself
        // with `swarm mailbox <name>`. The 2026-09-11 board rejected auto-creation outright; one is
        // the compromise, because an agent that cannot receive until it thinks to ask for a mailbox
        // cannot be written to by anybody who arrives first.
        let has_mailbox = self
            .view("swarm.mailbox.OpenMailboxes")
            .await?
            .iter()
            .any(|row| {
                row.get("agent_id").and_then(Json::as_str) == Some(agent_id)
                    && row.get("name").and_then(Json::as_str) == Some("main")
            });
        if !has_mailbox {
            let mut input = Map::new();
            input.insert("swarm_id".into(), Json::String(swarm_id));
            input.insert("agent_id".into(), Json::String(agent_id.to_owned()));
            input.insert("name".into(), Json::String("main".into()));
            input.insert("created_at".into(), Json::String(now()));
            self.issue(
                Some("swarm.mailbox.SwarmAgent"),
                "swarm.mailbox.OpenMailbox",
                input,
                &format!("mailbox:{agent_id}:main"),
            )
            .await?;
        }
        Ok(())
    }

    /// The unread messages addressed to one agent, newest first.
    pub async fn unread_for(&self, agent_id: &str) -> Vec<Map<String, Json>> {
        self.view("swarm.mailbox.UnreadMessages")
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.get("recipient_id").and_then(Json::as_str) == Some(agent_id))
            .collect()
    }

    /// The swarm's own record id, once `CreateSwarm` has made one.
    pub async fn swarm_id(&self) -> Option<String> {
        let world = self.world.lock().await;
        world
            .values()
            .find(|instance| instance.entity == "swarm.manager.Swarm")
            .map(|instance| instance.id.clone())
    }

    /// Posts one message to one mailbox, addressed by the pair the sender knows.
    ///
    /// `to` is `agent` or `agent/mailbox`; `main` is assumed when the mailbox is not named. The
    /// lookup is the host's job and is the reason no binding can do this: a binding maps one event
    /// field per input and has no expressions, so it cannot turn an agent into that agent's
    /// mailbox.
    pub async fn post(&self, mail: Outgoing<'_>) -> Result<Posted, Refused> {
        let (agent, mailbox) = match mail.to.split_once('/') {
            Some((agent, name)) => (agent, name),
            None => (mail.to, "main"),
        };

        let open = self.view("swarm.mailbox.OpenMailboxes").await?;
        let found = open.iter().find(|row| {
            row.get("agent_id").and_then(Json::as_str) == Some(agent)
                && row.get("name").and_then(Json::as_str) == Some(mailbox)
        });
        let Some(row) = found else {
            return Err(Refused::View(format!(
                "no open mailbox `{agent}/{mailbox}`"
            )));
        };
        let mailbox_id = row
            .get("mailbox_id")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();
        let swarm_id = row
            .get("swarm_id")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();

        let message = self
            .deliver(&mailbox_id, &swarm_id, agent, &mail, None)
            .await?;
        Ok(Posted {
            broadcast_id: None,
            messages: vec![message],
        })
    }

    /// Posts one message to every open mailbox in the swarm — the dynamic fan-out.
    ///
    /// ESS cannot express this: one outcome creates one instance, and a binding invokes one command
    /// against one instance. So the host does what the periodic binding's host already does — reads
    /// a view and issues the command once per row. The copies share one `broadcast_id`, which is
    /// what makes "who has not acked" answerable.
    ///
    /// Each copy is its own request, so a broadcast to thirty mailboxes is thirty pumps of one
    /// event rather than one pump of thirty, and `MAX_DEPTH` is never at risk.
    pub async fn broadcast(&self, mail: Outgoing<'_>) -> Result<Posted, Refused> {
        let open = self.view("swarm.mailbox.OpenMailboxes").await?;
        let broadcast_id = mint();
        let mut messages = Vec::new();

        for row in open {
            let (Some(mailbox_id), Some(recipient)) = (
                row.get("mailbox_id").and_then(Json::as_str),
                row.get("agent_id").and_then(Json::as_str),
            ) else {
                continue;
            };
            // A sender does not broadcast to itself: a copy it has to read and ack is noise it
            // already knows.
            if recipient == mail.sender {
                continue;
            }
            let swarm_id = row
                .get("swarm_id")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_owned();
            messages.push(
                self.deliver(
                    mailbox_id,
                    &swarm_id,
                    recipient,
                    &mail,
                    Some(broadcast_id.clone()),
                )
                .await?,
            );
        }

        Ok(Posted {
            broadcast_id: Some(broadcast_id),
            messages,
        })
    }

    /// One `PostMessage`, with the clock read here because the model does not read one.
    async fn deliver(
        &self,
        mailbox_id: &str,
        swarm_id: &str,
        recipient: &str,
        mail: &Outgoing<'_>,
        broadcast_id: Option<String>,
    ) -> Result<String, Refused> {
        let mut input = Map::new();
        input.insert("mailbox_id".into(), Json::String(mailbox_id.to_owned()));
        input.insert("swarm_id".into(), Json::String(swarm_id.to_owned()));
        input.insert("recipient_id".into(), Json::String(recipient.to_owned()));
        input.insert("sender_id".into(), Json::String(mail.sender.to_owned()));
        input.insert("subject".into(), Json::String(mail.subject.to_owned()));
        input.insert("body".into(), Json::String(mail.body.to_owned()));
        input.insert("sent_at".into(), Json::String(now()));
        if let Some(parent) = mail.reply_to {
            input.insert("parent_message_id".into(), Json::String(parent.to_owned()));
        }
        if let Some(broadcast) = broadcast_id {
            input.insert("broadcast_id".into(), Json::String(broadcast));
        }

        // Which of the mailbox domain's TWO actors this is. `swarm.mailbox.SwarmAgent` may open,
        // post, read and ack; `swarm.mailbox.Operator` `may: PostMessage` and nothing else,
        // because "a person may write to a running swarm" and "an ack the recipient did not
        // perform is the one lie this domain exists to prevent" (`src/core/domains/mailbox.yaml`).
        // A hand at this door is the second, and saying otherwise would put a claim to membership
        // in the actor column of every message a person ever sent.
        let actor = match mail.issuer {
            Issuer::Operator => "swarm.mailbox.Operator",
            _ => "swarm.mailbox.SwarmAgent",
        };
        let request = format!("mail:{}", mint());
        let issued = self
            .issue_as(
                &mail.issuer,
                Some(actor),
                "swarm.mailbox.PostMessage",
                input,
                &request,
            )
            .await?;

        issued
            .events
            .iter()
            .find(|event| event.name == "swarm.mailbox.MessagePosted")
            .and_then(|event| event.fields.get("message_id"))
            .and_then(Json::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| Refused::Command("the message was not posted".to_owned()))
    }

    /// One view, computed over the world as it stands.
    pub async fn view(&self, name: &str) -> Result<Vec<Map<String, Json>>, Refused> {
        let world = self.world.lock().await;
        view(self.spec.ir(), &world, name)
            .map(|computed| computed.rows)
            .map_err(|why| Refused::View(why.to_string()))
    }

    /// Every instance of one entity, whole. What the canvas draws.
    pub async fn instances(&self, entity: &str) -> Vec<Json> {
        let world = self.world.lock().await;
        world
            .values()
            .filter(|instance| instance.entity == entity)
            .filter_map(|instance| serde_json::to_value(instance).ok())
            .collect()
    }

    /// Throws the in-memory world away and rebuilds it from the log.
    ///
    /// The in-memory copy is a cache of the replay; this is how that claim is checked rather than
    /// asserted.
    pub async fn reload(&self) -> Result<(), Refused> {
        let rebuilt = self
            .store
            .world(self.spec.ir())
            .await
            .map_err(|why| Refused::Store(why.to_string()))?;
        *self.world.lock().await = rebuilt;
        self.announce(What::Reloaded);
        Ok(())
    }
}

/// An identity for something the specification says the implementation mints.
fn mint() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// The same, as the interpreter asks for them.
fn mint_each() -> impl FnMut() -> String {
    || mint()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A redelivery that succeeds, proved from a state `store.rs` admits: ONE handle on the log.
    ///
    /// The end-to-end proof in `tests/serves_a_swarm.rs` manufactures its transient failure by
    /// opening two `Swarm` handles over one SQLite file, which is a second connection to a database
    /// the store's own doc says has one writer. This one owes the delivery by hand instead — the
    /// queue is right here — so the thing being proved (a due delivery applies, commits, and stops
    /// being owed) rests on nothing the module excludes.
    #[tokio::test]
    async fn a_due_delivery_that_applies_is_committed_and_no_longer_owed() {
        let data = tempdir::TempDir::new("swarm-owed").expect("a scratch directory");
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Swarm::open(spec, data.path(), "owed")
            .await
            .expect("a swarm opens");

        let object = |value: Json| value.as_object().expect("an object").clone();
        swarm
            .issue(
                None,
                "swarm.manager.CreateSwarm",
                object(
                    serde_json::json!({"display_name": "Owed", "tmux_session": "s",
                       "home": "/tmp/owed", "created_at": "2026-09-12T10:00:00Z"}),
                ),
                "make",
            )
            .await
            .expect("the swarm is created");
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        swarm
            .issue(
                None,
                "swarm.config.DraftConfig",
                object(
                    serde_json::json!({"swarm_id": swarm_id, "paths": {}, "launch": {},
                       "schedules": {}, "budgets": {}, "board": {}}),
                ),
                "draft",
            )
            .await
            .expect("a config is drafted");
        let config_id = swarm.instances("swarm.config.Config").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();

        // One delivery owed, exactly as `owe()` would have left it after a first attempt that lost
        // a race with the write that created the swarm.
        swarm.owed.lock().await.push(Delivery {
            binding: "adopt-activated-config".to_owned(),
            command: "swarm.manager.AdoptConfig".to_owned(),
            attempts: 1,
            why: "the world had no such swarm yet".to_owned(),
            input: object(
                serde_json::json!({"swarm_id": swarm_id, "config_id": config_id.clone()}),
            ),
            request: "activate:adopt-activated-config".to_owned(),
            due: Instant::now(),
        });

        assert_eq!(swarm.redeliver(Instant::now()).await, 1);
        assert!(
            swarm.deliveries().await.is_empty(),
            "a delivery that succeeded is no longer owed"
        );
        assert_eq!(
            swarm.instances("swarm.manager.Swarm").await[0]["fields"]["active_config_id"],
            serde_json::json!(config_id)
        );

        // In the log, not only in the world the retry wrote to.
        swarm.reload().await.expect("the log replays");
        assert_eq!(
            swarm.instances("swarm.manager.Swarm").await[0]["fields"]["active_config_id"],
            serde_json::json!(config_id)
        );
    }

    /// A delivery not yet due is not attempted, and one past the bound is given up with a word.
    #[tokio::test]
    async fn a_delivery_that_cannot_apply_is_bounded_and_announced() {
        let data = tempdir::TempDir::new("swarm-bound").expect("a scratch directory");
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Swarm::open(spec, data.path(), "bound")
            .await
            .expect("a swarm opens");
        let mut watching = swarm.watch();

        let began = Instant::now();
        swarm.owed.lock().await.push(Delivery {
            binding: "adopt-activated-config".to_owned(),
            command: "swarm.manager.AdoptConfig".to_owned(),
            attempts: 1,
            why: "no such swarm".to_owned(),
            // A swarm no command ever created: this never applies, however often it is attempted.
            input: serde_json::json!({"swarm_id": "33333333-3333-4333-8333-333333333333",
                                      "config_id": "44444444-4444-4444-8444-444444444444"})
            .as_object()
            .expect("an object")
            .clone(),
            request: "never:adopt-activated-config".to_owned(),
            due: began + RETRY_DELAY,
        });

        assert_eq!(swarm.redeliver(began).await, 0, "not due yet");
        for attempt in 2..=MAX_ATTEMPTS {
            assert_eq!(
                swarm.redeliver(began + RETRY_DELAY * (attempt - 1)).await,
                1
            );
        }
        assert!(swarm.deliveries().await.is_empty(), "bounded, and given up");

        // The watcher is told which delivery was lost and after how many attempts. A delivery that
        // is dropped in silence is the defect this whole change exists to end.
        let mut said = None;
        while let Ok(change) = watching.try_recv() {
            if let What::Undelivered { attempts, .. } = change.what {
                said = Some(attempts);
            }
        }
        assert_eq!(said, Some(MAX_ATTEMPTS), "the loss was announced");
    }
}

#[cfg(test)]
mod attribution {
    use super::*;

    /// The half of the bound that `dsfsdf` could not have exposed, because one agent worked it.
    ///
    /// `spend_on` folds on `goal` alone. With N agents on one goal every row lands in one bucket,
    /// so the figures say what the goal cost and can never say what an agent cost — and an
    /// unattributable figure is one no per-agent ceiling can ever be written against. The row must
    /// name the agent that spent it, and the fold must be able to key on it.
    #[tokio::test]
    async fn spend_is_attributable_per_agent_as_well_as_per_goal() {
        let data = tempdir::TempDir::new("swarm-attribution").expect("a scratch directory");
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Swarm::open(spec, data.path(), "attribution")
            .await
            .expect("a swarm opens");
        let goal = "77fc1fcc-a89c-4fb9-b041-89ecfc75922f";

        let cost = |usd: f64| {
            let mut spent = Spent::default();
            spent.cost_usd = Some(usd);
            spent
        };
        swarm.record_spend("coordinator", goal, 1, &cost(1.00), Some(false), None);
        swarm.record_spend("worker-a", goal, 2, &cost(2.00), Some(false), None);
        swarm.record_spend("worker-a", goal, 3, &cost(4.00), Some(false), None);

        // Every row names who spent it. A row that does not is one the fold below silently loses.
        let text = std::fs::read_to_string(swarm.dir().join("turns").join("spend.jsonl"))
            .expect("the spend record exists");
        for line in text.lines() {
            let row: Json = serde_json::from_str(line).expect("a row");
            assert!(
                row.get("agent").and_then(Json::as_str).is_some(),
                "every recorded turn names the agent that spent it: {row}"
            );
        }

        // The goal's total is unchanged by the split.
        let (all, turns) = swarm.spend_on(goal);
        assert_eq!(turns, 3);
        assert_eq!(all.cost_usd, Some(7.00));

        // And each agent's share is readable on its own.
        let (coordinator, its_turns) = swarm.spend_by(goal, "coordinator");
        assert_eq!(its_turns, 1);
        assert_eq!(coordinator.cost_usd, Some(1.00));

        let (worker, its_turns) = swarm.spend_by(goal, "worker-a");
        assert_eq!(its_turns, 2);
        assert_eq!(worker.cost_usd, Some(6.00));

        // An agent that has spent nothing on this goal is zero, not the goal's total.
        let (nobody, none) = swarm.spend_by(goal, "worker-b");
        assert_eq!(none, 0);
        assert_eq!(nobody.cost_usd, None);
    }

    /// The fold a ceiling that is not keyed on a goal is read from.
    ///
    /// `spend_by` needs a goal to be given, so a caller that wants an agent's whole record has to
    /// know every goal it ever worked — which is the reason `story:spend-is-bounded-per-goal-only`
    /// existed. `spend_by_agent` asks nothing about the goal, and mutation-checking it means
    /// keying it on the goal again: do that and the $6.00 below reads $3.00.
    #[tokio::test]
    async fn an_agents_total_asks_nothing_about_the_goal() {
        let data = tempdir::TempDir::new("swarm-agent-total").expect("a scratch directory");
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Swarm::open(spec, data.path(), "agent-total")
            .await
            .expect("a swarm opens");

        let cost = |usd: f64| {
            let mut spent = Spent::default();
            spent.cost_usd = Some(usd);
            spent
        };
        for goal in ["goal-one", "goal-two"] {
            for turn in 1..=3 {
                swarm.record_spend("worker-a", goal, turn, &cost(1.00), Some(false), None);
            }
        }
        swarm.record_spend("worker-b", "goal-one", 1, &cost(9.00), Some(false), None);

        // Six turns and $6.00, spread over two goals, read without naming either.
        let (spent, turns) = swarm.spend_by_agent("worker-a");
        assert_eq!(turns, 6);
        assert_eq!(spent.cost_usd, Some(6.00));

        // Neither goal on its own can see it: three turns and $3.00 each, both inside the defaults.
        for goal in ["goal-one", "goal-two"] {
            let (spent, turns) = swarm.spend_by(goal, "worker-a");
            assert_eq!(turns, 3);
            assert_eq!(spent.cost_usd, Some(3.00));
        }

        // One agent's total is not another's, and is not the swarm's.
        let (other, its_turns) = swarm.spend_by_agent("worker-b");
        assert_eq!(its_turns, 1);
        assert_eq!(other.cost_usd, Some(9.00));
        let (nobody, none) = swarm.spend_by_agent("worker-c");
        assert_eq!(none, 0);
        assert_eq!(nobody.cost_usd, None);
    }
}

#[cfg(test)]
mod roster {
    use super::*;

    /// A created swarm, and its own record id.
    async fn a_swarm(data: &tempdir::TempDir, slug: &str) -> (Arc<Swarm>, String) {
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Arc::new(Spec::load(kernel).expect("the kernel resolves"));
        let swarm = Arc::new(
            Swarm::open(spec, data.path(), slug)
                .await
                .expect("a swarm opens"),
        );
        swarm
            .issue(
                None,
                "swarm.manager.CreateSwarm",
                serde_json::json!({"display_name": slug, "tmux_session": slug,
                                   "home": "/nowhere", "created_at": "2026-09-13T10:00:00Z"})
                .as_object()
                .expect("an object")
                .clone(),
                "create",
            )
            .await
            .expect("the swarm is created");
        let id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        (swarm, id)
    }

    /// Spawn clause 1: a role that is not `Coordinator` exists.
    ///
    /// Measured 2026-09-13 across all 11 logs under `data/swarms/*/eventlog.sqlite3`: the swarm
    /// `mail-check-1789197540` spawned `coordinator`, `reviewer`, `builder` and `tester`, and
    /// every one of them carries role `Coordinator` — because the enum offered nothing else. That
    /// is the defect, observed in a real log rather than argued from the type.
    #[tokio::test]
    async fn a_role_that_is_not_the_coordinator_is_declared() {
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Spec::load(kernel).expect("the kernel resolves");
        let types = serde_json::to_value(spec.ir().types()).expect("the IR serialises");
        let variants: Vec<&str> = types["swarm.agent.Role"]["body"]["variants"]
            .as_array()
            .expect("an enum has variants")
            .iter()
            .filter_map(Json::as_str)
            .collect();

        assert!(
            variants.contains(&"Coordinator"),
            "the role a swarm is born with is still there: {variants:?}"
        );
        assert!(
            variants.iter().any(|variant| *variant != "Coordinator"),
            "and a swarm that spawns a member can call it something else: {variants:?}"
        );
    }

    /// Spawn clause 2: a spawned agent with that role gets its own mailbox, and `MailboxOpened`
    /// names it.
    ///
    /// The mailbox is what makes a second agent addressable rather than merely recorded. Two
    /// agents sharing one mailbox would be one agent with two names.
    #[tokio::test]
    async fn a_second_agent_gets_a_mailbox_of_its_own() {
        let data = tempdir::TempDir::new("swarm-roster").expect("a scratch directory");
        let (swarm, _) = a_swarm(&data, "roster").await;

        swarm
            .ensure_agent("coordinator", "Coordinator", "The coordinator")
            .await
            .expect("the coordinator is registered");
        swarm
            .ensure_agent("builder", "Worker", "A member that builds")
            .await
            .expect("the member is registered");

        let roles: Vec<(String, String)> = swarm
            .instances("swarm.agent.Agent")
            .await
            .into_iter()
            .map(|agent| {
                (
                    agent["id"].as_str().unwrap_or_default().to_owned(),
                    agent["fields"]["role"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                )
            })
            .collect();
        assert!(
            roles.contains(&("builder".to_owned(), "Worker".to_owned())),
            "the member holds the role it was spawned with, not the coordinator's: {roles:?}"
        );

        let mailboxes: Vec<(String, String)> = swarm
            .view("swarm.mailbox.OpenMailboxes")
            .await
            .expect("the view computes")
            .into_iter()
            .filter_map(|row| {
                Some((
                    row.get("agent_id")?.as_str()?.to_owned(),
                    row.get("mailbox_id")?.as_str()?.to_owned(),
                ))
            })
            .collect();
        let coordinators: Vec<&String> = mailboxes
            .iter()
            .filter(|(agent, _)| agent == "coordinator")
            .map(|(_, id)| id)
            .collect();
        let members: Vec<&String> = mailboxes
            .iter()
            .filter(|(agent, _)| agent == "builder")
            .map(|(_, id)| id)
            .collect();
        assert_eq!(coordinators.len(), 1, "one each: {mailboxes:?}");
        assert_eq!(members.len(), 1, "one each: {mailboxes:?}");
        assert_ne!(
            coordinators[0], members[0],
            "and the member's is its own — a shared mailbox is one agent with two names"
        );

        let log = swarm.history(200).await.expect("the log reads back");
        let opened = log
            .iter()
            .filter(|event| event.name == "swarm.mailbox.MailboxOpened")
            .filter(|event| event.fields["agent_id"] == "builder")
            .count();
        assert_eq!(
            opened, 1,
            "MailboxOpened names the member whose mailbox it is"
        );

        // Idempotent, and called before every turn: a second pass adds neither agent nor mailbox.
        swarm
            .ensure_agent("builder", "Worker", "A member that builds")
            .await
            .expect("registering twice is not an error");
        assert_eq!(
            swarm
                .view("swarm.mailbox.OpenMailboxes")
                .await
                .expect("the view computes")
                .len(),
            mailboxes.len(),
            "asking again adds nothing"
        );
    }
}

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
    Recorded, Spec, Store, World, apply, pump, route::Occurrence, route::Routed,
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
    /// The coordinator was asked, or answered, or could not.
    Turn {
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
        goal: String,
        iterations: u64,
        seq: u64,
        spent: Spent,
        event: Json,
    },
    /// A goal used up a cap, so the loop stopped asking. Nothing in the model changed.
    Capped {
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

/// What a caller wants said, before it is addressed.
#[derive(Debug)]
pub struct Outgoing<'a> {
    /// `agent` or `agent/mailbox`. Ignored by a broadcast.
    pub to: &'a str,
    pub sender: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
    /// The message this answers, when it answers one.
    pub reply_to: Option<&'a str>,
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
            Self::Command(why) | Self::Routing(why) | Self::Store(why) | Self::View(why) => {
                f.write_str(why)
            }
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

    /// Appends one finished turn's figures to `turns/spend.jsonl`.
    ///
    /// A small file beside the transcripts, so a total can be read without reading every run.
    /// Appends one ATTEMPT's figures, whether or not it produced a verdict.
    ///
    /// A turn that was cut off before it answered still ran and still cost money, and until
    /// 2026-09-12 only answered turns were recorded — so a coordinator that never wrote a verdict
    /// spent without limit while both caps read zero.
    pub fn record_spend(
        &self,
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
            let _ = writeln!(file, "{line}");
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

    /// Applies one command, appends what it produced, and runs the bindings it set off.
    ///
    /// The lock is held across the whole of it on purpose. Applying reads the world, deciding the
    /// outcome depends on what it read, and the append records that decision — a reader that got in
    /// between would see a world no command had produced.
    pub async fn issue(
        &self,
        actor: Option<&str>,
        command: &str,
        input: Map<String, Json>,
        request: &str,
    ) -> Result<Issued, Refused> {
        let mut world = self.world.lock().await;

        let done = apply(
            self.spec.ir(),
            &world,
            actor,
            command,
            &input,
            Some(&mint()),
        )
        .map_err(|why| Refused::Command(why.to_string()))?;

        self.store
            .commit(&done, actor, request)
            .await
            .map_err(|why| Refused::Store(why.to_string()))?;

        if let Some(instance) = done.instance.clone() {
            world.insert((instance.entity.clone(), instance.id.clone()), instance);
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
                        .commit(applied, None, &self.delivery_key(request, &routed.binding))
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
                if let Err(why) = self.store.commit(&applied, None, &delivery.request).await {
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
                    .commit(applied, None, &request)
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

    /// Makes sure the coordinator is a record and has somewhere to be written to.
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
    pub async fn ensure_coordinator(&self, agent_id: &str) -> Result<(), Refused> {
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
            input.insert("role".into(), Json::String("Coordinator".into()));
            input.insert("harness".into(), Json::String("ClaudeCode".into()));
            input.insert(
                "display_name".into(),
                Json::String("The coordinator".into()),
            );
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

        let request = format!("mail:{}", mint());
        let issued = self
            .issue(
                Some("swarm.mailbox.SwarmAgent"),
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

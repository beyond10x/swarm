//! One running swarm: its log, its world, and the broadcast that tells the UI something changed.
//!
//! The world is held in memory and written through to the log, rather than re-folded on every read.
//! Both are true at once because the fold is deterministic — the in-memory copy is a cache of the
//! replay, and [`Swarm::reload`] throws it away and rebuilds when that needs proving.
//!
//! Every mutation goes through [`Swarm::issue`], which is the only place a command is applied, the
//! only place the log is written, and the only place the pump runs. A second path into the world
//! would be a second answer to what happened.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value as Json};
use tokio::sync::{Mutex, broadcast};

use crate::budget::Reached;
use crate::coordinator::Spent;
use ess_runtime::apply::Emitted;
use ess_runtime::{
    Recorded, Spec, Store, World, apply, pump, route::Occurrence, route::tick, view,
};

/// How many events a slow reader may fall behind before it is dropped and told to resync.
const BACKLOG: usize = 256;

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

/// One swarm, running.
pub struct Swarm {
    spec: Arc<Spec>,
    store: Store,
    world: Mutex<World>,
    changes: broadcast::Sender<Change>,
    slug: String,
    /// `data/swarms/<slug>`: the log, the work directory, the turns.
    dir: PathBuf,
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

        Ok(Self {
            spec,
            store,
            world: Mutex::new(world),
            changes,
            slug: slug.to_owned(),
            dir: root.join("swarms").join(slug),
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

        for routed in &caused {
            if let Ok(applied) = &routed.result {
                self.store
                    .commit(applied, None, &format!("{request}:{}", routed.binding))
                    .await
                    .map_err(|why| Refused::Store(why.to_string()))?;
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
        }

        Ok(Issued {
            outcome: done.outcome,
            error: done.error,
            events: appended,
        })
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

        if let Ok(applied) = &turned.result {
            self.store
                .commit(applied, None, &format!("tick:{}", mint()))
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

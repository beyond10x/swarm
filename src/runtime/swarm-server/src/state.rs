//! What the server holds: the specification, and every swarm open against it.
//!
//! One specification for all of them. A swarm is data, not a program — two swarms differ in what
//! their logs contain and in nothing else, which is what makes "the spec is the system" true rather
//! than aspirational.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;

use ess_runtime::Spec;

use crate::budget::{Caps, Reached};
use crate::swarm::{Refused, Summary, Swarm, now};

/// The entity a slug names one of. Its lifecycle decides what "finished" means.
const SWARM: &str = "swarm.manager.Swarm";

/// The whole server.
pub struct Server {
    spec: Arc<Spec>,
    root: PathBuf,
    swarms: RwLock<BTreeMap<String, Arc<Swarm>>>,
    /// One lock per slug, held across the construction of that slug's swarm, so at most one
    /// `Swarm::open` per slug runs at a time in this `Server`.
    ///
    /// The map's own lock cannot do this job: it has to be dropped across the await, which is what
    /// let two opens of one slug both construct a handle. It also cannot be left to the store to
    /// arbitrate — the event log sets no `busy_timeout` (`ess-runtime/src/store.rs:21-23`), so two
    /// simultaneous opens of one `eventlog.sqlite3` do not queue, one of them is refused outright.
    ///
    /// Per slug and not one lock for all of them, because `Swarm::open` is a store open plus a full
    /// replay of the log: one lock made an open of a slug with no directory, no log and nothing in
    /// common with another slug wait 253 ms for that other slug's 4000-command replay. Two clients
    /// creating two different swarms (`http.rs`, the only caller) have no reason to queue.
    ///
    /// The outer lock is `std` and is held only long enough to clone one `Arc` out of the map —
    /// never across an await. One entry per slug ever opened, which is bounded by the swarms on
    /// disk; nothing removes them, because a slug that was opened once may be opened again.
    opening: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// When this process started, RFC 3339.
    started_at: String,
    /// When the trigger will next fire, RFC 3339. `None` until the trigger has said.
    next_tick_at: Mutex<Option<String>>,
    /// How many times the trigger has fired since start.
    ticks: Mutex<u64>,
    /// Goals whose coordinator turn is running right now, as `slug/goal_id`.
    in_flight: Mutex<HashSet<String>>,
    /// Goals the loop has stopped asking about, keyed `slug/goal_id`. Holding the reason here and
    /// not only on the stream is what lets a page that connected afterwards still say which cap.
    capped: Mutex<BTreeMap<String, CappedGoal>>,
    /// What one goal may use up.
    caps: Caps,
}

impl Server {
    /// Compiles the specification and opens every swarm already on disk.
    pub async fn start(spec: Spec, root: PathBuf) -> Result<Self, Refused> {
        let spec = Arc::new(spec);
        let mut swarms = BTreeMap::new();

        // A swarm that exists is a directory that exists. Nothing is registered anywhere else, so
        // there is no index to fall out of step with the disk.
        if let Ok(entries) = std::fs::read_dir(root.join("swarms")) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let Some(slug) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                    continue;
                };
                let swarm = Swarm::open(Arc::clone(&spec), &root, &slug).await?;
                swarms.insert(slug, Arc::new(swarm));
            }
        }

        Ok(Self {
            spec,
            root,
            swarms: RwLock::new(swarms),
            opening: Mutex::new(BTreeMap::new()),
            started_at: now(),
            next_tick_at: Mutex::new(None),
            ticks: Mutex::new(0),
            in_flight: Mutex::new(HashSet::new()),
            capped: Mutex::new(BTreeMap::new()),
            caps: Caps::configured(),
        })
    }

    /// Opens a swarm, or returns the one already open.
    ///
    /// Every call for one slug returns the same handle, however many run at once — and a caller
    /// that arrives while another is still constructing waits for it rather than being refused.
    ///
    /// Opening is a read of the map, an await on `Swarm::open`, then an insert, and a second open
    /// of the same slug could finish inside that await: both missed the read, both constructed a
    /// handle, and the later insert replaced the earlier handle in the map. A handle can hold the
    /// only copy of an owed at-least-once delivery, so the replaced one took that delivery with it
    /// — no `give_up`, no `What::Undelivered`, no log line. Two handles over one event log is also
    /// two in-memory worlds, which disagree from the first command either one applies.
    ///
    /// So construction is serialised per slug on `opening`, and the map is re-read under that
    /// slug's lock: the second caller finds the first's handle and never constructs one, so there
    /// is no losing handle to drop and no insert that can land over a live one. That also answers
    /// the refusal the store would otherwise hand back: with no `busy_timeout` set
    /// (`ess-runtime/src/store.rs:21-23`), two simultaneous opens of one `eventlog.sqlite3` end
    /// with one refused `database is locked`, which reached `POST /swarms` as a `500` for a swarm
    /// that was open and healthy. Only one open of a given file is now in flight at a time.
    ///
    /// The lock is per slug, so an open of one swarm never waits on another's replay.
    ///
    /// When the store fails, the error is returned.
    pub async fn open(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        if let Some(open) = self.swarms.read().await.get(slug) {
            return Ok(Arc::clone(open));
        }

        // This slug's construction lock, and nobody else's. Taken under an `std` guard that is
        // dropped on the next line, so no await happens while it is held.
        let gate = Arc::clone(
            self.opening
                .lock()
                .expect("not poisoned")
                .entry(slug.to_owned())
                .or_default(),
        );
        let _constructing = gate.lock().await;

        // Whoever held this slug's gate before this call was opening this slug, so it is in the map
        // now and nothing is constructed twice.
        if let Some(open) = self.swarms.read().await.get(slug) {
            return Ok(Arc::clone(open));
        }

        let swarm = Arc::new(Swarm::open(Arc::clone(&self.spec), &self.root, slug).await?);
        self.swarms
            .write()
            .await
            .insert(slug.to_owned(), Arc::clone(&swarm));
        Ok(swarm)
    }

    /// Removes a swarm: its handle, its log, and the directory it lived in.
    ///
    /// Removable means **absent from the model or in a terminal state**. A place made by
    /// `POST /swarms` that no `CreateSwarm` ever followed has no `swarm.manager.Swarm` record at
    /// all, and nothing in the model can be asked about it — that is the swarm the operator could
    /// not remove and removed by hand. Anything else is refused with the error the specification
    /// already declares for a command that acts from a state the instance is not in.
    ///
    /// Which states are terminal is read from the specification, not written here: `manager.yaml`
    /// says `terminal: [Deleted]`, and a lifecycle that grows a second end should not need this
    /// file edited to agree with it.
    ///
    /// Three things this has to get right, all of them documented above:
    ///
    /// 1. **The per-slug `opening` gate is taken**, the same one `Server::open` takes. Removal and
    ///    construction of one slug are the same critical section: without it an open that missed
    ///    the map read constructs a handle while this is unlinking and inserts it afterwards,
    ///    leaving a live handle over a log that is gone.
    /// 2. **The handle leaves the map and the `Arc` is dropped before the files go.** Dropping it
    ///    closes the SQLite handle. This is the only eviction path there is — `.remove()` was never
    ///    called on `swarms` before this method existed — so nothing else will do it later.
    /// 3. **`eventlog.sqlite3`, `-wal` and `-shm` go together.** Removing the database and leaving
    ///    the WAL behind makes the next open of this slug run recovery against a fresh empty file.
    ///
    /// A handle held elsewhere — the trigger walks `all()` — keeps its own `Arc` alive, so the
    /// close happens when that reader lets go. The files are unlinked either way: on this platform
    /// an open descriptor does not block the unlink, and the slug is out of the map, so nothing
    /// hands that handle to a new caller.
    pub async fn remove(&self, slug: &str) -> Result<(), Removal> {
        // The same gate `open` takes, and for the same reason: at most one of "construct this
        // slug" and "remove this slug" runs at a time.
        let gate = Arc::clone(
            self.opening
                .lock()
                .expect("not poisoned")
                .entry(slug.to_owned())
                .or_default(),
        );
        let _removing = gate.lock().await;

        let held = self.swarms.read().await.get(slug).map(Arc::clone);
        let directory = self.root.join("swarms").join(slug);
        if held.is_none() && !directory.exists() {
            return Err(Removal::NotHere(slug.to_owned()));
        }

        // What the model says about it, if the model has been told it exists at all.
        if let Some(swarm) = &held {
            let state = swarm.summary().await.state;
            if let Some(state) = state
                && !self.terminal_states().contains(&state)
            {
                return Err(Removal::StateConflict {
                    slug: slug.to_owned(),
                    state,
                });
            }
        }

        // Out of the map, then the last `Arc` this function holds, then the files. In that order.
        self.swarms.write().await.remove(slug);
        drop(held);

        // The log and everything SQLite keeps beside it, together. A `-wal` that outlives its
        // database is read as a journal to recover the next time this slug is opened.
        let database = directory.join("eventlog.sqlite3");
        for file in [
            database.clone(),
            with_suffix(&database, "-wal"),
            with_suffix(&database, "-shm"),
        ] {
            match std::fs::remove_file(&file) {
                Ok(()) => {}
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
                Err(why) => {
                    return Err(Removal::Undeletable {
                        path: file.display().to_string(),
                        why: why.to_string(),
                    });
                }
            }
        }

        match std::fs::remove_dir_all(&directory) {
            Ok(()) => Ok(()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(Removal::Undeletable {
                path: directory.display().to_string(),
                why: why.to_string(),
            }),
        }
    }

    /// The states the specification calls ends for a swarm.
    ///
    /// Read from the compiled IR rather than spelled here, so `terminal: [Deleted]` remains the one
    /// place that decides it.
    fn terminal_states(&self) -> Vec<String> {
        self.spec
            .ir()
            .entities()
            .iter()
            .find(|(name, _)| name.to_string() == SWARM)
            .map(|(_, entity)| {
                entity
                    .lifecycle
                    .terminal
                    .iter()
                    .map(|state| state.as_str().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One open swarm.
    pub async fn get(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        self.swarms
            .read()
            .await
            .get(slug)
            .map(Arc::clone)
            .ok_or_else(|| Refused::View(format!("no swarm `{slug}`")))
    }

    /// Every open swarm, by slug. Terminal ones included: this is what the process holds.
    pub async fn slugs(&self) -> Vec<String> {
        self.swarms.read().await.keys().cloned().collect()
    }

    /// Every swarm a client is shown, by slug — which is every open one that is not finished.
    ///
    /// `DeleteSwarm` promises the swarm "no longer appears in the swarms list"
    /// (`manager.yaml:326`), and for as long as the list was the keys of the handle map that
    /// promise was simply false: six swarms sat in `Deleted` and all six were listed. Terminal is
    /// the specification's word, read from the lifecycle, not a name matched here.
    ///
    /// Separate from `slugs()` rather than a filter inside it: what the process HOLDS and what a
    /// client is SHOWN are two different questions, and boot logs the first.
    pub async fn listed(&self) -> Vec<String> {
        let terminal = self.terminal_states();
        // The map's lock is dropped before the first await: a summary takes the swarm's own world
        // lock, and holding the map across that is how two locks become an order to get wrong.
        let held: Vec<(String, Arc<Swarm>)> = self
            .swarms
            .read()
            .await
            .iter()
            .map(|(slug, swarm)| (slug.clone(), Arc::clone(swarm)))
            .collect();

        let mut shown = Vec::new();
        for (slug, swarm) in held {
            match swarm.summary().await.state {
                Some(state) if terminal.contains(&state) => {}
                _ => shown.push(slug),
            }
        }
        shown
    }

    /// Every open swarm, for the trigger to walk.
    pub async fn all(&self) -> Vec<Arc<Swarm>> {
        self.swarms.read().await.values().map(Arc::clone).collect()
    }

    /// The specification, for anything that needs to read the model.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// The trigger says when it will next fire, and that it just did.
    pub fn tick_scheduled(&self, in_: Duration) {
        let at = time::OffsetDateTime::now_utc() + in_;
        *self.next_tick_at.lock().expect("not poisoned") = at
            .format(&time::format_description::well_known::Rfc3339)
            .ok();
    }

    /// One more firing of the trigger.
    pub fn ticked(&self) {
        *self.ticks.lock().expect("not poisoned") += 1;
    }

    /// Claims a goal for a coordinator turn. `false` when one is already running for it, which is
    /// the `overlap: serial_per_instance` the binding declares, enforced here by the host.
    pub fn claim_turn(&self, slug: &str, goal_id: &str) -> bool {
        self.in_flight
            .lock()
            .expect("not poisoned")
            .insert(format!("{slug}/{goal_id}"))
    }

    /// The turn is over, whichever way.
    pub fn release_turn(&self, slug: &str, goal_id: &str) {
        self.in_flight
            .lock()
            .expect("not poisoned")
            .remove(&format!("{slug}/{goal_id}"));
    }

    /// How many coordinator turns are running right now.
    pub fn turns_in_flight(&self) -> usize {
        self.in_flight.lock().expect("not poisoned").len()
    }

    /// What one goal may use up.
    pub fn caps(&self) -> Caps {
        self.caps
    }

    /// Records that a goal has been capped. `false` when it already was, so the report is made
    /// once rather than every period.
    pub fn report_capped(&self, capped: CappedGoal) -> bool {
        self.capped
            .lock()
            .expect("not poisoned")
            .insert(format!("{}/{}", capped.swarm, capped.goal), capped)
            .is_none()
    }

    /// Forgets a cap report, so raising a cap reports the next one afresh.
    pub fn forget_capped(&self, slug: &str, goal_id: &str) {
        self.capped
            .lock()
            .expect("not poisoned")
            .remove(&format!("{slug}/{goal_id}"));
    }

    /// Every goal the loop has stopped asking about, with why.
    pub fn capped_goals(&self) -> Vec<CappedGoal> {
        self.capped
            .lock()
            .expect("not poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Every periodic binding, with the period the specification declares for it.
    pub fn periods(&self) -> Vec<(String, Duration)> {
        self.spec
            .ir()
            .bindings()
            .iter()
            .filter_map(|(name, binding)| {
                let periodic = binding.cause.periodic()?;
                Some((
                    name.to_string(),
                    Duration::from_secs(u64::from(periodic.contract.every.seconds())),
                ))
            })
            .collect()
    }

    /// Where the runtime is: what it runs, since when, what it drives, and every swarm in one row.
    pub async fn status(&self) -> Status {
        let mut swarms = Vec::new();
        for swarm in self.all().await {
            let mut summary = swarm.summary().await;
            summary.watchers = swarm.watchers();
            let events = swarm.count().await.unwrap_or_default();
            swarms.push(SwarmStatus { summary, events });
        }
        let coordinator = crate::coordinator::configured();
        Status {
            system: self.spec.ir().system().to_string(),
            spec_files: self.spec.files(),
            started_at: self.started_at.clone(),
            now: now(),
            periodic: self
                .periods()
                .into_iter()
                .map(|(binding, every)| Periodic {
                    binding,
                    every_s: every.as_secs(),
                })
                .collect(),
            next_tick_at: self.next_tick_at.lock().expect("not poisoned").clone(),
            ticks: *self.ticks.lock().expect("not poisoned"),
            caps: self.caps,
            capped: self.capped_goals(),
            coordinator: Coordinator {
                configured: coordinator.is_some(),
                program: coordinator.map(|launch| launch.describe()),
                in_flight: self.turns_in_flight(),
            },
            swarms,
        }
    }

    /// The entities a canvas draws: every entity the specification declares.
    ///
    /// Not a list kept here. A canvas that draws what the model declares gains a node kind when the
    /// model does, and a hand-kept list is the thing that would stop that being true.
    pub fn drawable_entities(&self) -> Vec<String> {
        self.spec
            .ir()
            .entities()
            .keys()
            .map(ToString::to_string)
            .collect()
    }

    /// What this system is, as the UI needs to know it.
    pub fn describe(&self) -> Description {
        let ir = self.spec.ir();
        Description {
            system: ir.system().to_string(),
            entities: ir
                .entities()
                .iter()
                .map(|(name, entity)| EntityShape {
                    name: name.to_string(),
                    identity: entity.identity.name.as_str().to_owned(),
                    states: entity
                        .lifecycle
                        .states
                        .iter()
                        .map(|state| state.as_str().to_owned())
                        .collect(),
                    // Which states are ends. A canvas draws a finished instance differently, and
                    // deriving that from a name would be guessing at what the lifecycle declares.
                    terminal: entity
                        .lifecycle
                        .terminal
                        .iter()
                        .map(|state| state.as_str().to_owned())
                        .collect(),
                    fields: entity
                        .fields
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                    // What this entity points at, and through which field. A canvas draws these as
                    // edges: the specification already says a goal references the swarm it belongs
                    // to, so nothing has to be told that twice.
                    relations: entity
                        .relations
                        .iter()
                        .map(|relation| RelationShape {
                            name: relation.name.clone(),
                            target: relation.target.name().to_string(),
                            via: relation.via.clone(),
                            owns: matches!(relation.kind, ess_domain::entity::RelationKind::Owns),
                        })
                        .collect(),
                })
                .collect(),
            commands: ir
                .commands()
                .iter()
                .map(|(name, command)| CommandShape {
                    name: name.to_string(),
                    input: command
                        .input
                        .iter()
                        .map(|field| InputShape {
                            name: field.name.as_str().to_owned(),
                            optional: field.type_ref.is_optional(),
                        })
                        .collect(),
                })
                .collect(),
            views: ir.views().keys().map(ToString::to_string).collect(),
        }
    }
}

/// The same path with something appended to its file name: `x.sqlite3` and `-wal` is
/// `x.sqlite3-wal`, which is not what `set_extension` would give.
fn with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Why a swarm was not removed.
#[derive(Debug)]
pub enum Removal {
    /// No handle and no directory: there is nothing here by that name.
    NotHere(String),
    /// The model holds a record, and it is not finished. The specification declares this refusal
    /// for every command that acts from a state the instance is not in; removal is not a command,
    /// but it is the same answer to the same question and inventing a second name for it would put
    /// two words in front of a client for one fact.
    StateConflict { slug: String, state: String },
    /// The files would not go.
    Undeletable { path: String, why: String },
}

impl std::fmt::Display for Removal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotHere(slug) => write!(f, "no swarm `{slug}`"),
            Self::StateConflict { slug, state } => write!(
                f,
                "`{slug}` is {state}, and only a swarm with no record or in a terminal state is removed"
            ),
            Self::Undeletable { path, why } => write!(f, "{path} could not be removed: {why}"),
        }
    }
}

impl std::error::Error for Removal {}

/// One goal the loop has stopped asking about.
#[derive(Clone, Debug, Serialize)]
pub struct CappedGoal {
    pub swarm: String,
    pub goal: String,
    pub turns: u64,
    pub spent_usd: Option<f64>,
    pub reached: Reached,
    pub why: String,
}

/// Where the runtime is.
#[derive(Debug, Serialize)]
pub struct Status {
    pub system: String,
    pub spec_files: usize,
    pub started_at: String,
    pub now: String,
    pub periodic: Vec<Periodic>,
    pub next_tick_at: Option<String>,
    pub ticks: u64,
    /// What one goal may use up before the loop stops asking.
    pub caps: Caps,
    /// Every goal the loop has stopped asking about, with why.
    pub capped: Vec<CappedGoal>,
    pub coordinator: Coordinator,
    pub swarms: Vec<SwarmStatus>,
}

#[derive(Debug, Serialize)]
pub struct Periodic {
    pub binding: String,
    pub every_s: u64,
}

#[derive(Debug, Serialize)]
pub struct Coordinator {
    pub configured: bool,
    pub program: Option<String>,
    /// Turns running right now, across every swarm.
    pub in_flight: usize,
}

#[derive(Debug, Serialize)]
pub struct SwarmStatus {
    #[serde(flatten)]
    pub summary: Summary,
    /// Events in the log.
    pub events: u64,
}

/// The shape of the system, for a UI that would rather read it than be told it.
#[derive(Debug, Serialize)]
pub struct Description {
    pub system: String,
    pub entities: Vec<EntityShape>,
    pub commands: Vec<CommandShape>,
    pub views: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EntityShape {
    pub name: String,
    pub identity: String,
    pub states: Vec<String>,
    pub terminal: Vec<String>,
    pub fields: Vec<String>,
    pub relations: Vec<RelationShape>,
}

/// One edge the specification declares between two entities.
#[derive(Debug, Serialize)]
pub struct RelationShape {
    pub name: String,
    pub target: String,
    /// The field holding the other instance's identity.
    pub via: String,
    /// `owns` rather than `references`: the target does not outlive this one.
    pub owns: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandShape {
    pub name: String,
    pub input: Vec<InputShape>,
}

#[derive(Debug, Serialize)]
pub struct InputShape {
    pub name: String,
    pub optional: bool,
}

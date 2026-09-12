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

/// The whole server.
pub struct Server {
    spec: Arc<Spec>,
    root: PathBuf,
    swarms: RwLock<BTreeMap<String, Arc<Swarm>>>,
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
            started_at: now(),
            next_tick_at: Mutex::new(None),
            ticks: Mutex::new(0),
            in_flight: Mutex::new(HashSet::new()),
            capped: Mutex::new(BTreeMap::new()),
            caps: Caps::configured(),
        })
    }

    /// Opens a swarm, or returns the one already open.
    pub async fn open(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
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

    /// One open swarm.
    pub async fn get(&self, slug: &str) -> Result<Arc<Swarm>, Refused> {
        self.swarms
            .read()
            .await
            .get(slug)
            .map(Arc::clone)
            .ok_or_else(|| Refused::View(format!("no swarm `{slug}`")))
    }

    /// Every open swarm, by slug.
    pub async fn slugs(&self) -> Vec<String> {
        self.swarms.read().await.keys().cloned().collect()
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

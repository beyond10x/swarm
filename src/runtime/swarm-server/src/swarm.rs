//! One running swarm: its log, its world, and the broadcast that tells the UI something changed.
//!
//! The world is held in memory and written through to the log, rather than re-folded on every read.
//! Both are true at once because the fold is deterministic — the in-memory copy is a cache of the
//! replay, and [`Swarm::reload`] throws it away and rebuilds when that needs proving.
//!
//! Every mutation goes through [`Swarm::issue`], which is the only place a command is applied, the
//! only place the log is written, and the only place the pump runs. A second path into the world
//! would be a second answer to what happened.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value as Json};
use tokio::sync::{Mutex, broadcast};

use ess_runtime::apply::Emitted;
use ess_runtime::{Spec, Store, World, apply, pump, route::Occurrence, route::tick, view};

/// How many events a slow reader may fall behind before it is dropped and told to resync.
const BACKLOG: usize = 256;

/// Something that happened, as the UI hears about it.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    /// A command was applied and these events were appended.
    Applied {
        command: String,
        outcome: String,
        events: Vec<Record>,
    },
    /// A binding carried an event to another command.
    Routed { binding: String, command: String },
    /// The loop turned.
    Ticked { binding: String, command: String },
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
        })
    }

    /// Which swarm this is.
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// A reader of everything that happens from now on.
    pub fn watch(&self) -> broadcast::Receiver<Change> {
        self.changes.subscribe()
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
        let _ = self.changes.send(Change::Applied {
            command: command.to_owned(),
            outcome: done.outcome.clone(),
            events: appended.clone(),
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
                let _ = self.changes.send(Change::Routed {
                    binding: routed.binding.clone(),
                    command: routed.command.clone(),
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
            let _ = self.changes.send(Change::Ticked {
                binding: turned.binding.clone(),
                command: turned.command.clone(),
            });
        }
        Ok(true)
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

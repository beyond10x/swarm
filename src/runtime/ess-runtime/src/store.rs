//! Where the world is kept: an append-only log, and state as a fold over it.
//!
//! Until now [`World`] was a map somebody handed in. Here it is derived: a command's events are
//! appended, and an instance is what replaying its stream produces. The log is the record and
//! everything else is a projection — so a projection can be dropped and rebuilt, and a state that
//! disagrees with the log is wrong by definition rather than by argument.
//!
//! The shape, one swarm at a time:
//!
//! ```text
//!   data/swarms/<slug>/eventlog.sqlite3
//!     tenant      = <slug>            one swarm cannot read another's history
//!     stream_type = swarm.goal.Goal   the entity
//!     stream_id   = <identity>        the instance
//! ```
//!
//! One database per swarm rather than one shared: `SqliteEventStore::open` is path-scoped with no
//! global state, so this is ordinary usage. It also means a swarm can be archived, copied or deleted
//! by moving one directory.
//!
//! **One writer process per file.** No `busy_timeout` is set anywhere in the store, so a second
//! process contending for the same database gets `SQLITE_BUSY` immediately rather than waiting. A
//! CLI and a daemon that both need to write must go through the daemon.
//!
//! What this replays is what the specification guarantees is replayable. `bin/check-sets-are-emitted.py`
//! refuses a spec in which an outcome writes a field no event carries, which is the rule that makes
//! the fold below total rather than best-effort.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value as Json};

use ess_compiler::EssIr;
use ess_compiler::ir::{ResolvedEffect, ResolvedInstance};
use eventlog_core::{
    CommandMeta, EventStore, Expected, NewEvent, RecordedEvent, StreamId, TenantId, request_hash,
};
use eventlog_sqlite::SqliteEventStore;

use crate::apply::{Applied, Emitted, World};
use crate::instance::Instance;

/// Every event this runtime writes carries this schema version.
///
/// One number for the whole log, because an ESS event's shape is decided by the specification and
/// the specification is compiled at boot: a reader that wants to know what an event meant asks the
/// specification it was written under, not a per-event integer. When that stops being true — when a
/// spec change has to be readable against old events — this is where the upcast goes.
pub const SCHEMA_VERSION: u32 = 1;

/// How many events one read pulls back while folding a stream.
const PAGE: usize = 500;

/// Why the store refused.
#[derive(Debug)]
pub enum StoreError {
    /// The log itself refused: a conflict, an invalid identifier, an IO failure.
    Log(eventlog_core::EventLogError),
    /// A swarm slug that is not a usable tenant.
    BadSlug { slug: String, why: String },
    /// The directory could not be made.
    Unusable { path: String, why: std::io::Error },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log(why) => write!(f, "{why}"),
            Self::BadSlug { slug, why } => write!(f, "`{slug}` is not a usable swarm slug: {why}"),
            Self::Unusable { path, why } => write!(f, "{path}: {why}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<eventlog_core::EventLogError> for StoreError {
    fn from(why: eventlog_core::EventLogError) -> Self {
        Self::Log(why)
    }
}

/// One swarm's log.
pub struct Store {
    events: SqliteEventStore,
    tenant: TenantId,
    slug: String,
}

impl Store {
    /// Opens, or creates, the log for one swarm at `data/swarms/<slug>/`.
    pub async fn open(root: impl AsRef<Path>, slug: &str) -> Result<Self, StoreError> {
        let directory = root.as_ref().join("swarms").join(slug);
        std::fs::create_dir_all(&directory).map_err(|why| StoreError::Unusable {
            path: directory.display().to_string(),
            why,
        })?;

        let database = directory.join("eventlog.sqlite3");
        let events = SqliteEventStore::open(&database.display().to_string(), "swarm").await?;
        let tenant = TenantId::new(slug).map_err(|why| StoreError::BadSlug {
            slug: slug.to_owned(),
            why: why.to_string(),
        })?;

        Ok(Self {
            events,
            tenant,
            slug: slug.to_owned(),
        })
    }

    /// An in-memory log, for a test or a dry run. Nothing is written to disk.
    pub async fn in_memory(slug: &str) -> Result<Self, StoreError> {
        let events = SqliteEventStore::in_memory("swarm").await?;
        let tenant = TenantId::new(slug).map_err(|why| StoreError::BadSlug {
            slug: slug.to_owned(),
            why: why.to_string(),
        })?;
        Ok(Self {
            events,
            tenant,
            slug: slug.to_owned(),
        })
    }

    /// Which swarm this is.
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Appends what a command produced, to the stream of the instance it acted on.
    ///
    /// `request` is an idempotency key, and it identifies one REQUEST rather than one command: two
    /// turns of the loop are two requests. Committing the same key twice with the same events is a
    /// retry and appends nothing; committing it with different events is refused, because accepting
    /// that would turn a caller's bookkeeping error into a lost write.
    ///
    /// The key is scoped to the instance's stream, not to the swarm. Two instances are two streams,
    /// so one key spent on each is two requests and not a clash — a caller that assumed swarm-wide
    /// keys would refuse honest writes, and one that assumed the opposite would accept dishonest
    /// ones. Cross-stream idempotency exists in the log as a `Claim`, and is not used here.
    ///
    /// `Expected::Any` rather than a version check, because the interpreter has already decided the
    /// outcome against the state it read: a concurrency guard belongs where that read and this write
    /// are one operation, and this runtime has one writer by construction. Where that changes, the
    /// expected version is what changes with it.
    pub async fn commit(
        &self,
        applied: &Applied,
        actor: Option<&str>,
        request: &str,
    ) -> Result<(), StoreError> {
        let Some(instance) = applied.instance.as_ref() else {
            return Ok(());
        };
        if applied.events.is_empty() {
            return Ok(());
        }

        let stream = StreamId::new(
            self.tenant.clone(),
            instance.entity.clone(),
            instance.id.clone(),
        )?;

        let events = applied
            .events
            .iter()
            .map(|emitted| {
                NewEvent::new(
                    emitted.name.clone(),
                    SCHEMA_VERSION,
                    Json::Object(emitted.fields.clone()),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let body = Json::Object(
            applied
                .events
                .iter()
                .map(|e| (e.name.clone(), Json::Object(e.fields.clone())))
                .collect(),
        );

        let meta = CommandMeta {
            idempotency_key: request.to_owned(),
            request_hash: request_hash(&body)?,
            subject: actor.unwrap_or("system").to_owned(),
            actor: actor.unwrap_or("system").to_owned(),
            request_id: request.to_owned(),
            trace_id: request.to_owned(),
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::now_utc(),
            claim: None,
        };

        self.events
            .append(&stream, Expected::Any, &events, &meta)
            .await?;
        Ok(())
    }

    /// Rebuilds one instance by replaying its stream.
    pub async fn rebuild(
        &self,
        ir: &EssIr,
        entity: &str,
        id: &str,
    ) -> Result<Option<Instance>, StoreError> {
        let stream = StreamId::new(self.tenant.clone(), entity.to_owned(), id.to_owned())?;
        let mut recorded = Vec::new();
        let mut after = 0;

        loop {
            let slice = self.events.read_stream(&stream, after, PAGE).await?;
            let end = slice.end_of_stream;
            after = slice.next_version.saturating_sub(1);
            recorded.extend(slice.events);
            if end {
                break;
            }
        }

        Ok(fold(ir, entity, id, &recorded))
    }

    /// Rebuilds the whole world by replaying the swarm's feed.
    ///
    /// The feed is in commit order across every stream, so one pass groups it by instance and folds
    /// each — which is also the order the events actually happened in, and therefore the only order
    /// that reconstructs what the swarm did rather than what its streams contain.
    pub async fn world(&self, ir: &EssIr) -> Result<World, StoreError> {
        let mut per_instance: BTreeMap<(String, String), Vec<RecordedEvent>> = BTreeMap::new();
        let mut after = 0;

        loop {
            let page = self.events.read_feed(&self.tenant, after, PAGE).await?;
            let more = page.has_more;
            after = page.next_position;
            for event in page.events {
                per_instance
                    .entry((event.stream_type.clone(), event.stream_id.clone()))
                    .or_default()
                    .push(event);
            }
            if !more {
                break;
            }
        }

        let mut world = World::new();
        for ((entity, id), events) in per_instance {
            if let Some(instance) = fold(ir, &entity, &id, &events) {
                world.insert((entity, id), instance);
            }
        }
        Ok(world)
    }
}

/// Replays one instance's events into the instance they describe.
///
/// The specification decides every step. Which outcome emitted an event says whether it created,
/// moved or merely updated the instance; what that outcome `sets:` says which of the event's fields
/// land on it. A field the event carries that no outcome writes is not copied — an event may carry
/// a value for a reader's benefit without it being state.
fn fold(ir: &EssIr, entity: &str, id: &str, recorded: &[RecordedEvent]) -> Option<Instance> {
    let (_, declared) = ir
        .entities()
        .iter()
        .find(|(name, _)| name.to_string() == entity)?;

    let mut instance: Option<Instance> = None;

    for event in recorded {
        // A redacted event's body is gone; its having happened is not. Skipping it keeps the fold
        // total rather than failing the whole stream for one erased payload.
        if event.redacted_at.is_some() {
            continue;
        }
        let Some(fields) = event.data.as_object() else {
            continue;
        };

        for command in ir.commands().values() {
            for outcome in &command.outcomes {
                if !outcome
                    .emits
                    .iter()
                    .any(|handle| handle.name().to_string() == event.name)
                {
                    continue;
                }
                let Some(subject) = outcome.subject.as_ref() else {
                    continue;
                };
                if subject.entity.name().to_string() != entity {
                    continue;
                }

                match &subject.effect {
                    ResolvedEffect::Creates => {
                        // Only the event the specification names as publishing the identity creates
                        // it; another outcome that happens to emit the same event does not.
                        if matches!(&subject.instance, ResolvedInstance::Observed { event: handle, .. }
                            if handle.name().to_string() == event.name)
                        {
                            instance = Some(Instance::created(
                                entity.to_owned(),
                                declared.identity.name.as_str(),
                                id.to_owned(),
                                declared.lifecycle.initial.as_str(),
                                declared.fields.iter().map(|f| f.name.as_str().to_owned()),
                            ));
                        }
                    }
                    ResolvedEffect::Moves { transition } => {
                        if let Some(existing) = instance.as_mut() {
                            existing.state = transition.to.as_str().to_owned();
                        }
                    }
                    ResolvedEffect::Updates => {}
                }

                if let Some(existing) = instance.as_mut() {
                    for written in &outcome.sets {
                        if let Some(value) = fields.get(&written.target) {
                            let _ = existing.set(&written.target, value.clone());
                        }
                    }
                    existing.revision += 1;
                }
            }
        }
    }

    instance
}

/// The events of one command, as they would be appended. Useful to a caller that wants to see the
/// record before committing it.
pub fn as_records(events: &[Emitted]) -> Vec<(String, Map<String, Json>)> {
    events
        .iter()
        .map(|event| (event.name.clone(), event.fields.clone()))
        .collect()
}

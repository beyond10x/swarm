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

use serde::Serialize;
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

/// Who issued a command, as the log records it.
///
/// Not the same question as the `actor`, and the difference is the whole of
/// `story:an-event-cannot-say-which-agent-acted`. The actor is the specification's: `apply`'s
/// `permitted` matches it against the declared actor TYPES, so every member of a swarm that may
/// emit a note records the same word `swarm.agent.Worker`, and two members' notes were the same
/// record twice. The issuer is the HOST's: which process, which agent instance, which pair of
/// hands. The model does not know about sockets any more than it knows what time it is, so this
/// arrives at the edge exactly as a timestamp does.
///
/// It is recorded in the envelope's `subject` column. eventlog calls that "the person the work is
/// for", which is a stretch: there is no person here, and what wants recording is who did it. It
/// is used because the alternative was what was there before — `subject` holding a second copy of
/// `actor`, two columns carrying one fact — and a duplicate records nothing at all. `subject` is
/// an `validate_identity` column, which is the right shape for an opaque agent slug.
///
/// **A row written before 2026-09-13 carries the actor type, or `system`, in that column.** No
/// value in the eleven logs on disk when this was written collides with anything
/// [`Issuer::label`] produces, and no actor type the specification declares does either — which
/// is what lets a reader tell an old row from a new one and say so.
///
/// That is a fact about those logs and about this specification, **not a property of the code**,
/// and the difference matters to anyone relying on it. `actor` is a caller-supplied free string
/// and `apply`'s `permitted` skips the check when no declared actor matches, so a caller that
/// issued a command with the literal actor `runtime` against an old build left a row a reader
/// will now read as the runtime's. Nothing can be done about that row; what can be done is not
/// claim it is impossible.
///
/// `examples/two-agents/check-two-agents.py` is the reader that tells them apart. There is no
/// parser on this side because nothing in Rust reads the column back, and a second parser with no
/// caller is a second vocabulary waiting to drift from this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Issuer {
    /// The runtime issuing to itself: a binding, the pump, the trigger, a turn's own bookkeeping.
    Runtime,
    /// Something at the HTTP surface that said nothing about who it is — a `curl`, the canvas, a
    /// CLI with no settings file. Called an operator because that is who it is in practice, and
    /// because "no operator touched this swarm" is the claim it makes checkable.
    Operator,
    /// One named agent instance, as it claimed at the door. A claim, not a proof: authentication
    /// is a different problem and is worth nothing without this one.
    Agent(String),
    /// An agent claimed a name the envelope's identity column cannot hold, kept as the claim it
    /// made so the mark can be derived from it.
    ///
    /// `swarm.agent.Spawn`'s `agent_id` is a caller-supplied `String` and `StreamId` accepts a
    /// space or an `@` in one; `validate_identity` does not, because an append-only log outlives
    /// every request to erase an address. So a member called `two words` can exist and cannot be
    /// named here. Recording this rather than refusing keeps that member working; recording it
    /// rather than `Operator` or `Runtime` keeps the record true.
    ///
    /// **It is per name.** The first version of this had one label for all of them, and two such
    /// members of one actor type then recorded the same envelope — which is acceptance clause 1
    /// negated, and is how the adversary of correction round 1 negated it. The label carries a
    /// digest of the claim, so two are two and one stays one.
    UnnameableAgent(String),
}

/// What [`Issuer::Agent`] is spelled with, and what a reader splits on.
const AGENT: &str = "agent:";

/// What [`Issuer::UnnameableAgent`] is spelled with, before its digest.
///
/// **An agent whose id begins with `?` shares this prefix, and no character removes that.** The
/// label is written into an identity column, so it must itself pass
/// `eventlog_core::validate_identity` — which refuses a space and an `@`, and accepts every other
/// ascii-graphic byte. That is exactly the set a nameable slug may begin with, so there is no
/// character that can start the one and not the other. The adversary of correction round 2
/// suggested `agent: ` for this and it cannot be used: the append would be refused, which is the
/// failure [`Issuer::UnnameableAgent`] exists to avoid.
///
/// What makes the collision harmless is that **no reader decides by the prefix**. An agent is what
/// a `swarm.agent.AgentSpawned` says it is, and `check-two-agents.py`'s `agent_named_by` asks the
/// log: a slug it spawned is that agent whatever it starts with, and a string it did not spawn is
/// not an agent whether it is a mark or an invention.
///
/// The first version of this comment accepted the collision and costed it at "one reader one wrong
/// attribution in a case nothing has ever produced". The measured cost was clause 2 NOT MET and
/// the demonstration exiting 1, on a member that had done everything the clause asks. The estimate
/// was the defect, not the collision.
const UNNAMEABLE: &str = "agent:?";

/// How many hex characters of the digest go in the label. 48 bits, which is enough to keep the
/// members of one swarm apart and short enough to read.
const MARK: usize = 12;

/// A stable mark for a name the log cannot write: FNV-1a, 64-bit, hex.
///
/// Written out rather than taken from `std::collections::hash_map::DefaultHasher`, whose algorithm
/// std explicitly does not specify and may change between releases. A digest that moved under a
/// toolchain upgrade would make one member's earlier events and its later ones read as two
/// members — the same defect the digest exists to fix, wearing the opposite sign. A log outlives a
/// compiler.
fn mark(claim: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in claim.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")[..MARK].to_owned()
}

impl Issuer {
    /// The claim a door received, as an issuer. No name is an operator, because the doors are the
    /// outside and the runtime does not reach itself through one.
    #[must_use]
    pub fn claimed(agent: Option<&str>) -> Self {
        match agent {
            Some(agent) if !agent.is_empty() => Self::agent(agent),
            _ => Self::Operator,
        }
    }

    /// One named agent, or the marked form when the log cannot hold that name.
    #[must_use]
    pub fn agent(name: &str) -> Self {
        let said = format!("{AGENT}{name}");
        if eventlog_core::validate_identity("issuer", &said).is_ok() {
            Self::Agent(name.to_owned())
        } else {
            Self::UnnameableAgent(name.to_owned())
        }
    }

    /// What goes in the column.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Runtime => "runtime".to_owned(),
            Self::Operator => "operator".to_owned(),
            Self::Agent(name) => format!("{AGENT}{name}"),
            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),
        }
    }
}

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
    /// `issuer` is taken rather than defaulted, and that is deliberate: the defect this whole
    /// method's envelope was fixed for was an identity that existed at the call site and was
    /// discarded one line before the wire (`swarm-cli`'s `actor_for(command, _agent)`). A
    /// parameter with a default here would be the same discard in a different place.
    pub async fn commit(
        &self,
        applied: &Applied,
        actor: Option<&str>,
        issuer: &Issuer,
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
            // Two columns, two facts. `actor` is the specification's actor TYPE, which is what
            // `permitted` checked; `subject` is which instance issued it. They used to be one
            // string written twice, and that is what made an `AssignmentTaken` naming a member
            // indistinguishable from the coordinator or a `curl` issuing it on that member's
            // behalf.
            subject: issuer.label(),
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

    /// The most recent `limit` events across every stream, oldest first.
    ///
    /// The whole feed is read and the tail kept, because the log has no "read backwards" and a
    /// swarm's log is small enough that reading it is cheaper than being wrong about a cursor.
    /// Returned in commit order, which is the order things happened in.
    pub async fn history(&self, limit: usize) -> Result<Vec<Recorded>, StoreError> {
        let mut all = Vec::new();
        let mut after = 0;
        loop {
            let page = self.events.read_feed(&self.tenant, after, PAGE).await?;
            let more = page.has_more;
            after = page.next_position;
            all.extend(page.events.iter().map(Recorded::from));
            if !more {
                break;
            }
        }
        let keep = all.len().saturating_sub(limit);
        Ok(all.split_off(keep))
    }

    /// How many events the log holds.
    pub async fn count(&self) -> Result<u64, StoreError> {
        let mut total = 0u64;
        let mut after = 0;
        loop {
            let page = self.events.read_feed(&self.tenant, after, PAGE).await?;
            total += page.events.len() as u64;
            after = page.next_position;
            if !page.has_more {
                break;
            }
        }
        Ok(total)
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

/// One event as history holds it, flattened for a reader.
///
/// The log's own record carries more (redaction, schema version, causation); this is what a person
/// watching a swarm wants to see, and nothing that would need the log's types to read.
#[derive(Clone, Debug, Serialize)]
pub struct Recorded {
    /// Position in the swarm's feed.
    pub seq: u64,
    /// When it was recorded, RFC 3339.
    pub at: String,
    /// The entity, and the instance.
    pub entity: String,
    pub id: String,
    /// Position in the instance's stream, from 1.
    pub version: u64,
    pub name: String,
    /// The specification's actor type, which is what `permitted` matched.
    pub actor: String,
    /// Which instance issued the command — see [`Issuer`].
    ///
    /// A row written before 2026-09-13 carries the actor type here instead, because the store
    /// wrote the actor into both columns. It is handed over verbatim rather than parsed, because
    /// flattening an old row into "nothing said" would destroy the evidence that says which kind
    /// of row it is — and that evidence is what a reader needs in order to say it is coping.
    pub issuer: String,
    pub request: String,
    pub fields: Json,
}

impl From<&RecordedEvent> for Recorded {
    fn from(event: &RecordedEvent) -> Self {
        Self {
            seq: event.global_seq,
            at: event
                .recorded_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            entity: event.stream_type.clone(),
            id: event.stream_id.clone(),
            version: event.version,
            name: event.name.clone(),
            actor: event.actor.clone(),
            issuer: event.subject.clone(),
            request: event.request_id.clone(),
            fields: if event.is_redacted() {
                Json::Null
            } else {
                event.data.clone()
            },
        }
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

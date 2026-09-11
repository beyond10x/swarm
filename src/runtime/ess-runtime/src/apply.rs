//! Applying one command to the world.
//!
//! This is the interpreter proper. Everything it does is determined by the IR; where the
//! specification is silent, it leaves the field undetermined rather than inventing a value.
//!
//! The order is fixed, and each step is a thing the specification already decided:
//!
//!   1. select the outcome — each `when:` predicate against the invocation's input, in declaration
//!      order, then the one `otherwise`, and `wrong_state` when the subject rests in a state that
//!      no move of this command starts from;
//!   2. resolve the subject — supplied in an input field, or, for `creates:`, minted here and
//!      published in the event field the specification names;
//!   3. apply the effect — a new instance at its lifecycle's initial state, a move along a declared
//!      transition, or neither;
//!   4. write what the outcome `sets:`;
//!   5. emit each event, with its payload mapped field by field.
//!
//! What it does NOT do is decide anything the model left open. A literal in a payload is text
//! because ESS says a literal in a binding is text; a field no `sets:` writes stays undetermined;
//! and an outcome that refuses moves nothing.

use std::collections::BTreeMap;

use serde_json::{Map, Value as Json};

use ess_compiler::EssIr;
use ess_compiler::ir::{
    ResolvedCommand, ResolvedCondition, ResolvedEffect, ResolvedInstance, ResolvedOutcome,
    ResolvedPayloadValue,
};
use ess_primitives::predicate::Truth;

use crate::facts::Input;
use crate::instance::Instance;

/// One event the interpreter decided to emit.
#[derive(Clone, Debug, PartialEq)]
pub struct Emitted {
    /// The qualified event name.
    pub name: String,
    /// Its payload, as the outcome's mapping produced it.
    pub fields: Map<String, Json>,
}

/// What applying a command did.
#[derive(Clone, Debug)]
pub struct Applied {
    /// Which outcome was taken.
    pub outcome: String,
    /// The instance as it now stands, when the outcome acted on one.
    pub instance: Option<Instance>,
    /// The events to append, in the order the outcome declares them.
    pub events: Vec<Emitted>,
    /// The error this outcome reports, when it refuses.
    pub error: Option<String>,
}

/// Why a command could not be applied.
///
/// These are all failures of the CALLER or the store, never of the specification: a command nobody
/// declares, an actor not permitted it, an identity nothing supplied. A specification that does not
/// resolve never reaches this module.
#[derive(Debug, Clone, PartialEq)]
pub enum ApplyError {
    /// No such command in this specification.
    NoSuchCommand(String),
    /// No such actor, or the actor may not invoke this command.
    NotPermitted { actor: String, command: String },
    /// The command's input is missing a field the specification declares.
    MissingInput { command: String, field: String },
    /// The instance the command names does not exist.
    NoSuchInstance { entity: String, id: String },
    /// Every outcome was conditional and none held. `ess-domain` refuses such a command, so this
    /// means the IR and this interpreter disagree — it is a bug here, reported rather than guessed
    /// around.
    NoOutcome(String),
    /// A `creates:` outcome whose identity the specification publishes in an event, but the caller
    /// supplied no identity to mint.
    NoIdentity(String),
    /// A write to a field the entity does not declare.
    UnknownField { entity: String, field: String },
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchCommand(name) => write!(f, "no command `{name}`"),
            Self::NotPermitted { actor, command } => {
                write!(f, "`{actor}` may not invoke `{command}`")
            }
            Self::MissingInput { command, field } => {
                write!(f, "`{command}` requires input field `{field}`")
            }
            Self::NoSuchInstance { entity, id } => write!(f, "no `{entity}` with id `{id}`"),
            Self::NoOutcome(name) => write!(
                f,
                "`{name}` has no outcome for this input, which a resolved specification forbids"
            ),
            Self::NoIdentity(name) => write!(f, "`{name}` creates an instance with no identity"),
            Self::UnknownField { entity, field } => {
                write!(f, "`{entity}` declares no field `{field}`")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

/// The world the interpreter reads and writes: instances, keyed by entity and identity.
pub type World = BTreeMap<(String, String), Instance>;

/// Applies one command, and says what it did.
///
/// `id` is the identity to give a newly created instance. It is supplied rather than minted here
/// because the kernel decides nothing the outside world knows — the same reason timestamps are
/// command inputs.
pub fn apply(
    ir: &EssIr,
    world: &World,
    actor: Option<&str>,
    command: &str,
    input: &Map<String, Json>,
    id: Option<&str>,
) -> Result<Applied, ApplyError> {
    let (_, declared) = ir
        .commands()
        .iter()
        .find(|(name, _)| name.to_string() == command)
        .ok_or_else(|| ApplyError::NoSuchCommand(command.to_owned()))?;

    if let Some(actor) = actor {
        permitted(ir, actor, command)?;
    }
    require_declared_input(declared, command, input)?;

    let subject = subject_state(ir, world, declared, input);
    let outcome = select(declared, input, subject.as_ref())
        .ok_or_else(|| ApplyError::NoOutcome(command.to_owned()))?;

    let mut events = emit(outcome, input);
    let instance = act(ir, world, outcome, input, id, &events)?;

    // A `creates:` outcome's identity is minted here and PUBLISHED in the event field the
    // specification names — that is what `ResolvedInstance::Observed` says, and an event that went
    // out without it would be a creation nobody could attribute. The payload itself is declared
    // `{generated: true}`, which is the model saying the implementation owns the value; owning it
    // means writing it.
    if let Some(created) = instance.as_ref()
        && let Some(subject) = outcome.subject.as_ref()
        && matches!(subject.effect, ResolvedEffect::Creates)
        && let ResolvedInstance::Observed { event, field } = &subject.instance
        && let Some(published) = events
            .iter_mut()
            .find(|emitted| emitted.name == event.name().to_string())
    {
        published.fields.insert(
            field.name.as_str().to_owned(),
            Json::String(created.id.clone()),
        );
    }

    Ok(Applied {
        outcome: outcome.name.as_str().to_owned(),
        instance,
        events,
        error: outcome
            .error
            .as_ref()
            .map(|handle| handle.name().to_string()),
    })
}

/// Refuses an actor the specification does not permit this command.
fn permitted(ir: &EssIr, actor: &str, command: &str) -> Result<(), ApplyError> {
    let found = ir
        .actors()
        .iter()
        .find(|(name, _)| name.to_string() == actor)
        .map(|(_, declared)| declared);

    let allowed = found.is_some_and(|declared| {
        declared
            .may
            .iter()
            .any(|handle| handle.name().to_string() == command)
    });

    if allowed {
        Ok(())
    } else {
        Err(ApplyError::NotPermitted {
            actor: actor.to_owned(),
            command: command.to_owned(),
        })
    }
}

/// Refuses an invocation missing a field the command declares as required.
fn require_declared_input(
    declared: &ResolvedCommand,
    command: &str,
    input: &Map<String, Json>,
) -> Result<(), ApplyError> {
    for field in &declared.input {
        if field.type_ref.is_optional() {
            continue;
        }
        if !input.contains_key(field.name.as_str()) {
            return Err(ApplyError::MissingInput {
                command: command.to_owned(),
                field: field.name.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

/// The state the subject rests in, when this command acts on an existing instance.
fn subject_state(
    ir: &EssIr,
    world: &World,
    declared: &ResolvedCommand,
    input: &Map<String, Json>,
) -> Option<String> {
    let _ = ir;
    for outcome in &declared.outcomes {
        // A wrong-state branch names no subject, so `continue` and not `?`: one branch without a
        // subject must not end the search for the branch that has one.
        let Some(subject) = outcome.subject.as_ref() else {
            continue;
        };
        let ResolvedInstance::Supplied { field } = &subject.instance else {
            continue;
        };
        let Some(id) = input.get(field.name.as_str()).and_then(Json::as_str) else {
            continue;
        };
        let entity = subject.entity.name().to_string();
        return world.get(&(entity, id.to_owned())).map(|i| i.state.clone());
    }
    None
}

/// Which outcome this invocation takes.
///
/// Declaration order decides, which is what the specification's own ordering means: a later guard
/// never overrides an earlier one that held. Conditional outcomes are tried first, the single
/// `otherwise` is the fallback, and a subject resting where no move of this command starts takes
/// the refusing branch whatever its guards would have said.
fn select<'ir>(
    declared: &'ir ResolvedCommand,
    input: &Map<String, Json>,
    resting_in: Option<&String>,
) -> Option<&'ir ResolvedOutcome> {
    let facts = Input::new(input);

    if let Some(state) = resting_in
        && let Some(refusal) = declared
            .outcomes
            .iter()
            .find(|o| matches!(o.condition, ResolvedCondition::WrongState))
        && !starts_from(declared, state)
    {
        return Some(refusal);
    }

    let mut fallback = None;
    for outcome in &declared.outcomes {
        match &outcome.condition {
            ResolvedCondition::When { predicate } => {
                if predicate.evaluate(&facts) == Truth::True {
                    return Some(outcome);
                }
            }
            // The subject must hold this state, and satisfy the extra input guard when one is
            // declared. Unlike `When`, this reads the world as well as the invocation.
            ResolvedCondition::SubjectState { state, predicate } => {
                let held = resting_in.is_some_and(|resting| resting == state.as_str());
                let guard = predicate
                    .as_ref()
                    .is_none_or(|p| p.evaluate(&facts) == Truth::True);
                if held && guard {
                    return Some(outcome);
                }
            }
            ResolvedCondition::Otherwise => fallback = fallback.or(Some(outcome)),
            // Decided outside the input; never selected by evaluating one.
            ResolvedCondition::External { .. } | ResolvedCondition::WrongState => {}
        }
    }
    fallback
}

/// Whether any move of this command starts from `state`.
fn starts_from(declared: &ResolvedCommand, state: &str) -> bool {
    declared.outcomes.iter().any(|outcome| {
        outcome
            .subject
            .as_ref()
            .and_then(|subject| subject.effect.transition())
            .is_some_and(|transition| transition.from.iter().any(|from| from.as_str() == state))
    })
}

/// The events this outcome emits, with each payload field mapped.
fn emit(outcome: &ResolvedOutcome, input: &Map<String, Json>) -> Vec<Emitted> {
    outcome
        .emits
        .iter()
        .map(|handle| {
            let name = handle.name().to_string();
            let mapped = outcome
                .payload
                .iter()
                .find(|payload| payload.event.name().to_string() == name);

            let mut fields = Map::new();
            for field in mapped.map(|p| p.fields.as_slice()).unwrap_or_default() {
                if let Some(value) = read(&field.value, input) {
                    fields.insert(field.target.clone(), value);
                }
            }
            Emitted { name, fields }
        })
        .collect()
}

/// One payload or `sets:` value, when the specification determines it.
///
/// `None` means the model did not decide this field, and the caller must leave it undetermined
/// rather than substitute anything. Two of the four cases are exactly that: `ResponseField` reads
/// the implementation's actual response and `Generated` is explicit implementation ownership —
/// both are the specification saying "not mine", and an interpreter that invented a value for
/// either would be manufacturing evidence.
///
/// A literal is text. The IR carries `Literal { value: String }` and nothing types it further, so
/// parsing `"7"` into a number would be this crate deciding something the specification did not.
fn read(value: &ResolvedPayloadValue, input: &Map<String, Json>) -> Option<Json> {
    match value {
        ResolvedPayloadValue::InputField { field, .. } => input.get(field).cloned(),
        ResolvedPayloadValue::Literal { value } => Some(Json::String(value.clone())),
        ResolvedPayloadValue::ResponseField { .. } | ResolvedPayloadValue::Generated => None,
    }
}

/// Creates, moves or updates the subject, and writes what the outcome sets.
fn act(
    ir: &EssIr,
    world: &World,
    outcome: &ResolvedOutcome,
    input: &Map<String, Json>,
    id: Option<&str>,
    events: &[Emitted],
) -> Result<Option<Instance>, ApplyError> {
    let Some(subject) = outcome.subject.as_ref() else {
        return Ok(None);
    };
    let entity_name = subject.entity.name().to_string();
    let entity = ir
        .entities()
        .iter()
        .find(|(name, _)| name.to_string() == entity_name)
        .map(|(_, declared)| declared)
        .ok_or_else(|| ApplyError::NoSuchInstance {
            entity: entity_name.clone(),
            id: String::new(),
        })?;

    let mut instance = match &subject.effect {
        ResolvedEffect::Creates => {
            let id = id
                .map(ToOwned::to_owned)
                .or_else(|| {
                    // The specification names the event field that publishes it; if the caller
                    // already put it there, that is the identity.
                    let ResolvedInstance::Observed { event, field } = &subject.instance else {
                        return None;
                    };
                    events
                        .iter()
                        .find(|e| e.name == event.name().to_string())
                        .and_then(|e| e.fields.get(field.name.as_str()))
                        .and_then(|v| v.as_str().map(ToOwned::to_owned))
                })
                .ok_or_else(|| ApplyError::NoIdentity(entity_name.clone()))?;

            Instance::created(
                entity_name.clone(),
                entity.identity.name.as_str(),
                id,
                entity.lifecycle.initial.as_str(),
                entity
                    .fields
                    .iter()
                    .map(|field| field.name.as_str().to_owned()),
            )
        }
        ResolvedEffect::Moves { .. } | ResolvedEffect::Updates => {
            let ResolvedInstance::Supplied { field } = &subject.instance else {
                return Err(ApplyError::NoIdentity(entity_name.clone()));
            };
            let id = input
                .get(field.name.as_str())
                .and_then(Json::as_str)
                .ok_or_else(|| ApplyError::NoIdentity(entity_name.clone()))?;
            world
                .get(&(entity_name.clone(), id.to_owned()))
                .cloned()
                .ok_or_else(|| ApplyError::NoSuchInstance {
                    entity: entity_name.clone(),
                    id: id.to_owned(),
                })?
        }
    };

    // `refuses` is the whole claim of a wrong-state branch and means nothing anywhere else — the
    // IR says so: "`true` on every other kind of outcome and read by nothing there". A branch that
    // refuses reports its error and does not act; one that accepts leaves the subject where it is.
    if matches!(outcome.condition, ResolvedCondition::WrongState) {
        return Ok(Some(instance));
    }

    if let ResolvedEffect::Moves { transition } = &subject.effect {
        instance.state = transition.to.as_str().to_owned();
    }

    for written in &outcome.sets {
        if let Some(value) = read(&written.value, input) {
            instance
                .set(&written.target, value)
                .map_err(|why| ApplyError::UnknownField {
                    entity: why.entity,
                    field: why.field,
                })?;
        }
    }

    instance.revision += 1;
    Ok(Some(instance))
}

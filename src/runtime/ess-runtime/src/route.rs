//! The pump: carrying an emitted event to the command a binding says it causes.
//!
//! This is the dataflow. Nothing routes an event except a binding declared in `components.yaml`,
//! and the runtime reads those rather than inventing any: each carries the command to invoke, the
//! mapping from event fields to that command's inputs, a delivery guarantee and a failure policy.
//!
//! Two causes, and they arrive from opposite directions:
//!
//!   - an **event** cause fires because something happened — a command emitted the event the
//!     binding names, and the pump carries it on;
//!   - a **periodic** cause fires because time passed. ESS owns the cadence and the overlap bound;
//!     the host supplies the two kinds of field the contract names, and answers whether the
//!     occurrence is eligible at all.
//!
//! A binding's work is bounded on purpose. `at_most_once` delivery is not retried, `Escalate`
//! publishes the declared event rather than raising, and an unmet mapping is refused rather than
//! filled with a default — an event that cannot supply a command's input is a defect in the
//! specification, and inventing the value would hide it.

use std::collections::VecDeque;

use serde_json::{Map, Value as Json};

use ess_compiler::EssIr;
use ess_compiler::ir::{ResolvedBinding, ResolvedMappingValue};
use ess_domain::binding::{Delivery, Failure};

use crate::apply::{Applied, ApplyError, Emitted, World, apply};

/// How far a pump will chase one event before it stops.
///
/// A binding may emit an event another binding reacts to, which is the point; a cycle in that graph
/// is a specification defect the runtime must not spin on. The bound is here rather than in the
/// specification because ESS says nothing about it, and a runaway pump is the runtime's failure.
pub const MAX_DEPTH: usize = 64;

/// What the host supplies to one occurrence of a periodic binding.
///
/// Split exactly as the contract splits it: `context` is lifetime-constant — which goal this poll
/// belongs to — and `read` is sampled fresh per occurrence. `eligible` is the host's answer to
/// whether this occurrence should run at all, which is where a paused swarm is excluded.
pub struct Occurrence {
    /// The binding this is an occurrence of.
    pub binding: String,
    /// Lifetime-constant fields.
    pub context: Map<String, Json>,
    /// Fields sampled at this tick.
    pub read: Map<String, Json>,
    /// Whether the host admits this occurrence.
    pub eligible: bool,
}

/// One command a binding caused, and what applying it did.
#[derive(Debug, Clone)]
pub struct Routed {
    /// Which binding caused it.
    pub binding: String,
    /// The command invoked.
    pub command: String,
    /// What it did, or why it could not be done.
    pub result: Result<Applied, ApplyError>,
}

/// Why a binding could not be carried out at all.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteError {
    /// The event cannot supply an input the command requires.
    Unmappable {
        binding: String,
        target: String,
        why: String,
    },
    /// No such binding.
    NoSuchBinding(String),
    /// The pump chased one event further than [`MAX_DEPTH`].
    TooDeep { from: String },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unmappable {
                binding,
                target,
                why,
            } => write!(f, "binding `{binding}` cannot supply `{target}`: {why}"),
            Self::NoSuchBinding(name) => write!(f, "no binding `{name}`"),
            Self::TooDeep { from } => write!(
                f,
                "routing `{from}` exceeded {MAX_DEPTH} steps, which is a cycle in the bindings"
            ),
        }
    }
}

impl std::error::Error for RouteError {}

/// Carries every event through the bindings that react to it, and to the commands they invoke.
///
/// Returns each command that was caused, in the order it was caused. The world is advanced as it
/// goes, because a binding's command acts on the world the previous one left.
pub fn pump(
    ir: &EssIr,
    world: &mut World,
    events: Vec<Emitted>,
    ids: &mut dyn FnMut() -> String,
) -> Result<Vec<Routed>, RouteError> {
    let mut queue: VecDeque<Emitted> = events.into();
    let mut caused = Vec::new();
    let mut steps = 0;

    while let Some(event) = queue.pop_front() {
        steps += 1;
        if steps > MAX_DEPTH {
            return Err(RouteError::TooDeep { from: event.name });
        }

        for (name, binding) in ir.bindings() {
            let Some(handle) = binding.cause.event() else {
                continue;
            };
            if handle.name().to_string() != event.name {
                continue;
            }

            let input = map_from_event(&name.to_string(), binding, &event)?;
            let routed = invoke(ir, world, &name.to_string(), binding, input, ids);
            enqueue(&routed, binding, &mut queue);
            caused.push(routed);
        }
    }

    Ok(caused)
}

/// Runs one occurrence of a periodic binding.
///
/// An ineligible occurrence does nothing and says so by returning `None` — the host decided, and
/// the contract's `eligibility: host_boolean` is exactly that decision being the host's to make.
pub fn tick(
    ir: &EssIr,
    world: &mut World,
    occurrence: &Occurrence,
    ids: &mut dyn FnMut() -> String,
) -> Result<Option<Routed>, RouteError> {
    let (name, binding) = ir
        .bindings()
        .iter()
        .find(|(name, _)| name.to_string() == occurrence.binding)
        .ok_or_else(|| RouteError::NoSuchBinding(occurrence.binding.clone()))?;

    if binding.cause.periodic().is_none() {
        return Err(RouteError::NoSuchBinding(occurrence.binding.clone()));
    }
    if !occurrence.eligible {
        return Ok(None);
    }

    let input = map_from_host(&name.to_string(), binding, occurrence)?;
    Ok(Some(invoke(
        ir,
        world,
        &name.to_string(),
        binding,
        input,
        ids,
    )))
}

/// Applies the command a binding invokes, and commits what it produced.
fn invoke(
    ir: &EssIr,
    world: &mut World,
    binding_name: &str,
    binding: &ResolvedBinding,
    input: Map<String, Json>,
    ids: &mut dyn FnMut() -> String,
) -> Routed {
    let command = binding.command.name().to_string();
    let minted = ids();

    // A binding is the system acting on itself, not an actor acting: `may` answers which actor may
    // issue a command, and there is no actor here to ask about.
    let result = apply(ir, world, None, &command, &input, Some(&minted));

    if let Ok(applied) = &result
        && let Some(instance) = applied.instance.clone()
    {
        world.insert((instance.entity.clone(), instance.id.clone()), instance);
    }

    Routed {
        binding: binding_name.to_owned(),
        command,
        result,
    }
}

/// Queues what a caused command emitted, subject to the binding's delivery and failure policy.
fn enqueue(routed: &Routed, binding: &ResolvedBinding, queue: &mut VecDeque<Emitted>) {
    match &routed.result {
        Ok(applied) => queue.extend(applied.events.iter().cloned()),
        Err(_) => match binding.on_failure() {
            // The declared escalation event is a fact, published like any other, so whatever reacts
            // to it gets its turn. This is how a lost delivery becomes visible instead of silent.
            ess_compiler::ir::ResolvedFailure::Escalate { emits } => {
                queue.push_back(Emitted {
                    name: emits.name().to_string(),
                    fields: Map::new(),
                });
            }
            // Retrying is the caller's business: this pump has no clock and no attempt budget, and
            // a tight in-process retry loop would be a busy-wait rather than a policy.
            ess_compiler::ir::ResolvedFailure::Retry => {
                debug_assert_eq!(binding.delivery, Delivery::AtLeastOnce);
            }
            ess_compiler::ir::ResolvedFailure::Drop => {}
        },
    }
}

/// The command's input, built from the event the binding reacted to.
fn map_from_event(
    binding_name: &str,
    binding: &ResolvedBinding,
    event: &Emitted,
) -> Result<Map<String, Json>, RouteError> {
    let mut input = Map::new();
    for mapping in &binding.mapping {
        let value = match &mapping.value {
            ResolvedMappingValue::EventField { field, .. } => event.fields.get(field).cloned(),
            ResolvedMappingValue::EventAccessor { plan, .. } => walk(&event.fields, plan),
            // A literal in a binding is text. ess-domain says so, and checks enum membership or
            // admits text for String-backed targets; it does not make `"7"` a number.
            ResolvedMappingValue::Literal { value } => Some(Json::String(value.clone())),
            ResolvedMappingValue::HostContext { .. }
            | ResolvedMappingValue::HostRead { .. }
            | ResolvedMappingValue::Selection { .. } => {
                return Err(RouteError::Unmappable {
                    binding: binding_name.to_owned(),
                    target: mapping.target.clone(),
                    why: "reads the host, but this binding was caused by an event".to_owned(),
                });
            }
        };

        let value = value.ok_or_else(|| RouteError::Unmappable {
            binding: binding_name.to_owned(),
            target: mapping.target.clone(),
            why: format!("`{}` carries no such value", event.name),
        })?;
        input.insert(mapping.target.clone(), value);
    }
    Ok(input)
}

/// The command's input, built from what the host supplied for this occurrence.
fn map_from_host(
    binding_name: &str,
    binding: &ResolvedBinding,
    occurrence: &Occurrence,
) -> Result<Map<String, Json>, RouteError> {
    let mut input = Map::new();
    for mapping in &binding.mapping {
        let value = match &mapping.value {
            ResolvedMappingValue::HostContext { field, .. } => occurrence.context.get(field),
            ResolvedMappingValue::HostRead { field, .. } => occurrence.read.get(field),
            ResolvedMappingValue::Literal { value } => {
                input.insert(mapping.target.clone(), Json::String(value.clone()));
                continue;
            }
            ResolvedMappingValue::EventField { .. }
            | ResolvedMappingValue::EventAccessor { .. }
            | ResolvedMappingValue::Selection { .. } => {
                return Err(RouteError::Unmappable {
                    binding: binding_name.to_owned(),
                    target: mapping.target.clone(),
                    why: "reads an event, but this binding was caused by a period".to_owned(),
                });
            }
        };

        let value = value.cloned().ok_or_else(|| RouteError::Unmappable {
            binding: binding_name.to_owned(),
            target: mapping.target.clone(),
            why: "the host supplied no such field for this occurrence".to_owned(),
        })?;
        input.insert(mapping.target.clone(), value);
    }
    Ok(input)
}

/// Follows an accessor's source spelling into a nested event payload.
///
/// The plan also carries a node graph, which exists for a walk that crosses a union and has to know
/// which branch it is on. This follows the segments only, and answers `None` where that is not
/// enough — a mapping that needs the graph is refused as unmappable rather than guessed at, which
/// is the honest failure. No binding in this kernel uses one.
fn walk(fields: &Map<String, Json>, plan: &ess_domain::accessor::AccessorPlan) -> Option<Json> {
    let mut here = fields.get(plan.segments.first()?)?;
    for segment in plan.segments.iter().skip(1) {
        here = here.as_object()?.get(segment)?;
    }
    Some(here.clone())
}

/// Whether every binding's delivery promise is one this pump keeps.
///
/// `at_least_once` asks for redelivery, which needs a store and a clock — so a caller that has
/// neither should know which bindings it is under-serving rather than find out in production.
pub fn needs_redelivery(ir: &EssIr) -> Vec<String> {
    ir.bindings()
        .iter()
        .filter(|(_, binding)| {
            binding.delivery == Delivery::AtLeastOnce && binding.failure == Failure::Retry
        })
        .map(|(name, _)| name.to_string())
        .collect()
}

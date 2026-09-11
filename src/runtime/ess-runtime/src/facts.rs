//! Making instances and command inputs readable by the specification's own predicate evaluator.
//!
//! This module is the whole of "writing a guard evaluator". `ess-primitives` ships the AST, the
//! parser and a three-valued Kleene evaluator; what it needs is somewhere to look a fact up. So the
//! guards in the specification — `reached == true`, `exit_code != 0` — are decided by the same code
//! that type-checked them, and this crate never parses a predicate.
//!
//! Two sources, because ESS guards read two different things:
//!
//!   - a command's `when:` reads that invocation's INPUT and nothing else (ESS-COMMAND-003), so
//!     [`Input`] is what an outcome is selected against;
//!   - a view's `filter:` and an entity's `invariants:` read the INSTANCE, so [`Observed`] is what
//!     those are decided against.
//!
//! Mixing them would be a bug that reads plausibly, which is why they are separate types rather than
//! one source with a flag.

use serde_json::Value as Json;

use ess_primitives::facts::{FactPath, FactSource, FactValue};

use crate::instance::Instance;

/// One JSON value as a fact, or `None` when it cannot be one.
///
/// A null is deliberately not a fact. ESS's third truth value means "nothing observed this", and a
/// null is the clearest case of exactly that; answering `Text("null")` would make `field == null`
/// comparable, which the grammar never promised.
fn as_fact(value: &Json) -> Option<FactValue> {
    match value {
        Json::Bool(flag) => Some(FactValue::bool(*flag)),
        Json::Number(number) => number.as_f64().and_then(|n| FactValue::number(n).ok()),
        Json::String(text) => Some(FactValue::text(text.clone())),
        // An array or an object is not a scalar. A quantifier reads `<path>.count` and
        // `<path>.0.…`; until something needs that, a collection is unobserved rather than wrong.
        Json::Null | Json::Array(_) | Json::Object(_) => None,
    }
}

/// The input of one command invocation — what a `when:` guard is allowed to read.
pub struct Input<'a> {
    fields: &'a serde_json::Map<String, Json>,
}

impl<'a> Input<'a> {
    /// Reads the arguments of one invocation.
    pub fn new(fields: &'a serde_json::Map<String, Json>) -> Self {
        Self { fields }
    }
}

impl FactSource for Input<'_> {
    fn fact(&self, path: &FactPath) -> Option<FactValue> {
        // A command's input is flat: one segment, the field's name.
        let [field] = path.segments() else {
            return None;
        };
        self.fields.get(field).and_then(as_fact)
    }
}

/// An instance at rest — what a view's filter and an entity's invariants read.
pub struct Observed<'a> {
    instance: &'a Instance,
}

impl<'a> Observed<'a> {
    /// Reads one instance.
    pub fn new(instance: &'a Instance) -> Self {
        Self { instance }
    }
}

impl FactSource for Observed<'_> {
    fn fact(&self, path: &FactPath) -> Option<FactValue> {
        let [field] = path.segments() else {
            return None;
        };

        // `state` is not a declared field but it is observable, and filters read it constantly —
        // `state == Active`, `state == Evaluating`. The compiler hands it over as a synthetic
        // field on the entity, so a view that names it is already checked against the state enum.
        if field == "state" {
            return Some(FactValue::text(self.instance.state.clone()));
        }

        self.instance.get(field).and_then(as_fact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ess_primitives::predicate::{Predicate, Truth};
    use serde_json::json;

    fn input(value: serde_json::Value) -> serde_json::Map<String, Json> {
        value.as_object().expect("an object").clone()
    }

    #[test]
    fn the_exit_guard_decides_on_the_input() {
        let guard = Predicate::parse_expression("reached == true").expect("a valid predicate");

        let yes = input(json!({"reached": true}));
        assert_eq!(guard.evaluate(&Input::new(&yes)), Truth::True);

        let no = input(json!({"reached": false}));
        assert_eq!(guard.evaluate(&Input::new(&no)), Truth::False);
    }

    #[test]
    fn a_fact_nobody_supplied_is_unknown_not_false() {
        let guard = Predicate::parse_expression("reached == true").expect("a valid predicate");
        let silent = input(json!({}));

        // The whole reason `Field::Undetermined` exists: this must not be `False`.
        assert_eq!(guard.evaluate(&Input::new(&silent)), Truth::Unknown);
    }

    #[test]
    fn a_view_filter_reads_the_lifecycle_state() {
        let guard = Predicate::parse_expression("state == Evaluating").expect("a valid predicate");

        let mut goal = Instance::created(
            "swarm.goal.Goal",
            "g1",
            "Evaluating",
            ["text".to_owned(), "iterations".to_owned()],
        );
        assert_eq!(guard.evaluate(&Observed::new(&goal)), Truth::True);

        goal.state = "Reached".to_owned();
        assert_eq!(guard.evaluate(&Observed::new(&goal)), Truth::False);
    }

    #[test]
    fn an_undetermined_field_is_unknown_on_an_instance_too() {
        let guard = Predicate::parse_expression("iterations > 0").expect("a valid predicate");
        let mut goal = Instance::created(
            "swarm.goal.Goal",
            "g1",
            "Open",
            ["iterations".to_owned()],
        );
        assert_eq!(guard.evaluate(&Observed::new(&goal)), Truth::Unknown);

        goal.set("iterations", json!(2)).expect("a declared field");
        assert_eq!(guard.evaluate(&Observed::new(&goal)), Truth::True);
    }

    #[test]
    fn a_field_the_entity_does_not_declare_is_refused() {
        let mut goal = Instance::created("swarm.goal.Goal", "g1", "Open", ["text".to_owned()]);
        let refused = goal.set("invented", json!(1)).unwrap_err();
        assert_eq!(refused.field, "invented");
    }
}

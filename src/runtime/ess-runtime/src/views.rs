//! Computing a view: filter, sort, project.
//!
//! A view is a read model the specification already describes completely — which entity it reads,
//! which instances it contains, in what order, and which of their fields it shows. All four are in
//! the IR, and the filter arrives as a parsed `Predicate` rather than a string, so `ListAgents()`
//! is three steps over instances and no query language at all.
//!
//! Two things the specification decides and this honours rather than improving on.
//!
//! A filter is satisfied only when it evaluates `True`. `Unknown` — a row whose filter reads a field
//! nothing has written — is not in the view, and is not excluded for being false either; it is
//! excluded for not being observably true, which is what the third truth value means. A view that
//! silently included unknowns would be answering a question nobody asked.
//!
//! `consistency` is reported, not enforced. Every view here is computed from the world as it stands,
//! so a view declared `eventual` is served immediately. That is a deviation, and a deliberate one:
//! reproducing lag matters when a suite is checking that a consumer tolerates it, and it is pure
//! harm when a person is waiting for a canvas to redraw. [`Computed::consistency`] carries the
//! declaration so a caller can tell the difference.

use serde_json::{Map, Value as Json};

use ess_compiler::EssIr;
use ess_domain::view::{Consistency, Direction};
use ess_primitives::predicate::Truth;

use crate::apply::World;
use crate::facts::Observed;
use crate::instance::Instance;

/// One row of a computed view: the projected fields, in the order the view declares them.
pub type Row = Map<String, Json>;

/// A view, computed.
#[derive(Debug, Clone)]
pub struct Computed {
    /// The view's qualified name.
    pub name: String,
    /// What the specification promises about its freshness. See the module note.
    pub consistency: Consistency,
    /// The rows, filtered and ordered.
    pub rows: Vec<Row>,
}

/// Why a view could not be computed.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewError {
    /// No such view in this specification.
    NoSuchView(String),
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchView(name) => write!(f, "no view `{name}`"),
        }
    }
}

impl std::error::Error for ViewError {}

/// Computes one view over the world.
pub fn view(ir: &EssIr, world: &World, name: &str) -> Result<Computed, ViewError> {
    let (_, declared) = ir
        .views()
        .iter()
        .find(|(view, _)| view.to_string() == name)
        .ok_or_else(|| ViewError::NoSuchView(name.to_owned()))?;

    let source = declared.source.name().to_string();

    let mut matched: Vec<&Instance> = world
        .values()
        .filter(|instance| instance.entity == source)
        .filter(|instance| match &declared.filter {
            // Only `True`. An `Unknown` row is one whose filter reads something nobody wrote.
            Some(predicate) => predicate.evaluate(&Observed::new(instance)) == Truth::True,
            None => true,
        })
        .collect();

    // Last key first, so that earlier keys win — a stable sort per key, applied in reverse, orders
    // by the whole list without needing a comparator over it.
    for ranking in declared.order_by.iter().rev() {
        matched.sort_by(|left, right| {
            let ordering = compare(left.get(&ranking.field), right.get(&ranking.field));
            match ranking.direction {
                Direction::Ascending => ordering,
                Direction::Descending => ordering.reverse(),
            }
        });
    }

    let rows = matched
        .into_iter()
        .map(|instance| project(instance, declared))
        .collect();

    Ok(Computed {
        name: name.to_owned(),
        consistency: declared.consistency,
        rows,
    })
}

/// Every view over one entity, computed. What a canvas asks for when it redraws.
pub fn views_over(ir: &EssIr, world: &World, entity: &str) -> Vec<Computed> {
    ir.views()
        .iter()
        .filter(|(_, declared)| declared.source.name().to_string() == entity)
        .filter_map(|(name, _)| view(ir, world, &name.to_string()).ok())
        .collect()
}

/// The fields a view shows, and only those.
///
/// A view is a promise about what it contains, so a field the view does not declare is absent even
/// when the instance holds it. `state` is projected from the lifecycle rather than the field map,
/// the same way a filter reads it.
fn project(instance: &Instance, declared: &ess_compiler::ir::ResolvedView) -> Row {
    let mut row = Row::new();
    for field in &declared.fields {
        let name = field.name.as_str();
        let value = if name == "state" {
            Some(Json::String(instance.state.clone()))
        } else if name == instance.identity_field {
            Some(Json::String(instance.id.clone()))
        } else {
            instance.get(name).cloned()
        };

        // An undetermined field is present and null: a consumer must be able to tell a row that
        // lacks a value from one the view never promised.
        row.insert(name.to_owned(), value.unwrap_or(Json::Null));
    }
    row
}

/// Orders two field values.
///
/// Numbers compare as numbers, text as text, booleans as false-then-true.
///
/// An undetermined value sorts as less, and reversing the direction reverses that with everything
/// else — it is not pinned to either end. Ordering by a field half the rows have not written is a
/// specification defect, and a rule that quietly herded the blanks to the bottom would hide it.
fn compare(left: Option<&Json>, right: Option<&Json>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => match (a, b) {
            (Json::Number(a), Json::Number(b)) => a
                .as_f64()
                .partial_cmp(&b.as_f64())
                .unwrap_or(Ordering::Equal),
            (Json::String(a), Json::String(b)) => a.cmp(b),
            (Json::Bool(a), Json::Bool(b)) => a.cmp(b),
            _ => Ordering::Equal,
        },
    }
}

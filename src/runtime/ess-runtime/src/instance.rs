//! An entity instance, and the three answers a field can give.
//!
//! A field is `Undetermined`, or it holds a value. The distinction is not pedantry: ESS decides what
//! an outcome writes but not, for example, what time it is, so a field nothing has supplied is a
//! field nobody has observed — and a predicate over it must come back `Unknown`, not `False`. An
//! interpreter that defaults undetermined fields to zero or empty turns "nobody looked" into "it is
//! wrong", which is the one thing the three-valued evaluator exists to avoid.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// What an entity's field holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Field {
    /// A value was written.
    Known(Json),
    /// Nothing has written this field. Distinct from a written `null`.
    Undetermined,
}

impl Field {
    /// The value, when one was written.
    pub fn known(&self) -> Option<&Json> {
        match self {
            Self::Known(value) => Some(value),
            Self::Undetermined => None,
        }
    }
}

impl From<Json> for Field {
    fn from(value: Json) -> Self {
        Self::Known(value)
    }
}

/// One instance of one entity: what it is, which state it rests in, and what its fields hold.
///
/// The identity is kept out of `fields` deliberately. It is the one value that cannot change and
/// cannot be undetermined, and a field anybody could write is not an identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    /// The qualified entity name, such as `swarm.goal.Goal`.
    pub entity: String,
    /// The identity value, rendered as text — the form a fact path reads it in.
    pub id: String,
    /// The lifecycle state it currently rests in.
    pub state: String,
    /// Every declared field, including the ones nothing has written.
    pub fields: BTreeMap<String, Field>,
    /// How many events have been applied to it. Not a field; the store's business.
    pub revision: u64,
}

impl Instance {
    /// A new instance in its lifecycle's initial state, with every field undetermined.
    pub fn created(
        entity: impl Into<String>,
        id: impl Into<String>,
        state: impl Into<String>,
        declared: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            entity: entity.into(),
            id: id.into(),
            state: state.into(),
            fields: declared
                .into_iter()
                .map(|name| (name, Field::Undetermined))
                .collect(),
            revision: 0,
        }
    }

    /// Writes a field. A field the entity does not declare is refused rather than invented.
    pub fn set(&mut self, field: &str, value: Json) -> Result<(), UnknownField> {
        match self.fields.get_mut(field) {
            Some(slot) => {
                *slot = Field::Known(value);
                Ok(())
            }
            None => Err(UnknownField {
                entity: self.entity.clone(),
                field: field.to_owned(),
            }),
        }
    }

    /// The value of a field, when something has written it.
    pub fn get(&self, field: &str) -> Option<&Json> {
        self.fields.get(field).and_then(Field::known)
    }
}

/// A write to a field the entity does not declare.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownField {
    pub entity: String,
    pub field: String,
}

impl std::fmt::Display for UnknownField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` declares no field `{}`", self.entity, self.field)
    }
}

impl std::error::Error for UnknownField {}

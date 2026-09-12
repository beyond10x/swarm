//! An interpreter for an Executable System Specification.
//!
//! ESS says what a system is: entities with lifecycles, commands whose outcomes move them and emit
//! events, bindings that route those events back into commands, and views that project the result.
//! It does not say how any of it runs. This crate runs it.
//!
//! What it takes from `ess-compiler` rather than reinventing: the resolved IR, with every handle
//! already resolved to the thing it names; the parsed `Predicate` on every guard, together with the
//! three-valued evaluator that decides it; the precomputed set of states in which a command is in
//! the wrong state; and the binding table, with each field mapping and its delivery and failure
//! policy. What it adds: instances, the application of a command to one, the pump that carries an
//! emitted event to the next command, and the projection of views.

pub mod apply;
pub mod facts;
pub mod instance;
pub mod route;
pub mod spec;
pub mod store;
pub mod views;

pub use apply::{Applied, ApplyError, Emitted, World, apply};
pub use instance::{Field, Instance};
pub use route::{Occurrence, RouteError, Routed, pump, tick};
pub use spec::{LoadError, Spec};
pub use store::{Recorded, Store, StoreError};
pub use views::{Computed, Row, ViewError, view, views_over};

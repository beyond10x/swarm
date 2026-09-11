//! Serving one swarm manager over HTTP.
//!
//! The interpreter in `ess-runtime` can run any ESS specification; this crate serves THIS one. What
//! is swarm-specific lives here — which view the trigger reads to find work, what makes a swarm
//! eligible to tick — and everything else is the interpreter doing what the model says.

pub mod coordinator;
pub mod http;
pub mod state;
pub mod swarm;
pub mod trigger;

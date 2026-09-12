//! Serving one swarm manager over HTTP.
//!
//! The interpreter in `ess-runtime` can run any ESS specification; this crate serves THIS one. What
//! is swarm-specific lives here — which view the trigger reads to find work, what makes a swarm
//! eligible to tick — and everything else is the interpreter doing what the model says.

pub mod budget;
pub mod coordinator;
pub mod http;
pub mod state;
pub mod swarm;
pub mod trigger;

/// One lock over the process environment, for the cases that must read a real variable.
///
/// `Caps::configured` and `coordinator::configured` read `std::env`, which is process-wide, and
/// `cargo test` runs a binary's cases on many threads at once. A case that sets `SWARM_MAX_TURNS`
/// and one that unsets `SWARM_COORDINATOR` are then reading each other's world, and the failure is
/// intermittent — which is worse than either case not existing.
///
/// Every case in this crate that touches the environment holds this first. The alternative is to
/// stop driving the environment at all and construct `Caps` by hand, and that is precisely what hid
/// `SWARM_MAX_SPEND_USD=-1`: the only code that reads the variable had no test caller.
#[cfg(test)]
pub(crate) static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

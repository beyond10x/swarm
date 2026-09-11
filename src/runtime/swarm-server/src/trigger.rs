//! The host half of a periodic binding.
//!
//! The specification owns the cadence, the overlap bound and the missed-tick policy; this owns the
//! two things the contract leaves to the host, and nothing else:
//!
//!   - **which instance** an occurrence is for, and what to read off it. The binding's
//!     `context_fields` and `read_fields` say what is needed; a view says which instances need it.
//!   - **eligibility** — whether this occurrence should run at all. That is where a swarm that is
//!     not running is excluded, because "is this swarm running" is not a fact the binding can read.
//!
//! What it does not own is when. `every: PT30S` is in the specification, and this reads it from
//! there rather than carrying a constant of its own — a cadence written in two places is a cadence
//! that will eventually be two different cadences.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value as Json};

use ess_runtime::route::Occurrence;

use crate::state::Server;
use crate::swarm::Swarm;

/// The view that says which goals are waiting for a turn.
///
/// Named here because the binding does not name it: the contract says the host supplies
/// `goal_id`, and which goals need one is the host's question to answer.
const AWAITING: &str = "swarm.goal.GoalsAwaitingATick";

/// Drives every periodic binding the specification declares, for every swarm the server holds.
///
/// Runs until cancelled. One task for the server rather than one per swarm: the cadence belongs to
/// the binding, not to the swarm, and N tasks ticking the same period would be N chances to drift.
pub async fn run(server: Arc<Server>) {
    let periodic = declared_periods(&server);
    if periodic.is_empty() {
        tracing::info!("no periodic bindings; the trigger has nothing to drive");
        return;
    }

    for (binding, every) in &periodic {
        tracing::info!(binding = %binding, every = ?every, "driving a periodic binding");
    }

    // `first: after_period` — there is no immediate call, so the first sleep comes before the first
    // tick. The specification says so; this is not a choice being made here.
    let shortest = periodic
        .iter()
        .map(|(_, every)| *every)
        .min()
        .unwrap_or(Duration::from_secs(30));

    loop {
        tokio::time::sleep(shortest).await;
        for swarm in server.all().await {
            for (binding, _) in &periodic {
                if let Err(why) = fire(&server, &swarm, binding).await {
                    tracing::warn!(swarm = %swarm.slug(), binding = %binding, error = %why,
                                   "an occurrence could not be delivered");
                }
            }
        }
    }
}

/// Every periodic binding, with the period the specification declares for it.
fn declared_periods(server: &Server) -> Vec<(String, Duration)> {
    server
        .spec()
        .ir()
        .bindings()
        .iter()
        .filter_map(|(name, binding)| {
            let periodic = binding.cause.periodic()?;
            Some((
                name.to_string(),
                Duration::from_secs(u64::from(periodic.contract.every.seconds())),
            ))
        })
        .collect()
}

/// Delivers one occurrence per instance that wants one.
async fn fire(server: &Server, swarm: &Arc<Swarm>, binding: &str) -> Result<(), String> {
    let waiting = swarm.view(AWAITING).await.map_err(|why| why.to_string())?;

    for row in waiting {
        let Some(goal) = row.get("goal_id").and_then(Json::as_str) else {
            continue;
        };

        let mut context = Map::new();
        context.insert("goal_id".to_owned(), Json::String(goal.to_owned()));

        // `iterations` counts the turns driven. ESS has no arithmetic, so the count is the host's
        // to keep — it reads what the last turn recorded and hands on the next number.
        let so_far = row
            .get("iterations")
            .and_then(Json::as_u64)
            .unwrap_or_default();
        let mut read = Map::new();
        read.insert("iterations".to_owned(), Json::from(so_far + 1));

        let occurrence = Occurrence {
            binding: binding.to_owned(),
            context,
            read,
            eligible: eligible(server, swarm).await,
        };

        match swarm.tick(occurrence).await {
            Ok(true) => tracing::debug!(swarm = %swarm.slug(), goal, "turned the loop"),
            Ok(false) => tracing::trace!(swarm = %swarm.slug(), goal, "not eligible"),
            Err(why) => return Err(why.to_string()),
        }
    }
    Ok(())
}

/// Whether this swarm's loop should turn at all.
///
/// `eligibility: host_boolean` puts this decision here because the binding cannot read it: whether
/// a swarm is Running is a fact about a different entity than the one being ticked. A paused or
/// stopped swarm has its occurrences dropped, silently, which is what pausing means.
async fn eligible(server: &Server, swarm: &Arc<Swarm>) -> bool {
    let _ = server;
    let running = swarm
        .instances("swarm.manager.Swarm")
        .await
        .into_iter()
        .any(|instance| {
            instance
                .get("state")
                .and_then(Json::as_str)
                .is_some_and(|state| state == "Running")
        });

    // A swarm with no Swarm record has not been created yet — its log is an empty directory, and a
    // loop turning in it would be acting on a swarm nobody made.
    running
}

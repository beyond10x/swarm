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
use std::time::{Duration, Instant};

use serde_json::{Map, Value as Json};

use ess_runtime::route::Occurrence;

use crate::coordinator;
use crate::state::Server;
use crate::swarm::{Swarm, TurnPhase, What};

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
    let periodic = server.periods();
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
        server.tick_scheduled(shortest);
        tokio::time::sleep(shortest).await;
        server.ticked();
        for swarm in server.all().await {
            for (binding, _) in &periodic {
                if let Err(why) = fire(&server, &swarm, binding).await {
                    tracing::warn!(swarm = %swarm.slug(), binding = %binding, error = %why,
                                   "an occurrence could not be delivered");
                }
            }
            // A turn the loop started is a turn something has to finish. This is the middle arrow
            // of `[loop] -> [coordinator] -> [goal]`, and it is run after the tick rather than
            // inside it because the tick's job ends when the goal is Pursuing.
            ask_the_coordinator(&server, &swarm).await;
        }
    }
}

/// Asks the coordinator about every goal that is mid-turn.
///
/// Goals already in `Pursuing` are picked up too, not only ones this cycle started — a turn
/// interrupted by a restart is still a turn nobody answered.
///
/// Each turn runs as its own task. A coordinator that thinks for three minutes must not hold the
/// trigger for three minutes, and `overlap: serial_per_instance` is kept by the claim on the goal
/// rather than by running everything in one line.
async fn ask_the_coordinator(server: &Arc<Server>, swarm: &Arc<Swarm>) {
    for instance in swarm.instances("swarm.goal.Goal").await {
        if instance.get("state").and_then(Json::as_str) != Some("Pursuing") {
            continue;
        }
        let Some(id) = instance.get("id").and_then(Json::as_str) else {
            continue;
        };
        if !server.claim_turn(swarm.slug(), id) {
            continue;
        }
        let fields = instance.get("fields");
        let goal = fields
            .and_then(|f| f.get("text"))
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();
        let iterations = fields
            .and_then(|f| f.get("iterations"))
            .and_then(Json::as_u64)
            .unwrap_or_default();

        let server = Arc::clone(server);
        let swarm = Arc::clone(swarm);
        let id = id.to_owned();
        tokio::spawn(async move {
            one_turn(&swarm, &id, &goal, iterations).await;
            server.release_turn(swarm.slug(), &id);
        });
    }
}

/// One coordinator turn, announced at both ends.
async fn one_turn(swarm: &Arc<Swarm>, id: &str, goal: &str, iterations: u64) {
    // Watchers hear the turn begin, so a coordinator that takes a minute is seen working
    // rather than seen as a loop that stalled.
    swarm.announce(What::Turn {
        goal: id.to_owned(),
        iterations,
        phase: TurnPhase::Asking,
        reached: None,
        note: None,
        error: None,
        took_ms: None,
        spent: None,
    });
    let began = Instant::now();

    match coordinator::take_a_turn(swarm, id, goal, iterations).await {
        Ok(answer) => {
            tracing::info!(swarm = %swarm.slug(), goal = %id, reached = answer.verdict.reached,
                           "a turn was answered");
            swarm.announce(What::Turn {
                goal: id.to_owned(),
                iterations,
                phase: TurnPhase::Answered,
                reached: Some(answer.verdict.reached),
                note: answer.verdict.note,
                error: None,
                took_ms: Some(began.elapsed().as_millis() as u64),
                spent: Some(answer.spent),
            });
        }
        // Not configured is the ordinary state of a swarm nobody has given a coordinator, and
        // logging it every period would bury everything else. Watchers are told once per turn,
        // because for them it is the one fact that explains a goal sitting in Pursuing.
        Err(why) => {
            match &why {
                coordinator::Unfinished::NoCoordinator => {
                    tracing::debug!(swarm = %swarm.slug(), goal = %id, "waiting for a coordinator");
                }
                _ => {
                    tracing::warn!(swarm = %swarm.slug(), goal = %id, error = %why,
                                   "the turn could not be finished");
                }
            }
            swarm.announce(What::Turn {
                goal: id.to_owned(),
                iterations,
                phase: TurnPhase::Unfinished,
                reached: None,
                note: None,
                error: Some(why.to_string()),
                took_ms: Some(began.elapsed().as_millis() as u64),
                spent: why.spent().cloned(),
            });
        }
    }
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

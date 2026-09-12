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

use crate::budget::Reached;
use crate::coordinator;
use crate::state::{CappedGoal, Server};
use crate::swarm::{Swarm, TurnPhase, What};

/// The view that says which goals are waiting for a turn.
///
/// Named here because the binding does not name it: the contract says the host supplies
/// `goal_id`, and which goals need one is the host's question to answer.
const AWAITING: &str = "swarm.goal.GoalsAwaitingATick";

/// The role slug a swarm's first agent holds. `swarm.agent.Role` declares exactly one variant, so
/// there is exactly one of these until the enum grows.
pub const COORDINATOR: &str = "coordinator";

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
        // The clock the redelivery queue needed. A failed `at_least_once` delivery is re-attempted
        // from here rather than from a timer of its own: the trigger is already the one loop that
        // walks every swarm on a period, and a second one would be a second cadence to keep in step
        // with the first.
        //
        // What this is NOT is a thirty-second retry. `RETRY_DELAY` is a floor: a delivery queued
        // just after a drain waits a full period beyond it, so the real wait is 30 to 60 seconds,
        // and the drain of the LAST swarm additionally waits for every earlier swarm's occurrences
        // — `fire` is awaited, not spawned — so the upper bound grows with the number of swarms
        // held. Draining every swarm before any of them is fired keeps that growth to one pass of
        // cheap work rather than one pass of the tick path; it does not remove it. A bound that
        // does not move with the swarm count needs its own task, which is the second cadence this
        // deliberately does not have.
        for swarm in server.all().await {
            let attempted = swarm.redeliver(Instant::now()).await;
            if attempted > 0 {
                tracing::info!(swarm = %swarm.slug(), attempted, "re-attempted owed deliveries");
            }
        }

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
    // The coordinator is a record before it is a process, so it can be addressed, drawn and
    // written to. Idempotent; a swarm made before this existed gets one here.
    if let Err(why) = swarm.ensure_coordinator(COORDINATOR).await {
        tracing::warn!(swarm = %swarm.slug(), error = %why, "the coordinator could not be registered");
    }

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

        // A goal past its cap is not asked, even when a turn left it in `Pursuing`. Without this
        // the cap would stop the tick and the coordinator would still be run once a period.
        if capped(server, swarm, id, iterations).is_some() {
            server.release_turn(swarm.slug(), id);
            continue;
        }

        let server = Arc::clone(server);
        let swarm = Arc::clone(swarm);
        let id = id.to_owned();
        tokio::spawn(async move {
            one_turn(&swarm, COORDINATOR, &id, &goal, iterations).await;
            server.release_turn(swarm.slug(), &id);
        });
    }
}

/// One coordinator turn, announced at both ends.
async fn one_turn(swarm: &Arc<Swarm>, actor: &str, id: &str, goal: &str, iterations: u64) {
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

    match coordinator::take_a_turn(swarm, actor, id, goal, iterations).await {
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
            // A turn that failed still ran and still cost money. Recording it is what makes the
            // caps able to see a coordinator that never answers; without this, the goal stays at
            // the same turn number for ever and both caps read zero while the money goes out.
            if let Some(spent) = why.spent() {
                swarm.record_spend(id, iterations, spent, None, Some(&why.to_string()));
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

        // A goal past its cap is ineligible, the same way a paused swarm's goals are. Nothing in
        // the model changes; the loop stops asking, and raising the cap resumes it here.
        let within_budget = match capped(server, swarm, goal, so_far) {
            None => true,
            Some(reached) => {
                let (spent, _) = swarm.spend_on(goal);
                let capped = CappedGoal {
                    swarm: swarm.slug().to_owned(),
                    goal: goal.to_owned(),
                    turns: so_far,
                    spent_usd: spent.cost_usd,
                    reached,
                    why: reached.to_string(),
                };
                if server.report_capped(capped.clone()) {
                    tracing::warn!(swarm = %swarm.slug(), goal, %reached, "the loop stopped asking");
                    swarm.announce(What::Capped {
                        goal: capped.goal,
                        turns: capped.turns,
                        spent_usd: capped.spent_usd,
                        reached: capped.reached,
                        why: capped.why,
                    });
                }
                false
            }
        };

        let occurrence = Occurrence {
            binding: binding.to_owned(),
            context,
            read,
            eligible: within_budget && eligible(server, swarm).await,
        };

        match swarm.tick(occurrence).await {
            Ok(true) => tracing::debug!(swarm = %swarm.slug(), goal, "turned the loop"),
            Ok(false) => tracing::trace!(swarm = %swarm.slug(), goal, "not eligible"),
            Err(why) => return Err(why.to_string()),
        }
    }
    Ok(())
}

/// Whether this goal has used up what it may.
///
/// Measured on ATTEMPTS, not on the goal's own `iterations`. A turn that fails leaves the goal
/// where it was, so the next attempt carries the same number — and a coordinator that never writes
/// a verdict would sit at turn 1 for ever, with the turn cap reading 1 on every pass, while it
/// spent without limit. Observed 2026-09-12, which is how this line came to be written.
///
/// The larger of the two is used, because a swarm whose records were archived should not have its
/// cap reset by the absence.
fn capped(server: &Server, swarm: &Arc<Swarm>, goal_id: &str, iterations: u64) -> Option<Reached> {
    let (spent, attempts) = swarm.spend_on(goal_id);
    server
        .caps()
        .exceeded(iterations.max(attempts), spent.cost_usd)
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

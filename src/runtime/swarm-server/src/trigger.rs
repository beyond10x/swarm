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

use crate::budget::{Caps, Reached};
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
        //
        // `iterations - 1`, and the subtraction is the whole point: `fire` has already applied the
        // `Pursue` for the turn about to be taken, so the goal's own field counts that turn, while
        // `fire` measured the ones BEFORE it. Two guards over one bound, disagreeing by one, stop
        // the loop a turn early — `SWARM_MAX_TURNS=3` ran twice. Both now measure turns already
        // taken, which is what a cap of three means.
        if let Some(reached) = capped(server.caps(), swarm, id, iterations.saturating_sub(1)) {
            // Reported from here as well as from `fire`, and that is not belt-and-braces: a
            // coordinator that never writes a verdict leaves its goal in `Pursuing`, which
            // `GoalsAwaitingATick` does not select, so `fire` never sees the goal again after the
            // first period and THIS is the only guard that ever trips. Stopping here in silence
            // left a goal sitting in `Pursuing` with nothing in `capped_goals()`, nothing on the
            // watch stream and no warning — which is precisely the picture the $11.35 run
            // presented to everybody who looked at it.
            report_capped(server, swarm, id, iterations.saturating_sub(1), reached);
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
        // "Nobody has given this swarm a coordinator" stopped being a failure when resolution
        // gained a default; what is left is a config that named one this host cannot launch, and
        // that is loud on purpose. Watchers are told either way, because for them it is the one
        // fact that explains a goal sitting in Pursuing.
        Err(why) => {
            tracing::warn!(swarm = %swarm.slug(), goal = %id, error = %why,
                           "the turn could not be finished");
            // A turn that failed still ran and still cost money. Recording it is what makes the
            // caps able to see a coordinator that never answers; without this, the goal stays at
            // the same turn number for ever and both caps read zero while the money goes out.
            if let Some(spent) = why.spent() {
                swarm.record_spend(actor, id, iterations, spent, None, Some(&why.to_string()));
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
        let within_budget = match capped(server.caps(), swarm, goal, so_far) {
            None => true,
            Some(reached) => {
                report_capped(server, swarm, goal, so_far, reached);
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

/// Records that the loop has stopped asking about a goal, and tells everybody who can hear.
///
/// Both cap guards call this, because a reader cannot tell which guard stopped a goal and must not
/// have to. `report_capped` is idempotent per goal, so the once-per-turn warning stays once.
fn report_capped(server: &Server, swarm: &Arc<Swarm>, goal: &str, turns: u64, reached: Reached) {
    let (spent, _) = swarm.spend_on(goal);
    let capped = CappedGoal {
        swarm: swarm.slug().to_owned(),
        goal: goal.to_owned(),
        turns,
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
fn capped(caps: Caps, swarm: &Arc<Swarm>, goal_id: &str, iterations: u64) -> Option<Reached> {
    let (spent, attempts) = swarm.spend_on(goal_id);
    caps.exceeded(iterations.max(attempts), spent.cost_usd)
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

#[cfg(test)]
mod bounds {
    use super::*;
    use crate::swarm::Swarm;
    use ess_runtime::Spec;
    use serde_json::json;

    /// A coordinator program satisfying the stdin→stdout contract, which never says `reached`.
    ///
    /// `examples/coordinator-manual.sh` is the same contract. A goal it can never finish is the
    /// shape that cost $11.35: the loop turns until something refuses, and the cap is the only
    /// thing that can.
    fn never_reached(at: &std::path::Path) -> String {
        std::fs::write(
            at,
            "#!/bin/sh\ncat > /dev/null\nprintf '{\"reached\": false, \"note\": \"not yet\"}\\n'\n",
        )
        .expect("the coordinator is written");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755))
            .expect("the coordinator is runnable");
        at.display().to_string()
    }

    /// A started swarm with one goal, served by a real [`Server`].
    async fn a_swarm_with_a_goal(
        data: &tempdir::TempDir,
        slug: &str,
    ) -> (Arc<Server>, Arc<Swarm>, String) {
        let kernel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/core")
            .canonicalize()
            .expect("the kernel specification is beside this crate");
        let spec = Spec::load(kernel).expect("the kernel resolves");
        let server = Arc::new(
            Server::start(spec, data.path().to_path_buf())
                .await
                .expect("the server starts"),
        );
        let swarm = server.open(slug).await.expect("a swarm opens");

        let issue = async |command: &str, input: Json| {
            swarm
                .issue(
                    None,
                    command,
                    input.as_object().expect("an object").clone(),
                    command,
                )
                .await
                .expect("the command applies")
        };
        issue(
            "swarm.manager.CreateSwarm",
            json!({"display_name": slug, "tmux_session": slug, "home": "/tmp/x",
                   "created_at": "2026-09-12T10:00:00Z"}),
        )
        .await;
        let id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        issue(
            "swarm.manager.StartSwarm",
            json!({"swarm_id": id, "started_at": "2026-09-12T10:01:00Z"}),
        )
        .await;
        issue(
            "swarm.goal.SetGoal",
            json!({"swarm_id": id, "text": "2342342"}),
        )
        .await;
        let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        (server, swarm, goal)
    }

    /// One period of the real loop, exactly as [`run`] drives it: fire, then ask, then wait for
    /// the turns `ask_the_coordinator` spawned to finish.
    async fn one_period(server: &Arc<Server>, swarm: &Arc<Swarm>) {
        for (binding, _) in &server.periods() {
            fire(server, swarm, binding).await.expect("fired");
        }
        ask_the_coordinator(server, swarm).await;
        while server.turns_in_flight() > 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// The acceptance, driven: `SWARM_MAX_TURNS=3` stops a goal at three turns.
    ///
    /// Everything here is the real path. The cap is read from the environment by
    /// `Caps::configured` inside `Server::start`, which is the only code that reads that variable
    /// and which had no test caller at all until this one — that absence is exactly how
    /// `SWARM_MAX_SPEND_USD=-1` stayed invisible. The refusal is the guard at [`fire`] and the one
    /// at [`ask_the_coordinator`], reached through those functions rather than re-implemented
    /// beside them: delete either and this goes red, which was not true of the case it replaces.
    #[tokio::test]
    async fn a_goal_with_a_cap_of_three_stops_at_three_turns() {
        let _guard = crate::ENVIRONMENT.lock().await;
        let data = tempdir::TempDir::new("swarm-cap").expect("a scratch directory");
        let program = never_reached(&data.path().join("coordinator.sh"));
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_TURNS", "3");
            std::env::set_var("SWARM_MAX_SPEND_USD", "off");
            std::env::set_var("SWARM_COORDINATOR", &program);
        }

        let (server, swarm, goal) = a_swarm_with_a_goal(&data, "cap").await;

        // Twelve periods against a cap of three. A bound that holds ends this after three.
        for _ in 0..12 {
            one_period(&server, &swarm).await;
        }

        let (_, attempts) = swarm.spend_on(&goal);
        assert_eq!(attempts, 3, "a declared cap of three turns stops at three");
        assert_eq!(
            swarm.instances("swarm.goal.Goal").await[0]["fields"]["iterations"],
            json!(3),
            "and the goal itself went no further"
        );
        assert!(
            server.capped_goals().iter().any(|c| c.goal == goal),
            "the goal is reported capped, so a reader can see why the loop stopped"
        );
    }

    /// The `dsfsdf` shape: a coordinator that never writes a verdict, stopped at three attempts.
    ///
    /// The goal never leaves turn 1, because nothing it does advances its own count. A cap
    /// measured on `iterations` would read 1 for ever while the money went out; measured on
    /// attempts it trips. Driven through the real guard, with a program that exits non-zero.
    #[tokio::test]
    async fn a_coordinator_that_never_answers_is_stopped_at_three_attempts() {
        let _guard = crate::ENVIRONMENT.lock().await;
        let data = tempdir::TempDir::new("swarm-cap-unanswered").expect("a scratch directory");
        let program = data.path().join("coordinator.sh");
        std::fs::write(&program, "#!/bin/sh\ncat > /dev/null\nexit 3\n").expect("written");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("runnable");
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_TURNS", "3");
            std::env::set_var("SWARM_MAX_SPEND_USD", "off");
            std::env::set_var("SWARM_COORDINATOR", &program);
        }

        let (server, swarm, goal) = a_swarm_with_a_goal(&data, "unanswered").await;
        for _ in 0..12 {
            one_period(&server, &swarm).await;
        }

        let (_, attempts) = swarm.spend_on(&goal);
        assert_eq!(attempts, 3, "attempts are what the cap is measured on");
        assert!(
            swarm.instances("swarm.goal.Goal").await[0]["fields"]["iterations"]
                .as_u64()
                .is_some_and(|turns| turns <= 1),
            "while the goal's own count never advanced"
        );
    }

    /// Every cap `Caps::configured` reads refuses a negative value rather than lifting itself.
    ///
    /// The class, not the instance. `SWARM_MAX_SPEND_USD=-1` was the one found; it lifted the cap
    /// because `read` compared `<= T::default()`, while `SWARM_MAX_TURNS=-1` did the documented
    /// thing only because `u64` cannot parse it. Both names are listed here, and a third cap added
    /// to `Caps` without being added to this list is the next instance of the same defect — which
    /// is why the assertion below reads the whole struct rather than one field.
    #[tokio::test]
    async fn no_cap_is_lifted_by_a_value_that_is_merely_wrong() {
        let _guard = crate::ENVIRONMENT.lock().await;
        for bad in ["-1", "-0.01", "nonsense"] {
            // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT`.
            unsafe {
                std::env::set_var("SWARM_MAX_TURNS", bad);
                std::env::set_var("SWARM_MAX_SPEND_USD", bad);
            }
            let caps = crate::budget::Caps::configured();
            assert_eq!(
                caps.max_turns,
                Caps::default().max_turns,
                "SWARM_MAX_TURNS={bad} keeps the default"
            );
            assert_eq!(
                caps.max_spend_usd,
                Caps::default().max_spend_usd,
                "SWARM_MAX_SPEND_USD={bad} keeps the default"
            );
        }

        // And the two values that are documented to lift a cap still lift it.
        for lift in ["0", "off", "none"] {
            // SAFETY: as above.
            unsafe {
                std::env::set_var("SWARM_MAX_TURNS", lift);
                std::env::set_var("SWARM_MAX_SPEND_USD", lift);
            }
            let caps = Caps::configured();
            assert_eq!(caps.max_turns, None, "SWARM_MAX_TURNS={lift} lifts it");
            assert_eq!(
                caps.max_spend_usd, None,
                "SWARM_MAX_SPEND_USD={lift} lifts it"
            );
        }
    }
}

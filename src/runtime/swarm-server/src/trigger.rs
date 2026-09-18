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

use crate::budget::{Bound, Capped, Caps, Reached};
use crate::coordinator;
use crate::state::{CappedGoal, Server, Unit};
use crate::swarm::{Swarm, TurnPhase, What};

/// The view that says which goals are waiting for a turn.
///
/// Named here because the binding does not name it: the contract says the host supplies
/// `goal_id`, and which goals need one is the host's question to answer.
const AWAITING: &str = "swarm.goal.GoalsAwaitingATick";

/// The view that says what each member is meant to be doing now.
///
/// `assign.sh`'s `assigned` rows, as `agent.yaml` names them. Read rather than kept, for the same
/// reason `AWAITING` is: which members have work is the specification's fact, and a second list
/// here would be a second answer to it.
const OPEN_ASSIGNMENTS: &str = "swarm.agent.OpenAssignments";

/// The role slug a swarm's first agent holds.
pub const COORDINATOR: &str = "coordinator";

/// The variant of `swarm.agent.Role` that first agent holds.
///
/// Separate from the slug, and the distinction is the one `agent.yaml` makes about identity: the
/// slug is who an agent is and the role is what it does. They were the same string while the enum
/// had one variant, which is exactly why eleven event logs record four differently-named agents
/// all carrying `Coordinator`.
pub const COORDINATOR_ROLE: &str = "Coordinator";

/// The variant a member spawned to do work holds, and the display name it is given.
///
/// A worker is not a second coordinator: `agent.yaml`'s actors already say so — the Coordinator
/// commands and the Worker reports about itself — and until 2026-09-13 the enum could not.
pub const WORKER_ROLE: &str = "Worker";

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
            // And every member the coordinator has posted an assignment to. A second arrow of the
            // same shape, driven from the same period: a roster a coordinator builds and nothing
            // ever runs is a roster of records.
            work_the_assignments(&server, &swarm).await;
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
pub async fn ask_the_coordinator(server: &Arc<Server>, swarm: &Arc<Swarm>) {
    if !swarm.control.running() {
        return;
    }
    // The coordinator is a record before it is a process, so it can be addressed, drawn and
    // written to. Idempotent; a swarm made before this existed gets one here.
    if let Err(why) = swarm
        .ensure_agent(COORDINATOR, COORDINATOR_ROLE, "The coordinator")
        .await
    {
        tracing::warn!(swarm = %swarm.slug(), error = %why, "the coordinator could not be registered");
    }

    for instance in swarm.instances("swarm.goal.Goal").await {
        if instance.get("state").and_then(Json::as_str) != Some("Pursuing") {
            continue;
        }
        let Some(id) = instance.get("id").and_then(Json::as_str) else {
            continue;
        };
        let unit = Unit::goal(id);
        let Some(claim) = swarm
            .control
            .admit(format!("{}:{}", unit.kind(), unit.id()))
        else {
            continue;
        };
        // How wide this swarm may run at one moment, decided before anything is launched. A swarm
        // that is full does not refuse the goal — it takes it at the next period — so this breaks
        // rather than reporting anything: every remaining goal would meet the same full swarm.
        match server.claim_within(swarm.slug(), &unit, server.caps().max_in_flight) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(in_flight) => {
                tracing::debug!(swarm = %swarm.slug(), unit = id, in_flight,
                                ceiling = ?server.caps().max_in_flight,
                                "the swarm is at its breadth ceiling; the turn waits for a period \
                                 with room in it");
                break;
            }
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
        if let Some(bound) = bounded(
            server.caps(),
            swarm,
            COORDINATOR,
            id,
            iterations.saturating_sub(1),
        ) {
            // Reported from here as well as from `fire`, and that is not belt-and-braces: a
            // coordinator that never writes a verdict leaves its goal in `Pursuing`, which
            // `GoalsAwaitingATick` does not select, so `fire` never sees the goal again after the
            // first period and THIS is the only guard that ever trips. Stopping here in silence
            // left a goal sitting in `Pursuing` with nothing in `capped_goals()`, nothing on the
            // watch stream and no warning — which is precisely the picture the $11.35 run
            // presented to everybody who looked at it.
            report_capped(server, swarm, COORDINATOR, &unit, &bound);
            server.release(swarm.slug(), &unit);
            continue;
        }

        let server = Arc::clone(server);
        let swarm = Arc::clone(swarm);
        let id = id.to_owned();
        tokio::spawn(claim.scope(async move {
            one_turn(&swarm, COORDINATOR, &id, &goal, iterations).await;
            server.release(swarm.slug(), &unit);
        }));
    }
}

/// Runs every member that is holding an assignment nobody is working.
///
/// The second arrow. `swarm.agent.OpenAssignments` is the specification's own answer to "what is
/// each member meant to be doing now", so this reads it rather than keeping a list of its own.
///
/// Measured 2026-09-13, before this existed: across all 11 logs under
/// `data/swarms/*/eventlog.sqlite3`, `swarm.agent.AssignmentTaken` had fired **zero times, ever**.
/// A coordinator could spawn members and post them assignments, and nothing in the system would
/// ever run one.
///
/// The coordinator's own slug is skipped. It has a loop already — the goal — and a coordinator that
/// posted itself an assignment would be run twice a period, once at each.
pub async fn work_the_assignments(server: &Arc<Server>, swarm: &Arc<Swarm>) {
    if !swarm.control.running() {
        return;
    }
    let open = match swarm.view(OPEN_ASSIGNMENTS).await {
        Ok(rows) => rows,
        Err(why) => {
            tracing::warn!(swarm = %swarm.slug(), error = %why,
                           "the open assignments could not be read");
            return;
        }
    };

    for row in open {
        let Some(assignment) = coordinator::Assignment::from_row(&row) else {
            tracing::warn!(swarm = %swarm.slug(), row = %Json::Object(row),
                           "an assignment row is missing a field a member would be launched on");
            continue;
        };
        if assignment.agent_id == COORDINATOR {
            continue;
        }
        let unit = Unit::assignment(&assignment.assignment_id);
        let Some(claim) = swarm
            .control
            .admit(format!("{}:{}", unit.kind(), unit.id()))
        else {
            continue;
        };
        // The same ceiling, over the same count, and it is the reason this arrow is the one that
        // needed it: the coordinator has one goal at a time and a coordinator that posts ten
        // assignments gets ten tasks here, bounded by nothing until now. The coordinator's goals
        // are claimed first, one pass earlier in `run`, so a ceiling of one is spent on the
        // coordinator and the members wait — which is the ceiling choosing, not a defect.
        match server.claim_within(swarm.slug(), &unit, server.caps().max_in_flight) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(in_flight) => {
                tracing::debug!(swarm = %swarm.slug(), unit = %assignment.assignment_id, in_flight,
                                ceiling = ?server.caps().max_in_flight,
                                "the swarm is at its breadth ceiling; the assignment waits for a \
                                 period with room in it");
                break;
            }
        }

        // What the member has already spent, across every unit it has worked. There is no
        // `iterations` on an assignment — the specification counts turns for a goal and not for
        // this — so the member's own recorded attempts are both the turn number and the figure the
        // cap is measured on. They are the same number on purpose: a member that never answers
        // would otherwise sit at turn 1 for ever while its money went out, which is the defect
        // `spend_on` was given attempts to close.
        let (_, attempts) = swarm.spend_by_agent(&assignment.agent_id);
        if let Some(bound) = bounded(
            server.caps(),
            swarm,
            &assignment.agent_id,
            &assignment.assignment_id,
            attempts,
        ) {
            report_capped(server, swarm, &assignment.agent_id, &unit, &bound);
            server.release(swarm.slug(), &unit);
            continue;
        }

        let server = Arc::clone(server);
        let swarm = Arc::clone(swarm);
        tokio::spawn(claim.scope(async move {
            one_assignment(&swarm, &assignment, attempts + 1).await;
            server.release(swarm.slug(), &unit);
        }));
    }
}

/// One member's turn at one assignment, announced at both ends.
///
/// `TakeAssignment` is issued first, and only when the member has not already taken it: the event
/// is the record that this agent started on this work, and a session that ran without one is work
/// nobody can attribute. A member already `Working` is continuing, not starting again — and the
/// specification would refuse a second take through its own `wrong-state` branch rather than
/// quietly allowing it.
async fn one_assignment(swarm: &Arc<Swarm>, assignment: &coordinator::Assignment, turn: u64) {
    if !take_it(swarm, assignment).await {
        return;
    }

    swarm.announce(What::Turn {
        agent: assignment.agent_id.clone(),
        goal: assignment.assignment_id.clone(),
        iterations: turn,
        phase: TurnPhase::Asking,
        reached: None,
        note: None,
        error: None,
        took_ms: None,
        spent: None,
    });
    let began = Instant::now();

    match coordinator::work_an_assignment(swarm, assignment, turn).await {
        Ok(answer) => {
            tracing::info!(swarm = %swarm.slug(), agent = %assignment.agent_id,
                           assignment = %assignment.assignment_id,
                           reached = answer.verdict.reached, "a member reported");
            swarm.announce(What::Turn {
                agent: assignment.agent_id.clone(),
                goal: assignment.assignment_id.clone(),
                iterations: turn,
                phase: TurnPhase::Answered,
                reached: Some(answer.verdict.reached),
                note: answer.verdict.note,
                error: None,
                took_ms: Some(began.elapsed().as_millis() as u64),
                spent: Some(answer.spent),
            });
        }
        Err(why) => {
            tracing::warn!(swarm = %swarm.slug(), agent = %assignment.agent_id,
                           assignment = %assignment.assignment_id, error = %why,
                           "a member's turn could not be finished");
            if !matches!(why, coordinator::Unfinished::Unreportable { .. })
                && let Some(spent) = why.spent()
            {
                swarm.record_spend(
                    &assignment.agent_id,
                    &assignment.assignment_id,
                    turn,
                    spent,
                    None,
                    Some(&why.to_string()),
                );
            }
            swarm.announce(What::Turn {
                agent: assignment.agent_id.clone(),
                goal: assignment.assignment_id.clone(),
                iterations: turn,
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

/// Records that the member has started on its assignment, when it has not already.
///
/// `false` when the take was refused, and then no session is launched: a member whose own record
/// says it is not working this is not one the runtime may run anyway.
async fn take_it(swarm: &Arc<Swarm>, assignment: &coordinator::Assignment) -> bool {
    let state = swarm
        .instances("swarm.agent.Agent")
        .await
        .into_iter()
        .find(|agent| agent.get("id").and_then(Json::as_str) == Some(&assignment.agent_id))
        .and_then(|agent| {
            agent
                .get("state")
                .and_then(Json::as_str)
                .map(ToOwned::to_owned)
        });
    if state.as_deref() == Some("Working") {
        return true;
    }

    let mut input = Map::new();
    input.insert(
        "agent_id".to_owned(),
        Json::String(assignment.agent_id.clone()),
    );
    input.insert(
        "ref".to_owned(),
        assignment
            .artifact_ref
            .clone()
            .map_or(Json::Null, Json::String),
    );
    input.insert(
        "msg".to_owned(),
        Json::String(format!("started on: {}", assignment.outcome)),
    );
    match swarm
        .issue(
            Some("swarm.agent.Worker"),
            "swarm.agent.TakeAssignment",
            input,
            &format!("take:{}", assignment.assignment_id),
        )
        .await
    {
        Ok(issued) if issued.error.is_none() => true,
        Ok(issued) => {
            tracing::warn!(swarm = %swarm.slug(), agent = %assignment.agent_id,
                           outcome = %issued.outcome, error = ?issued.error,
                           "the member could not take its assignment, so it was not run");
            false
        }
        Err(why) => {
            tracing::warn!(swarm = %swarm.slug(), agent = %assignment.agent_id, error = %why,
                           "the member could not take its assignment, so it was not run");
            false
        }
    }
}

/// One coordinator turn, announced at both ends.
async fn one_turn(swarm: &Arc<Swarm>, actor: &str, id: &str, goal: &str, iterations: u64) {
    // Watchers hear the turn begin, so a coordinator that takes a minute is seen working
    // rather than seen as a loop that stalled.
    swarm.announce(What::Turn {
        agent: actor.to_owned(),
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
                agent: actor.to_owned(),
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
            if !matches!(why, coordinator::Unfinished::Unreportable { .. })
                && let Some(spent) = why.spent()
            {
                swarm.record_spend(actor, id, iterations, spent, None, Some(&why.to_string()));
            }
            swarm.announce(What::Turn {
                agent: actor.to_owned(),
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
        let within_budget = match bounded(server.caps(), swarm, COORDINATOR, goal, so_far) {
            None => true,
            Some(bound) => {
                report_capped(server, swarm, COORDINATOR, &Unit::goal(goal), &bound);
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
///
/// What is published are the figures of the bound that FIRED, not the goal's. It composed the two
/// from different folds until 2026-09-13 — `turns` and `spent_usd` from `spend_on(goal)`, `reached`
/// and `why` from whichever branch of [`bounded`] tripped — and once the agent bound existed those
/// two folds stopped agreeing: a goal that had taken 2 turns was published as `turns: 2,
/// spent_usd: null` beside `"the turn cap was reached: 4 of 3"`, telling every reader of `/status`
/// and of the watch stream to raise a cap that goal had used none of.
fn report_capped(server: &Server, swarm: &Arc<Swarm>, agent: &str, unit: &Unit, bound: &Capped) {
    let reached = bound.reached;
    let goal = unit.id();
    let capped = CappedGoal {
        swarm: swarm.slug().to_owned(),
        goal: goal.to_owned(),
        unit: unit.kind().to_owned(),
        agent: agent.to_owned(),
        turns: bound.turns,
        spent_usd: bound.spent_usd,
        reached,
        why: bound.why(),
    };
    if server.report_capped(unit, capped.clone()) {
        tracing::warn!(swarm = %swarm.slug(), unit = goal, kind = unit.kind(), agent,
                       why = %capped.why, "the loop stopped asking");
        // And durably, beside the spend the bound was measured on. The two lines below reach a
        // watcher and a log; a swarm whose run is over has neither, and "was anything ever
        // refused here" is the first question its records are asked.
        swarm.record_capped(&crate::swarm::CappedRecord {
            agent,
            unit: goal,
            bound: match &bound.bound {
                Bound::Goal => "unit",
                Bound::Agent(_) => "agent",
                Bound::Swarm => "swarm",
            },
            turns: bound.turns,
            spent_usd: bound.spent_usd,
            // `Capped::variable`, not `Reached::variable`: the swarm bound and the per-agent bound
            // are both a `Reached::Spend`, and the measurement alone cannot say which number it was
            // compared against. A record that named the wrong one is a record that sends its reader
            // to raise a cap nothing reached.
            cap: bound.variable(),
            why: &capped.why,
        });
        swarm.announce(What::Capped {
            agent: capped.agent,
            unit: capped.unit,
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
/// The larger of the two is used ON THE GOAL BOUND ONLY, because a swarm whose records were
/// archived should not have its cap reset by the absence. The agent bound below reads the agent's
/// recorded attempts alone, and has no equivalent defence: there is no `iterations` figure kept
/// per agent to compare it against, and the goal's is not one — it counts a different subject.
///
/// So "the same caps apply to both" is true of the CAPS and not of that defence, and an earlier
/// revision of this comment claimed otherwise. Nothing found reaches it either way: there is no
/// archive, rotate or prune path for `spend.jsonl` anywhere in the tree (searched 2026-09-13), so
/// the only way an agent's rows disappear is somebody deleting the file by hand. If one is ever
/// added, this is the line it has to answer.
///
/// Two bounds, not one, and the second is the one a goal cannot get round. A cap keyed on a goal
/// binds a swarm to the number of goals it has: one agent at $1.00 a turn over two goals reaches
/// $6.00 against a $5.00 cap with each goal holding $3.00, and every per-goal fold says it is
/// inside its bounds. So the agent's whole record is read as well, keyed on nothing but the agent,
/// and the same caps apply to it. An agent is bounded whichever goals it spreads its turns over —
/// which means a goal that has used up NOTHING of its own can be refused here, and that is the
/// bound working rather than failing.
///
/// The caps are the same numbers deliberately: an operator who set `SWARM_MAX_SPEND_USD=5` meant
/// five dollars, and a ceiling that let the same agent spend five per goal would be answering a
/// question nobody asked it. Lifting a cap still lifts both, because `None` exceeds nothing.
///
/// Three bounds since `story:run-two-workers-at-once`, and the third is the one an AGENT cannot get
/// round. The argument that carried the second one level up carries again: a bound keyed on the
/// agent binds a swarm to the number of agents in it, so four members at $2.00 each reach $8.00
/// against a $5.00 cap with every per-agent fold reporting itself inside its bounds. That was
/// arithmetic about a swarm nobody could run until this wave; a swarm that runs its members at once
/// is what makes it a bill. So the swarm's whole record is read as well — `Swarm::spend()`, which
/// asks neither which unit nor which agent — against `SWARM_MAX_TOTAL_SPEND_USD`.
///
/// It is read LAST, and that ordering is what a reader is told to do about it. A unit over its own
/// bound and an agent over its own are each actionable by somebody; a swarm over its total is only
/// actionable by whoever owns the swarm, and reporting it while a narrower bound also holds would
/// name the widest cap to a reader who could have raised the narrowest. When the narrower two are
/// inside their bounds it is the only bound that can stop the run, and then it is the one reported.
pub fn bounded(
    caps: Caps,
    swarm: &Arc<Swarm>,
    agent: &str,
    goal_id: &str,
    iterations: u64,
) -> Option<Capped> {
    let (unit_spent, unit_attempts) = swarm.spend_on(goal_id);
    let turns = iterations.max(unit_attempts);
    let unit = caps.exceeded(turns, unit_spent.cost_usd);
    // The same caps, read off a figure that names no unit. Nothing here asks which unit the agent
    // is working, which is the whole point of it.
    let (agent_spent, agent_attempts) = swarm.spend_by_agent(agent);
    let whole_agent = caps.exceeded(agent_attempts, agent_spent.cost_usd);

    // WHICH bound is reported, when both hold, is not a detail: it is what a reader is told to
    // raise, and `Capped`'s own doc records a wave spent on getting it wrong.
    //
    // The unit bound is reported when the unit's OWN RECORD carries it — not when `iterations`
    // alone does. `iterations` is the caller's figure FOR THIS UNIT, and it exists so that a goal
    // whose spend rows were archived is not handed its cap back by their absence. A caller that
    // passes a figure belonging to something else would otherwise have the refusal filed against a
    // unit that has used nothing of it, which is exactly what `work_the_assignments` did until
    // correction round 2: it passed the AGENT's whole record, so a member retasked to a second
    // assignment was refused at its first turn there and `turns/capped.jsonl` told a reader to
    // raise a cap that assignment had never touched.
    //
    // So: the unit, when the unit's own rows carry it, or when nothing else does — the archive
    // defence, which is the only thing `iterations` is for. Otherwise the agent, which is the
    // record that actually holds.
    let unit_alone = caps.exceeded(unit_attempts, unit_spent.cost_usd).is_some();
    if let Some(reached) = unit
        && (unit_alone || whole_agent.is_none())
    {
        return Some(Capped {
            bound: Bound::Goal,
            reached,
            turns,
            spent_usd: unit_spent.cost_usd,
        });
    }
    // The figures that travel with the verdict are this fold's, because the unit's no longer
    // describe what was measured.
    if let Some(reached) = whole_agent {
        return Some(Capped {
            bound: Bound::Agent(agent.to_owned()),
            reached,
            turns: agent_attempts,
            spent_usd: agent_spent.cost_usd,
        });
    }
    // And the swarm's, which is every row of the file and asks nothing about who wrote it. Read
    // here and not earlier so that the narrowest bound that holds is the one a reader is sent to,
    // and not at all when there is no cap to read it against — which is what the bare `?` below
    // says: no cap, no fold, no verdict. This is a THIRD full pass over `spend.jsonl` in one call,
    // and an operator who lifted the cap should not be charged for it.
    caps.max_total_spend_usd?;
    let (swarm_spent, swarm_attempts) = swarm.spend();
    caps.swarm_exceeded(swarm_spent.cost_usd)
        .map(|reached| Capped {
            bound: Bound::Swarm,
            reached,
            turns: swarm_attempts,
            spent_usd: swarm_spent.cost_usd,
        })
}

/// The verdict alone, for a caller that wants only whether this goal may take another turn.
pub fn capped(
    caps: Caps,
    swarm: &Arc<Swarm>,
    agent: &str,
    goal_id: &str,
    iterations: u64,
) -> Option<Reached> {
    bounded(caps, swarm, agent, goal_id, iterations).map(|capped| capped.reached)
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

    /// The agent bound, driven through the real loop — the coverage the `agent` argument had none of.
    ///
    /// Every other loop-driving case builds a swarm with ONE goal, where `capped`'s per-goal branch
    /// returns first and the agent branch is never the one that fires. Replace `COORDINATOR` with
    /// any other string at either call site and none of them goes red. This one does: two goals,
    /// a cap of three turns, and a coordinator that never finishes either. Neither goal reaches
    /// three on its own, so the only bound that can stop this swarm is the one read off the
    /// agent's whole record.
    ///
    /// It also drives `report_capped`: what a capped goal PUBLISHES must be the figures of the
    /// bound that actually fired. Before this case, a goal stopped by the agent bound was published
    /// with its own turn count — 1 or 2 — beside `why` reading "the turn cap was reached: 3 of 3",
    /// which tells a reader to raise a cap that goal used none of.
    #[tokio::test]
    async fn an_agent_is_stopped_by_its_own_record_across_two_goals() {
        let _guard = crate::ENVIRONMENT.lock().await;
        let data = tempdir::TempDir::new("swarm-two-goals").expect("a scratch directory");
        let program = never_reached(&data.path().join("coordinator.sh"));
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_TURNS", "3");
            std::env::set_var("SWARM_MAX_SPEND_USD", "off");
            std::env::set_var("SWARM_COORDINATOR", &program);
        }

        let (server, swarm, first) = a_swarm_with_a_goal(&data, "two-goals").await;
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        swarm
            .issue(
                None,
                "swarm.goal.SetGoal",
                json!({"swarm_id": swarm_id, "text": "a second goal, equally unreachable"})
                    .as_object()
                    .expect("an object")
                    .clone(),
                "second-goal",
            )
            .await
            .expect("the command applies");
        let goals: Vec<String> = swarm
            .instances("swarm.goal.Goal")
            .await
            .iter()
            .filter_map(|goal| goal["id"].as_str().map(ToOwned::to_owned))
            .collect();
        assert_eq!(goals.len(), 2, "the swarm has two goals: {goals:?}");
        assert!(goals.contains(&first));

        for _ in 0..12 {
            one_period(&server, &swarm).await;
        }

        let per_goal: Vec<u64> = goals.iter().map(|goal| swarm.spend_on(goal).1).collect();
        let (agent_spent, agent_turns) = swarm.spend_by_agent(COORDINATOR);
        assert_eq!(
            agent_turns,
            per_goal.iter().sum::<u64>(),
            "the agent's record is every row of both goals"
        );

        // The bound that fired is the agent's. Neither goal reached three on its own, so a cap
        // keyed on a goal would have let this swarm run for ever.
        for (goal, turns) in goals.iter().zip(&per_goal) {
            assert!(
                *turns < 3,
                "goal {goal} took {turns} turns, so the per-goal bound is what stopped it and this \
                 case has stopped measuring the agent bound: {per_goal:?}"
            );
        }
        assert!(
            (3..=4).contains(&agent_turns),
            "a cap of three turns stops the agent at three, or at four when both goals were \
             already in flight when it tripped: {agent_turns}"
        );

        // The guard in `fire` as well as the one in `ask_the_coordinator`. `fire` applies the
        // `Pursue` for the turn about to be taken, so a goal whose occurrences keep being issued
        // keeps advancing its own `iterations` even when no turn runs — twelve periods of it here.
        // Without this, the agent argument at `fire`'s call site has no coverage: break it and
        // the other guard still stops the turns, and every assertion above still passes.
        for (goal, turns) in goals.iter().zip(&per_goal) {
            let iterations = swarm
                .instances("swarm.goal.Goal")
                .await
                .iter()
                .find(|instance| instance["id"].as_str() == Some(goal.as_str()))
                .and_then(|instance| instance["fields"]["iterations"].as_u64())
                .expect("the goal counts its own turns");
            assert_eq!(
                iterations, *turns,
                "goal {goal} went no further than the turns it took: the loop stopped ASKING, so \
                 its own count stopped too"
            );
        }

        // What a reader is told. The figures published must be the ones the bound was measured on.
        let reported = server.capped_goals();
        assert!(!reported.is_empty(), "the loop said why it stopped");
        for capped in &reported {
            assert_eq!(
                capped.turns, agent_turns,
                "a goal stopped by the agent bound publishes the agent's figure, not its own: \
                 {capped:?}"
            );
            assert_eq!(
                capped.spent_usd, agent_spent.cost_usd,
                "and the agent's spend, not the goal's: {capped:?}"
            );
            assert!(
                capped.why.contains(COORDINATOR),
                "and says whose record it was measured on: {}",
                capped.why
            );
        }
    }

    /// The same bound at the OTHER guard, which is the only one a stuck goal ever reaches.
    ///
    /// A coordinator that never writes a verdict leaves its goal in `Pursuing`, and
    /// `GoalsAwaitingATick` does not select those — so `fire` never sees either goal again after
    /// the first period and `ask_the_coordinator`'s guard is the only thing that can stop this
    /// swarm. That makes it the case that covers the `agent` argument at `trigger.rs:146`:
    /// replace `COORDINATOR` there with any other string and the agent's record reads zero, the
    /// bound never trips, and twelve periods take twenty-four attempts instead of four.
    ///
    /// Two cases, not one, because the two call sites are reached by different shapes of failure
    /// and a mutation at either survives the other's case.
    #[tokio::test]
    async fn a_stuck_agent_is_stopped_across_two_goals_by_the_other_guard() {
        let _guard = crate::ENVIRONMENT.lock().await;
        let data = tempdir::TempDir::new("swarm-two-goals-stuck").expect("a scratch directory");
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

        let (server, swarm, _first) = a_swarm_with_a_goal(&data, "two-goals-stuck").await;
        let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
            .as_str()
            .expect("an identity")
            .to_owned();
        swarm
            .issue(
                None,
                "swarm.goal.SetGoal",
                json!({"swarm_id": swarm_id, "text": "a second goal nobody answers"})
                    .as_object()
                    .expect("an object")
                    .clone(),
                "second-stuck-goal",
            )
            .await
            .expect("the command applies");
        let goals: Vec<String> = swarm
            .instances("swarm.goal.Goal")
            .await
            .iter()
            .filter_map(|goal| goal["id"].as_str().map(ToOwned::to_owned))
            .collect();
        assert_eq!(goals.len(), 2, "the swarm has two goals: {goals:?}");

        for _ in 0..12 {
            one_period(&server, &swarm).await;
        }

        let per_goal: Vec<u64> = goals.iter().map(|goal| swarm.spend_on(goal).1).collect();
        let (_, agent_turns) = swarm.spend_by_agent(COORDINATOR);
        for (goal, turns) in goals.iter().zip(&per_goal) {
            assert!(
                *turns < 3,
                "goal {goal} took {turns} of its own turns, so the per-goal bound is what stopped \
                 this and the case has stopped measuring the agent bound: {per_goal:?}"
            );
        }
        assert!(
            (3..=4).contains(&agent_turns),
            "a cap of three turns stops the agent at three, or at four when both goals were \
             already in flight when it tripped: {agent_turns} attempts over twelve periods"
        );
        assert!(
            server.capped_goals().iter().any(|c| c.turns == agent_turns),
            "and the goal is reported capped on the figure the bound was measured on: {:?}",
            server.capped_goals()
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
    /// thing only because `u64` cannot parse it. Every name `Caps::configured` reads is listed
    /// here, and a cap added to `Caps` without being added to this list is the next instance of the
    /// same defect — which is why the assertions below read the whole struct rather than one field.
    #[tokio::test]
    async fn no_cap_is_lifted_by_a_value_that_is_merely_wrong() {
        let _guard = crate::ENVIRONMENT.lock().await;
        // The four variables `Caps::configured` reads. The two swarm-wide ones arrived with
        // `story:run-two-workers-at-once`, and arrived in this list in the same breath.
        const EVERY: [&str; 4] = [
            "SWARM_MAX_TURNS",
            "SWARM_MAX_SPEND_USD",
            "SWARM_MAX_IN_FLIGHT",
            "SWARM_MAX_TOTAL_SPEND_USD",
        ];
        for bad in ["-1", "-0.01", "nonsense"] {
            // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT`.
            unsafe {
                for name in EVERY {
                    std::env::set_var(name, bad);
                }
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
            assert_eq!(
                caps.max_in_flight,
                Caps::default().max_in_flight,
                "SWARM_MAX_IN_FLIGHT={bad} keeps the default"
            );
            assert_eq!(
                caps.max_total_spend_usd,
                Caps::default().max_total_spend_usd,
                "SWARM_MAX_TOTAL_SPEND_USD={bad} keeps the default"
            );
        }

        // And the two values that are documented to lift a cap still lift it.
        for lift in ["0", "off", "none"] {
            // SAFETY: as above.
            unsafe {
                for name in EVERY {
                    std::env::set_var(name, lift);
                }
            }
            let caps = Caps::configured();
            assert_eq!(caps.max_turns, None, "SWARM_MAX_TURNS={lift} lifts it");
            assert_eq!(
                caps.max_spend_usd, None,
                "SWARM_MAX_SPEND_USD={lift} lifts it"
            );
            assert_eq!(
                caps.max_in_flight, None,
                "SWARM_MAX_IN_FLIGHT={lift} lifts it"
            );
            assert_eq!(
                caps.max_total_spend_usd, None,
                "SWARM_MAX_TOTAL_SPEND_USD={lift} lifts it"
            );
        }

        // SAFETY: as above. The swarm-wide caps are left where every other case expects them.
        unsafe {
            std::env::remove_var("SWARM_MAX_IN_FLIGHT");
            std::env::remove_var("SWARM_MAX_TOTAL_SPEND_USD");
        }
    }

    /// A swarm no agent of which is over anything, stopped by the bound that is the swarm's own.
    ///
    /// The planted rows are four members at $2.00 each: under the $5.00 per-agent cap, on four
    /// units none of which is near it, and not one of them the coordinator. Every fold that existed
    /// before this wave says this swarm may keep going, and $8.00 has gone out of it.
    ///
    /// Driven through the real loop, so the refusal is `fire`'s own guard and the record is the one
    /// `report_capped` writes. Delete the `swarm_exceeded` branch in [`bounded`] and this goes red
    /// three ways: the coordinator takes turns, the swarm's total moves, and `capped.jsonl` is
    /// never written at all.
    #[tokio::test]
    async fn a_swarm_over_its_total_is_stopped_with_the_bound_that_fired_named() {
        let _guard = crate::ENVIRONMENT.lock().await;
        let data = tempdir::TempDir::new("swarm-total").expect("a scratch directory");
        let program = never_reached(&data.path().join("coordinator.sh"));
        // SAFETY: every case in this crate that reads the environment holds `ENVIRONMENT` first.
        unsafe {
            std::env::set_var("SWARM_MAX_TURNS", "off");
            std::env::set_var("SWARM_MAX_SPEND_USD", "5");
            std::env::set_var("SWARM_MAX_TOTAL_SPEND_USD", "5");
            std::env::set_var("SWARM_MAX_IN_FLIGHT", "off");
            std::env::set_var("SWARM_COORDINATOR", &program);
        }

        let (server, swarm, goal) = a_swarm_with_a_goal(&data, "over-its-total").await;
        let members = ["worker-a", "worker-b", "worker-c", "worker-d"];
        let mut two_dollars = crate::coordinator::Spent::default();
        two_dollars.cost_usd = Some(2.00);
        for member in members {
            swarm.record_spend(
                member,
                &format!("assignment-for-{member}"),
                1,
                &two_dollars,
                Some(false),
                None,
            );
        }
        let planted = swarm.spend().0.cost_usd;
        assert_eq!(planted, Some(8.00), "four members at $2.00 each");
        for member in members {
            let (spent, turns) = swarm.spend_by_agent(member);
            assert!(
                server.caps().exceeded(turns, spent.cost_usd).is_none(),
                "{member} is inside every bound that is its own, which is what makes this case \
                 about the swarm's"
            );
        }

        for _ in 0..3 {
            one_period(&server, &swarm).await;
        }

        // Nothing was launched, so nothing was spent: the check precedes the turn, and a cap that
        // fires once the money is gone has stopped nothing.
        assert_eq!(
            swarm.spend_on(&goal).1,
            0,
            "the goal took no turn under a swarm that is over its total"
        );
        assert_eq!(
            swarm.spend_by_agent(COORDINATOR).1,
            0,
            "and the coordinator never ran"
        );
        assert_eq!(
            swarm.spend().0.cost_usd,
            planted,
            "the refusal cost $0.00: the swarm's total is what it was planted at"
        );

        // What the retained record says. `capped.jsonl` outlives the process, the watch stream and
        // the log line, and it is where "was anything ever refused here" is answered.
        let text = std::fs::read_to_string(swarm.dir().join("turns").join("capped.jsonl"))
            .expect("the bound that fired was written down");
        let rows: Vec<Json> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("a row is JSON"))
            .collect();
        assert_eq!(rows.len(), 1, "written once, not once a period: {rows:?}");
        let row = &rows[0];
        // The retained row, for a run that is recorded rather than summarised.
        println!("the bound that fired, from turns/capped.jsonl: {row}");
        assert_eq!(row["bound"], json!("swarm"), "the swarm's own fold: {row}");
        assert_eq!(
            row["cap"],
            json!("SWARM_MAX_TOTAL_SPEND_USD"),
            "named by the variable that moves it, and not by the per-agent cap nothing reached: \
             {row}"
        );
        assert_eq!(
            row["spent_usd"],
            json!(8.0),
            "on the figure it fired on: {row}"
        );
        assert!(
            row["why"].as_str().is_some_and(|why| {
                why.contains("SWARM_MAX_TOTAL_SPEND_USD") && why.contains("$8.00 of $5.00")
            }),
            "and the sentence a person reads says both: {row}"
        );

        // The same bound, as a reader of `/status` and of the watch stream sees it.
        let reported = server.capped_goals();
        assert!(
            reported.iter().any(|capped| {
                capped.why.contains("SWARM_MAX_TOTAL_SPEND_USD") && capped.spent_usd == Some(8.00)
            }),
            "the loop said which bound stopped it: {reported:?}"
        );

        // SAFETY: as above.
        unsafe {
            std::env::remove_var("SWARM_MAX_TOTAL_SPEND_USD");
            std::env::remove_var("SWARM_MAX_IN_FLIGHT");
        }
    }
}

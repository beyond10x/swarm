//! `story:spawn-a-second-agent`, clauses 3, 4 and 5, driven through the real runtime.
//!
//! Measured across all 11 logs under `data/swarms/*/eventlog.sqlite3` on 2026-09-13, before any of
//! this existed: `AssignmentPosted` had fired once in the history of this repository,
//! `AssignmentRecorded` once, and `AssignmentTaken`, `StepDone`, `GateGreen`, `GateRed`,
//! `FinishAssignment`, `AgentBlocked` and `DecisionParked` **zero times, ever**. The working half of
//! `agent.yaml` had never run. This file is what makes it run.
//!
//! No money is spent here. The member's session is a `Launch::Program` — the same stdin→stdout
//! contract `examples/coordinator-manual.sh` satisfies — resolved from `SWARM_COORDINATOR`, so the
//! path under test is the runtime's: which agent is asked, what it is asked about, what is recorded
//! and what is issued. What a real `metaharness run claude` does with a frame is measured in
//! `story:a-turn-is-confined-by-a-frame`, twice, for $0.224.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::coordinator;
use swarm_server::state::{Server, Unit};
use swarm_server::swarm::Swarm;
use swarm_server::trigger;

/// The process environment is one object and one case here sets it. Held for the whole of that
/// case, on the convention `the_turn_cap_under_attack_pass_2.rs` sets.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A member's session, satisfying the stdin→stdout contract and costing nothing.
///
/// It streams one refused tool call before its verdict, so the transcript is not empty and the
/// refusal has something to reach.
fn a_session(at: &std::path::Path, reached: bool) -> String {
    std::fs::write(
        at,
        format!(
            "#!/bin/sh\ncat > /dev/null\n\
             printf '{{\"event\":\"tool.requested\",\"call_id\":\"t1\",\"tool_name\":\"Bash\"}}\\n'\n\
             printf '{{\"event\":\"tool.decided\",\"call_id\":\"t1\",\"decided_by\":\"frame\",\
             \"seam\":\"hook\",\"decision\":{{\"decision\":\"deny\",\"reason\":\
             \"this step admits [file.read] and Bash is [shell], which it does not\"}}}}\\n'\n\
             printf '{{\"reached\": {reached}, \"note\": \"the member reported\"}}\\n'\n"
        ),
    )
    .expect("the session program is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755))
        .expect("the session program is runnable");
    at.display().to_string()
}

/// A started swarm, served by a real [`Server`], with a coordinator and one member holding an
/// assignment.
async fn a_swarm_with_a_member(
    data: &tempdir::TempDir,
    slug: &str,
) -> (Arc<Server>, Arc<Swarm>, String, String) {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Arc::new(
        Server::start(spec, data.path().to_path_buf())
            .await
            .expect("the server starts"),
    );
    let swarm = server.open(slug).await.expect("a swarm opens");

    let issue = async |actor: Option<&str>, command: &str, input: Json| {
        swarm
            .issue(
                actor,
                command,
                input.as_object().expect("an object").clone(),
                &format!("{command}:{}", input),
            )
            .await
            .expect("the command applies")
    };
    issue(
        None,
        "swarm.manager.CreateSwarm",
        json!({"display_name": slug, "tmux_session": slug, "home": "/nowhere",
               "created_at": "2026-09-13T10:00:00Z"}),
    )
    .await;
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        None,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": swarm_id, "started_at": "2026-09-13T10:01:00Z"}),
    )
    .await;
    issue(
        None,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "two agents, working"}),
    )
    .await;
    let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    swarm
        .ensure_agent(
            trigger::COORDINATOR,
            trigger::COORDINATOR_ROLE,
            "The coordinator",
        )
        .await
        .expect("the coordinator is a record");
    swarm
        .ensure_agent("builder", trigger::WORKER_ROLE, "A member that builds")
        .await
        .expect("the member is a record");

    // What a coordinator does with `swarm do`: one outcome, the sign-off it runs under, and what it
    // may not do (`swarm/AGENTS.md` §4 step 4). The binding on `AssignmentPosted` records it.
    issue(
        Some("swarm.agent.Coordinator"),
        "swarm.agent.Assign",
        json!({"agent_id": "builder", "outcome": "make the suite green",
               "signoff": "Timo, verbatim", "forbidden": "do not touch the planning store",
               "artifact_ref": "story:spawn-a-second-agent"}),
    )
    .await;

    let assignment = swarm.instances("swarm.agent.Assignment").await[0]["id"]
        .as_str()
        .expect("the binding recorded the assignment")
        .to_owned();
    (server, swarm, goal, assignment)
}

/// Waits for every turn the trigger spawned.
async fn settle(server: &Arc<Server>) {
    for _ in 0..600 {
        if server.turns_in_flight() == 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("a turn never finished");
}

/// Clause 3: the runtime runs the member as its own session against an assignment, and
/// `AssignmentTaken` fires with that agent as its subject.
///
/// The event has never been emitted in this repository. Everything else here is downstream of it:
/// a session that ran without a `take` is work nobody can attribute, and the lifecycle the domain
/// declares (`Assigned -> take -> Working`) would never have left its first state.
#[tokio::test]
async fn the_runtime_runs_a_member_and_assignment_taken_fires() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-second-agent").expect("a scratch directory");
    let program = a_session(&data.path().join("session.sh"), true);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };
    let (server, swarm, _goal, assignment) = a_swarm_with_a_member(&data, "second-agent").await;

    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;

    let log = swarm.history(500).await.expect("the log reads back");
    let taken: Vec<_> = log
        .iter()
        .filter(|event| event.name == "swarm.agent.AssignmentTaken")
        .collect();
    assert_eq!(
        taken.len(),
        1,
        "AssignmentTaken fires once for the assignment that was taken: {:?}",
        log.iter().map(|event| &event.name).collect::<Vec<_>>()
    );
    assert_eq!(
        taken[0].id, "builder",
        "and its subject is the member, not the coordinator"
    );
    assert_eq!(taken[0].fields["agent_id"], "builder");

    let agents: Vec<(String, String, String)> = swarm
        .instances("swarm.agent.Agent")
        .await
        .into_iter()
        .map(|agent| {
            (
                agent["id"].as_str().unwrap_or_default().to_owned(),
                agent["state"].as_str().unwrap_or_default().to_owned(),
                agent["fields"]["role"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect();
    // `Spawned -> assign -> Assigned -> take -> Working -> go_idle -> Idle`, all four transitions
    // in one period. It ends Idle because this member said it was finished: `Assign` runs from
    // `Spawned` or `Idle` only, so a member left Working after finishing could never be retasked,
    // and "retasking a member is a new post, not an edit" would be unreachable for every member
    // that ever completed anything.
    assert!(
        agents.contains(&("builder".to_owned(), "Idle".to_owned(), "Worker".to_owned())),
        "the member finished what it took, in a role that is not the coordinator's: {agents:?}"
    );
    let order: Vec<&str> = log
        .iter()
        .map(|event| event.name.as_str())
        .filter(|name| {
            matches!(
                *name,
                "swarm.agent.AssignmentTaken"
                    | "swarm.agent.AssignmentDone"
                    | "swarm.agent.AgentIdle"
            )
        })
        .collect();
    assert_eq!(
        order,
        vec![
            "swarm.agent.AssignmentTaken",
            "swarm.agent.AssignmentDone",
            "swarm.agent.AgentIdle"
        ],
        "and it went through Working to get there: taken, done, idle, in that order"
    );

    // The turn was the member's: its own spend row, and its own session.
    let (spent, attempts) = swarm.spend_by_agent("builder");
    assert_eq!(attempts, 1, "one attempt, filed under the member");
    assert_eq!(spent.refused, 1, "and the refusal it met is in the figures");

    // A reached verdict finishes the assignment, so the next period does not take it again.
    let assignments: Vec<(String, String)> = swarm
        .instances("swarm.agent.Assignment")
        .await
        .into_iter()
        .map(|record| {
            (
                record["id"].as_str().unwrap_or_default().to_owned(),
                record["state"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert!(
        assignments.contains(&(assignment.clone(), "Done".to_owned())),
        "a member that says it is finished finishes the record: {assignments:?}"
    );
}

/// Clause 5: two agents on one unit do not overwrite each other's transcript.
///
/// `turn_file()` carried the turn number, the attempt and the first eight characters of the goal,
/// and nothing that said who ran it — so a second agent's turn would have been written to the
/// first's name. The agent is in the name now, and the case reads two files rather than trusting
/// the format string.
#[tokio::test]
async fn two_agents_on_one_unit_write_two_transcripts() {
    let data = tempdir::TempDir::new("swarm-two-transcripts").expect("a scratch directory");
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));
    let swarm = Swarm::open(spec, data.path(), "transcripts")
        .await
        .expect("a swarm opens");
    let unit = "77fc1fcc-a89c-4fb9-b041-89ecfc75922f";

    let coordinator = coordinator::turn_file(&swarm, "coordinator", unit, 4);
    let builder = coordinator::turn_file(&swarm, "builder", unit, 4);
    assert_ne!(
        coordinator, builder,
        "the same unit, the same turn, two agents: two names"
    );
    for at in [&coordinator, &builder] {
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory is made");
        std::fs::write(at, "{}\n").expect("a transcript is written");
    }
    let written: Vec<String> = std::fs::read_dir(swarm.dir().join("turns"))
        .expect("the directory exists")
        .filter_map(|entry| Some(entry.ok()?.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_eq!(
        written.len(),
        2,
        "and two files on disk, neither having replaced the other: {written:?}"
    );
    assert!(
        written.iter().any(|name| name.contains("builder")),
        "a reader can tell whose a transcript is from its name alone: {written:?}"
    );

    // The attempt still counts per agent, so a member's retry does not overwrite its own record
    // either.
    let retry = coordinator::turn_file(&swarm, "builder", unit, 4);
    assert_ne!(retry, builder, "a retry is a new attempt: {retry:?}");
}

/// Clause 4: the in-flight claim is keyed on the unit of work, not on the goal.
///
/// `claim_turn(slug, goal_id)` meant one claim per goal per swarm, so a member working an
/// assignment would have waited on the coordinator's turn at the goal, and two members on one goal
/// would have run one at a time. `overlap: serial_per_instance` is a bound per INSTANCE, and an
/// assignment is a different instance from a goal.
#[tokio::test]
async fn two_units_of_work_are_two_claims() {
    let data = tempdir::TempDir::new("swarm-claims").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");

    let goal = Unit::goal("77fc1fcc-a89c-4fb9-b041-89ecfc75922f");
    let one = Unit::assignment("aaaaaaaa-0000-0000-0000-000000000001");
    let two = Unit::assignment("aaaaaaaa-0000-0000-0000-000000000002");

    assert!(server.claim("s", &goal), "the coordinator claims the goal");
    assert!(
        server.claim("s", &one),
        "and a member's assignment is not the goal: it is claimable while the goal is claimed"
    );
    assert!(server.claim("s", &two), "as is a second member's");
    assert_eq!(server.turns_in_flight(), 3, "three turns, three claims");

    assert!(
        !server.claim("s", &one),
        "the same unit twice is still refused — serial per instance is the bound that is kept"
    );
    server.release("s", &one);
    assert!(
        server.claim("s", &one),
        "and released when its turn is over, whichever way it went"
    );

    // The slug is part of the key, so two swarms working identically named units do not collide.
    assert!(server.claim("other", &one));
}

/// A bound that fires leaves a record a reader finds after the run, not only a line on a stream
/// nobody was listening to.
///
/// `report_capped` published to the watch stream and to the tracing log, and both are gone the
/// moment the process is. So a finished swarm's own records could not show that a cap had ever
/// refused anything — which is what the demonstration story needs to report, and what the $11.35
/// run could not have been asked about afterwards either.
///
/// The member's session never says it is finished, so the assignment stays open and the second
/// period has something real to refuse. A case that ran out of work instead would assert the same
/// absence of a second turn for the wrong reason entirely.
#[tokio::test]
async fn a_cap_that_refuses_a_turn_is_recorded_where_a_reader_finds_it() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-capped-record").expect("a scratch directory");
    let program = a_session(&data.path().join("session.sh"), false);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var("SWARM_COORDINATOR", &program);
        std::env::set_var("SWARM_MAX_TURNS", "1");
        std::env::set_var("SWARM_MAX_SPEND_USD", "off");
    }
    let (server, swarm, _goal, assignment) = a_swarm_with_a_member(&data, "capped-record").await;

    // One turn taken, which is the whole of the cap. The member did not say it was finished, so
    // the assignment is still open and a second period would run it again.
    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;
    assert_eq!(swarm.spend_by_agent("builder").1, 1, "one turn, the cap");
    let still_open: Vec<String> = swarm
        .view("swarm.agent.OpenAssignments")
        .await
        .expect("the view computes")
        .into_iter()
        .filter_map(|row| Some(row.get("assignment_id")?.as_str()?.to_owned()))
        .collect();
    assert_eq!(
        still_open,
        vec![assignment.clone()],
        "an unfinished assignment stays open, so the cap has something to refuse"
    );

    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;
    // SAFETY: as above.
    unsafe {
        std::env::remove_var("SWARM_MAX_TURNS");
        std::env::remove_var("SWARM_MAX_SPEND_USD");
    }

    assert_eq!(
        swarm.spend_by_agent("builder").1,
        1,
        "the second turn was refused by the cap, so no second attempt was spent"
    );

    let rows: Vec<Json> = std::fs::read_to_string(swarm.dir().join("turns").join("capped.jsonl"))
        .expect("a bound that fired left a record")
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(rows.len(), 1, "one bound fired, one row: {rows:?}");
    assert_eq!(rows[0]["agent"], "builder", "whose turn was refused");
    assert_eq!(rows[0]["unit"], assignment, "and which unit it refused");
    assert_eq!(rows[0]["turns"], 1);
    assert_eq!(rows[0]["cap"], "SWARM_MAX_TURNS", "and which cap to raise");
    assert!(
        rows[0]["why"]
            .as_str()
            .expect("a sentence a person reads")
            .contains("cap was reached"),
        "and what it says, in the words a reader is given: {}",
        rows[0]
    );
}

/// Two members of one swarm keep two work directories, not one.
///
/// Every agent was given `data/swarms/<slug>/work` and both prompts tell an agent to keep a
/// `NOTES.md` in its work directory and to read it first — so two members of one swarm wrote over
/// each other's memory of what they had done, in a wave whose entire point is that there are two
/// members. Clause 5 separated the transcripts; nothing separated the directories the work happens
/// in. Found by the adversary of correction round 1.
#[tokio::test]
async fn two_members_of_one_swarm_do_not_share_one_work_directory() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-two-desks").expect("a scratch directory");
    let program = a_session(&data.path().join("session.sh"), false);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };
    let (server, swarm, _goal, _assignment) = a_swarm_with_a_member(&data, "two-desks").await;

    swarm
        .ensure_agent("tester", trigger::WORKER_ROLE, "A member that tests")
        .await
        .expect("the second member is a record");
    swarm
        .issue(
            Some("swarm.agent.Coordinator"),
            "swarm.agent.Assign",
            json!({"agent_id": "tester", "outcome": "prove it",
                   "signoff": "Timo, verbatim", "forbidden": "nothing new",
                   "artifact_ref": Json::Null})
            .as_object()
            .expect("an object")
            .clone(),
            "assign-tester",
        )
        .await
        .expect("the second assignment is posted");

    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;

    let work = swarm.dir().join("work");
    for agent in ["builder", "tester"] {
        let own = work.join(agent);
        assert!(
            own.is_dir(),
            "{agent} has a directory of its own under the swarm's: {own:?}"
        );
        // `.swarm/config.json` is what the `swarm` CLI reads to know who it is speaking as, so it
        // is the directory saying whose it is rather than the case asserting it from the name.
        let settings: Json = serde_json::from_str(
            &std::fs::read_to_string(own.join(".swarm").join("config.json"))
                .unwrap_or_else(|why| panic!("{agent}'s settings are in its own directory: {why}")),
        )
        .expect("the settings are JSON");
        assert_eq!(
            settings["agent"], agent,
            "and the directory belongs to the agent that worked in it: {settings}"
        );
    }

    // A `NOTES.md` each, which is the file the prompt asks every agent to keep and the file two
    // members sharing one directory would have written over.
    for agent in ["builder", "tester"] {
        std::fs::write(
            work.join(agent).join("NOTES.md"),
            format!("{agent} was here\n"),
        )
        .expect("an agent writes its notes");
    }
    for agent in ["builder", "tester"] {
        assert_eq!(
            std::fs::read_to_string(work.join(agent).join("NOTES.md")).expect("it reads back"),
            format!("{agent} was here\n"),
            "neither member's notes are the other's"
        );
    }

    // The scope follows the directory — rule 2 of `frame::scope` is the agent's own and rule 3 is
    // the shared one, readable — but not here: these sessions are `Launch::Program`, which has no
    // frame at all by construction. That half is driven by
    // `frame::tests::the_scope_is_the_agents_own_directory_and_a_readable_shared_one` and, through
    // the real metaharness arm, by `tests/the_frame_states_the_attempt_it_frames.rs`.
}

/// A cap report names the kind of unit it is about, and two units with equal ids are two reports.
///
/// The in-flight claim moved to `Unit{kind,id}` because `state.rs` says in its own words that equal
/// ids across kinds are enforced by nothing — and `report_capped` in the same change went on
/// keying on the bare id, into a map called `capped_goals`. So an assignment id reached `/status`
/// under a field called `goal`, and a goal and an assignment that happened to share an id were one
/// report. Found by the adversary of correction round 1.
#[tokio::test]
async fn a_cap_report_says_which_kind_of_unit_it_is_about() {
    let data = tempdir::TempDir::new("swarm-capped-units").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");

    let same = "77fc1fcc-a89c-4fb9-b041-89ecfc75922f";
    let report = |unit: Unit, agent: &str| swarm_server::state::CappedGoal {
        swarm: "s".to_owned(),
        goal: unit.id().to_owned(),
        unit: unit.kind().to_owned(),
        agent: agent.to_owned(),
        turns: 3,
        spent_usd: None,
        reached: swarm_server::budget::Reached::Turns { turns: 3, max: 3 },
        why: "the turn cap was reached: 3 of 3".to_owned(),
    };

    assert!(
        server.report_capped(&Unit::goal(same), report(Unit::goal(same), "coordinator")),
        "a goal's first report is made"
    );
    assert!(
        server.report_capped(
            &Unit::assignment(same),
            report(Unit::assignment(same), "builder")
        ),
        "and an assignment with the same id is a different unit, so its report is made too"
    );
    assert!(
        !server.report_capped(&Unit::goal(same), report(Unit::goal(same), "coordinator")),
        "while the same unit twice is still reported once — which is what keeps the warning to one"
    );

    let published = server.capped_goals();
    assert_eq!(published.len(), 2, "two units, two reports: {published:?}");
    let kinds: std::collections::BTreeSet<&str> =
        published.iter().map(|c| c.unit.as_str()).collect();
    assert_eq!(
        kinds,
        ["assignment", "goal"].into_iter().collect(),
        "and a /status reader is told which kind each id is, rather than reading an assignment id \
         under a field called `goal`: {published:?}"
    );
    let agents: std::collections::BTreeSet<&str> =
        published.iter().map(|c| c.agent.as_str()).collect();
    assert_eq!(
        agents,
        ["builder", "coordinator"].into_iter().collect(),
        "and whose turn each refusal was: {published:?}"
    );

    server.forget_capped("s", &Unit::goal(same));
    assert_eq!(
        server.capped_goals().len(),
        1,
        "forgetting one unit's report leaves the other's, which a bare id could not do"
    );
}

/// The fourth of the class: the live stream says whose turn it is watching.
///
/// The adversary named three places where the coordinator's assumptions followed the member
/// through — one attempt number, one work directory, one set of knobs — and asked for the fourth.
/// It is the watch stream. `What::Turn` and `What::Agent` carry the unit and the turn number and
/// nothing that says who is taking it, because while a swarm had one agent the unit identified the
/// turn. Two members working at once publish into one channel, and a reader — the canvas, or
/// anyone reading `/watch` — cannot tell one member's events from the other's. It is clause 5's
/// defect exactly, on the stream instead of on disk.
#[tokio::test]
async fn the_watch_stream_says_which_agent_a_turn_belongs_to() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-whose-turn").expect("a scratch directory");
    let program = a_session(&data.path().join("session.sh"), true);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe { std::env::set_var("SWARM_COORDINATOR", &program) };
    let (server, swarm, _goal, _assignment) = a_swarm_with_a_member(&data, "whose-turn").await;

    let mut watching = swarm.watch();
    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;

    let mut turns = 0;
    let mut events = 0;
    while let Ok(change) = watching.try_recv() {
        let published: Json = serde_json::to_value(&change).expect("a change serialises");
        match published["kind"].as_str() {
            Some("turn") => {
                turns += 1;
                assert_eq!(
                    published["agent"], "builder",
                    "a turn on the stream says whose it is: {published}"
                );
            }
            Some("agent") => {
                events += 1;
                assert_eq!(
                    published["agent"], "builder",
                    "and so does each event of its session: {published}"
                );
            }
            _ => {}
        }
    }
    assert!(turns >= 2, "the turn was announced at both ends: {turns}");
    assert!(
        events >= 1,
        "and its session's events were streamed: {events}"
    );
}

/// The fourth variant: a cap published live says whose turn it refused, and what kind of unit.
///
/// `What::Turn` and `What::Agent` gained `agent` in correction round 1 and `What::Capped` did not,
/// so the one announcement that says work STOPPED was the one that did not say whose work. It
/// published an assignment UUID in a field named `goal` with nothing beside it. Found by the
/// adversary of correction round 2.
#[tokio::test]
async fn a_cap_published_live_says_whose_turn_it_refused() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-capped-live").expect("a scratch directory");
    let program = a_session(&data.path().join("session.sh"), false);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var("SWARM_COORDINATOR", &program);
        std::env::set_var("SWARM_MAX_TURNS", "1");
        std::env::set_var("SWARM_MAX_SPEND_USD", "off");
    }
    let (server, swarm, _goal, assignment) = a_swarm_with_a_member(&data, "capped-live").await;

    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;

    let mut watching = swarm.watch();
    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server).await;
    // SAFETY: as above.
    unsafe {
        std::env::remove_var("SWARM_MAX_TURNS");
        std::env::remove_var("SWARM_MAX_SPEND_USD");
    }

    let mut seen = 0;
    while let Ok(change) = watching.try_recv() {
        let published: Json = serde_json::to_value(&change).expect("a change serialises");
        if published["kind"] != "capped" {
            continue;
        }
        seen += 1;
        assert_eq!(
            published["agent"], "builder",
            "the one announcement that says work stopped says whose: {published}"
        );
        assert_eq!(
            published["unit"], "assignment",
            "and what kind of thing the id beside it identifies: {published}"
        );
        assert_eq!(published["goal"], assignment);
    }
    assert_eq!(seen, 1, "the bound fired once and was published once");
}

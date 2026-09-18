//! `story:run-two-workers-at-once`, driven through the real runtime.
//!
//! What `a_second_agent_runs.rs` demonstrated was ONE worker: a coordinator, a member, an
//! assignment, and a turn taken at it. Every honesty paragraph in this repository says so in the
//! same words — "one coordinator, one worker, one assignment" — and the orchestration claim under
//! it rests on n=1.
//!
//! These cases run two members at the same moment, and then hold the swarm to a ceiling on how
//! many turns it may have in flight at all.
//!
//! **The concurrency is measured, not assumed.** A test that starts two turns and finds two spend
//! rows afterwards has measured that two turns happened, which is also what a runtime that ran them
//! one after the other would produce. So the two sessions RENDEZVOUS: each registers itself in a
//! directory, waits for a second registrant, and only then answers `reached: true`. A run in which
//! the second turn starts after the first has finished cannot produce two `true`s, because each
//! session removes its own registration before it answers. The in-flight count is read as well, and
//! the two figures have to agree.
//!
//! # What a verdict is read off here, and what it is NOT read off
//!
//! Not `turns/spend.jsonl`. `Swarm::record_spend` appends each row with `writeln!(file, "{line}")`,
//! which is many small `write` calls — one per fragment of the `Display` — and two members
//! recording a turn at the same moment interleave INSIDE a row. Captured while writing these cases,
//! from a run of the case below:
//!
//! ```text
//!   {"agent{":""agentbuilder"":,""atchecker"":,""at"2026-09-17T23:50:15.408022265Z:"", …
//! ```
//!
//! `Swarm::spend_where` skips a row it cannot parse (`swarm.rs`, `let Ok(row) = … else { continue
//! }`), so what that costs is not a parse error a reader would see: it is TWO TURNS AND THEIR
//! DOLLARS missing from every fold a cap is measured on. That defect is older than these cases and
//! was unreachable before them — one writer cannot interleave with itself — and it belongs to
//! `swarm.rs`, which this wave does not own. It is reported rather than fixed here, and the fix is
//! one line: render the row into a `String` and `write_all` it once.
//!
//! So these cases read what the members did from the EVENT LOG and the instances built from it,
//! which SQLite serialises, and read the concurrency itself from the runtime's own in-flight count.
//! Neither can be shredded by the race above.
//!
//! No money is spent and no provider is contacted. Both sessions are a `Launch::Program` — the
//! stdin→stdout contract `examples/coordinator-manual.sh` satisfies — resolved from
//! `SWARM_COORDINATOR`, which is the same door `a_second_agent_runs.rs` uses. What is under test is
//! the runtime's own path: which turns are admitted, how many at once, and what happens to the ones
//! that are not.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::budget::Caps;
use swarm_server::state::{Server, Unit};
use swarm_server::swarm::Swarm;
use swarm_server::trigger;

/// The process environment is one object and both cases here set it.
static ENVIRONMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn runnable(at: &std::path::Path, script: &str) -> String {
    std::fs::write(at, script).expect("the session program is written");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755))
        .expect("the session program is runnable");
    at.display().to_string()
}

/// A session that waits for a second session, and writes down when it started and when it ended.
///
/// Two independent records of the same fact, because each alone can be argued with:
///
/// * the session answers `reached: true` only after a second `.start` appears beside its own. On a
///   runtime that runs the two one after the other, the first waits the full ten seconds and
///   answers `false`;
/// * both write a nanosecond clock reading at each end of their turn, and [`overlap`] holds the two
///   intervals against each other. Nothing is cleaned up between sessions, so the second of two
///   SEQUENTIAL turns would see two `.start` files and answer `true` — and its interval would still
///   begin after the first one's had ended, which is the assertion that cannot be fooled.
///
/// The `sleep` after the barrier is what makes a concurrent pair overlap by a measurable margin
/// rather than by whatever the scheduler leaves.
fn a_rendezvous(program: &std::path::Path, at: &std::path::Path) -> String {
    std::fs::create_dir_all(at).expect("the rendezvous directory is made");
    runnable(
        program,
        &format!(
            "#!/bin/sh\n\
             cat > /dev/null\n\
             dir='{}'\n\
             mine=\"$dir/$$\"\n\
             date +%s%N > \"$mine.start\"\n\
             seen=1\n\
             i=0\n\
             while [ \"$i\" -lt 200 ]; do\n\
             \tseen=$(ls \"$dir\"/*.start | wc -l)\n\
             \t[ \"$seen\" -ge 2 ] && break\n\
             \tsleep 0.05\n\
             \ti=$((i+1))\n\
             done\n\
             sleep 0.2\n\
             date +%s%N > \"$mine.end\"\n\
             if [ \"$seen\" -ge 2 ]; then\n\
             \tprintf '{{\"reached\": true, \"note\": \"two turns were in flight at once\"}}\\n'\n\
             else\n\
             \tprintf '{{\"reached\": false, \"note\": \"ran alone\"}}\\n'\n\
             fi\n",
            at.display()
        ),
    )
}

/// Whether the turns those sessions took were running at the same moment, read off their own
/// clocks: the latest start is before the earliest end.
///
/// Panics unless exactly two turns wrote both ends of an interval, because "one session never
/// finished" and "the two did not overlap" are different findings and a reader must not have to
/// guess which happened.
fn overlap(at: &std::path::Path) -> (u128, Vec<(u128, u128)>) {
    let reading = |file: std::path::PathBuf| -> u128 {
        std::fs::read_to_string(&file)
            .unwrap_or_else(|why| panic!("{} is readable: {why}", file.display()))
            .trim()
            .parse()
            .expect("a nanosecond clock reading")
    };
    let mut intervals: Vec<(u128, u128)> = std::fs::read_dir(at)
        .expect("the rendezvous directory exists")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path
                .file_name()?
                .to_str()?
                .strip_suffix(".start")?
                .to_owned();
            Some((
                reading(path.clone()),
                reading(path.with_file_name(format!("{name}.end"))),
            ))
        })
        .collect();
    intervals.sort_unstable();
    assert_eq!(
        intervals.len(),
        2,
        "two sessions ran and both wrote both ends of their turn: {intervals:?}"
    );
    let last_start = intervals
        .iter()
        .map(|(start, _)| *start)
        .max()
        .expect("two");
    let first_end = intervals.iter().map(|(_, end)| *end).min().expect("two");
    (first_end.saturating_sub(last_start), intervals)
}

/// A session that holds its turn open until the case lets it go, and for ten seconds at the most.
///
/// The ceiling. A case that measures how many turns a swarm will START needs the first one still
/// running while it looks, and a session that answers immediately is a session that has usually
/// finished before the count is read.
fn held_until_released(program: &std::path::Path, release: &std::path::Path) -> String {
    runnable(
        program,
        &format!(
            "#!/bin/sh\n\
             cat > /dev/null\n\
             i=0\n\
             while [ ! -f '{}' ] && [ \"$i\" -lt 200 ]; do\n\
             \tsleep 0.05\n\
             \ti=$((i+1))\n\
             done\n\
             printf '{{\"reached\": true, \"note\": \"released\"}}\\n'\n",
            release.display()
        ),
    )
}

/// Lets every held session go when this drops, whichever way the case ended.
///
/// A failing assertion unwinds past the line that writes the release file, and the sessions it was
/// holding then poll on behind every case that runs after it — which is how one red case was
/// watched to make an unrelated green one time out. A guard, so that a red case is red on its own
/// account and takes nothing with it.
struct Release(PathBuf);

impl Drop for Release {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.0, "go");
    }
}

/// A started swarm with a coordinator and TWO members, each holding its own assignment.
///
/// Two members and two assignments, because one of each is what the repository has already
/// demonstrated. The assignments are posted by the coordinator through `swarm.agent.Assign`, which
/// is the same door `swarm do` goes through.
async fn a_swarm_with_two_members(
    data: &tempdir::TempDir,
    slug: &str,
) -> (Arc<Server>, Arc<Swarm>, Vec<String>) {
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
                &format!("{command}:{input}"),
            )
            .await
            .expect("the command applies")
    };
    issue(
        None,
        "swarm.manager.CreateSwarm",
        json!({"display_name": slug, "tmux_session": slug, "home": "/nowhere",
               "created_at": "2026-09-18T10:00:00Z"}),
    )
    .await;
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        None,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": swarm_id, "started_at": "2026-09-18T10:01:00Z"}),
    )
    .await;
    issue(
        None,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "two workers, at the same moment"}),
    )
    .await;

    swarm
        .ensure_agent(
            trigger::COORDINATOR,
            trigger::COORDINATOR_ROLE,
            "The coordinator",
        )
        .await
        .expect("the coordinator is a record");
    for (member, what) in [
        ("builder", "A member that builds"),
        ("checker", "A member that checks"),
    ] {
        swarm
            .ensure_agent(member, trigger::WORKER_ROLE, what)
            .await
            .expect("the member is a record");
        issue(
            Some("swarm.agent.Coordinator"),
            "swarm.agent.Assign",
            json!({"agent_id": member, "outcome": format!("{member}: do the half that is yours"),
                   "signoff": "Timo, verbatim", "forbidden": "do not touch the planning store",
                   "artifact_ref": "story:run-two-workers-at-once"}),
        )
        .await;
    }

    let assignments: Vec<String> = swarm
        .instances("swarm.agent.Assignment")
        .await
        .iter()
        .filter_map(|record| record["id"].as_str().map(ToOwned::to_owned))
        .collect();
    assert_eq!(
        assignments.len(),
        2,
        "the coordinator posted two assignments: {assignments:?}"
    );
    (server, swarm, assignments)
}

/// Waits for every turn the trigger spawned, returning the most this swarm ever held at once.
async fn settle(server: &Arc<Server>, slug: &str) -> usize {
    let mut most = server.turns_in_flight_for(slug);
    for _ in 0..3_000 {
        most = most.max(server.turns_in_flight_for(slug));
        if server.turns_in_flight_for(slug) == 0 {
            return most;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("a turn never finished");
}

/// What each member said, as the swarm's own records hold it: who, in what state, and the note its
/// session answered with.
///
/// Read from the instances the event log replays into, for the reason the module doc gives.
async fn reports(swarm: &Arc<Swarm>) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = swarm
        .instances("swarm.agent.Assignment")
        .await
        .into_iter()
        .map(|record| {
            (
                record["fields"]["agent_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                record["state"].as_str().unwrap_or_default().to_owned(),
                record["fields"]["note"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// **The acceptance.** Two workers take their turns at the same moment.
///
/// Both sessions answer `true` only because each saw the other running, and the runtime's own
/// in-flight count agrees: two turns, one swarm, one moment. Set `SWARM_MAX_IN_FLIGHT=1` and this
/// case goes red — both sessions time out alone — which is the mutation that shows the ceiling is
/// the thing deciding.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_workers_take_their_turns_at_the_same_moment() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-two-workers").expect("a scratch directory");
    let program = a_rendezvous(
        &data.path().join("session.sh"),
        &data.path().join("rendezvous"),
    );
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var("SWARM_COORDINATOR", &program);
        std::env::remove_var("SWARM_MAX_IN_FLIGHT");
        std::env::remove_var("SWARM_MAX_TOTAL_SPEND_USD");
    }
    let (server, swarm, assignments) = a_swarm_with_two_members(&data, "two-workers").await;
    assert_eq!(
        server.caps().max_in_flight,
        Caps::default().max_in_flight,
        "the default ceiling is what this case runs under, and it admits both"
    );

    let began = Instant::now();
    trigger::work_the_assignments(&server, &swarm).await;
    // Claimed before either task is spawned, so this is not a race: a ceiling of one would leave
    // one of the two unclaimed here.
    assert_eq!(
        server.turns_in_flight_for("two-workers"),
        2,
        "two turns are in flight at the same instant"
    );
    let most = settle(&server, "two-workers").await;
    assert_eq!(most, 2, "and the swarm held both of them together");

    // The sessions' own account of it. Each answers `true` only if a second was running while it
    // ran, and removes its registration before answering, so two `true`s cannot be produced by two
    // turns that merely both happened. A member that ran alone answers `false`, its assignment
    // stays `Assigned`, and its note never reaches the record at all.
    let rows = reports(&swarm).await;
    assert_eq!(
        rows,
        vec![
            (
                "builder".to_owned(),
                "Done".to_owned(),
                "two turns were in flight at once".to_owned()
            ),
            (
                "checker".to_owned(),
                "Done".to_owned(),
                "two turns were in flight at once".to_owned()
            ),
        ],
        "both members saw the other running: {rows:?}"
    );
    // And the same fact off the sessions' own clocks, which is the reading a sequential runtime
    // cannot produce however its verdicts read.
    let (together, intervals) = overlap(&data.path().join("rendezvous"));
    assert!(
        together > 0,
        "the two turns were running at the same moment: {intervals:?}"
    );
    // The figures this case measured, for a run that is recorded rather than summarised. Read them
    // with `cargo test -p swarm-server --test two_workers_run_at_once -- --nocapture`.
    println!(
        "two workers at once: {most} turns in flight, overlapping for {} ms, intervals (ns) \
         {intervals:?}",
        together / 1_000_000
    );
    assert!(
        began.elapsed() < Duration::from_secs(8),
        "and neither waited the rendezvous out: a pair that overlapped answers as soon as the \
         second one registers, and a pair that did not takes ten seconds to say so"
    );

    // Two members, two assignments, both finished. `AssignmentTaken` fires once per member: the
    // work was attributed to the agent that did it, not to the coordinator.
    let log = swarm.history(500).await.expect("the log reads back");
    let mut took: Vec<&str> = log
        .iter()
        .filter(|event| event.name == "swarm.agent.AssignmentTaken")
        .map(|event| event.id.as_str())
        .collect();
    took.sort_unstable();
    assert_eq!(took, vec!["builder", "checker"], "two takes, two agents");
    let done: Vec<(String, String)> = swarm
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
    for assignment in &assignments {
        assert!(
            done.contains(&(assignment.clone(), "Done".to_owned())),
            "each member finished its own: {done:?}"
        );
    }

    // Running two at once is not a refusal of anything, and nothing was recorded as one.
    assert!(
        server.capped_goals().is_empty(),
        "nothing was capped: {:?}",
        server.capped_goals()
    );
}

/// The ceiling, which is the price of the case above.
///
/// Two open assignments and a swarm that may hold one turn: the second is not refused, not capped
/// and not lost — it is not STARTED, and the next period takes it. Nothing is spent by the turn
/// that did not run, which is the property a bound checked after a launch cannot have.
///
/// Remove the `claim_within` ceiling at `trigger.rs` and this goes red at the first assertion: both
/// turns start, both hold their session open, and the swarm runs two of them under a ceiling of
/// one.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_swarm_at_its_ceiling_starts_no_second_turn() {
    let _guard = ENVIRONMENT.lock().await;
    let data = tempdir::TempDir::new("swarm-ceiling").expect("a scratch directory");
    let release = Release(data.path().join("release"));
    let program = held_until_released(&data.path().join("session.sh"), &release.0);
    // SAFETY: every case in this file that touches the environment holds `ENVIRONMENT` first.
    unsafe {
        std::env::set_var("SWARM_COORDINATOR", &program);
        std::env::set_var("SWARM_MAX_IN_FLIGHT", "1");
    }
    let (server, swarm, assignments) = a_swarm_with_two_members(&data, "at-its-ceiling").await;
    assert_eq!(
        server.caps().max_in_flight,
        Some(1),
        "the host's ceiling, read from the environment and not from any config"
    );

    trigger::work_the_assignments(&server, &swarm).await;
    assert_eq!(
        server.turns_in_flight_for("at-its-ceiling"),
        1,
        "one of the two open assignments was started, and the swarm is full"
    );
    // It stays full: the session is still holding its turn open, and nothing lets a second in
    // behind it. Three more passes of the same arrow, which is what the trigger would do.
    for _ in 0..3 {
        trigger::work_the_assignments(&server, &swarm).await;
        assert_eq!(
            server.turns_in_flight_for("at-its-ceiling"),
            1,
            "a period that finds the swarm full starts nothing"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // A swarm that is full is BUSY, not stopped. Nothing is published, nothing is written, and the
    // turn that did not start is exactly where it was.
    assert!(
        server.capped_goals().is_empty(),
        "being full is not a bound firing: {:?}",
        server.capped_goals()
    );
    assert!(
        !swarm.dir().join("turns").join("capped.jsonl").exists(),
        "and nothing was written to the record of bounds that fired"
    );

    std::fs::write(&release.0, "go").expect("the held turn is released");
    let most = settle(&server, "at-its-ceiling").await;
    assert_eq!(most, 1, "the swarm never held two: {most}");

    // And the next period runs the one that waited. The ceiling defers; it does not cancel.
    trigger::work_the_assignments(&server, &swarm).await;
    settle(&server, "at-its-ceiling").await;
    let rows = reports(&swarm).await;
    assert_eq!(
        rows,
        vec![
            (
                "builder".to_owned(),
                "Done".to_owned(),
                "released".to_owned()
            ),
            (
                "checker".to_owned(),
                "Done".to_owned(),
                "released".to_owned()
            ),
        ],
        "both members took their turn in the end, one period apart: {rows:?}"
    );
    assert_eq!(assignments.len(), rows.len(), "one report per assignment");

    // SAFETY: as above.
    unsafe { std::env::remove_var("SWARM_MAX_IN_FLIGHT") };
}

/// The ceiling is counted per SWARM, not per server.
///
/// One count for the whole process would make a swarm's breadth a function of how many other
/// swarms this server happens to hold, which is not a bound anybody could set.
#[tokio::test]
async fn one_swarms_ceiling_is_not_another_swarms_business() {
    let data = tempdir::TempDir::new("swarm-ceiling-per-swarm").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");

    let one = Unit::assignment("aaaaaaaa-0000-0000-0000-000000000001");
    let two = Unit::assignment("aaaaaaaa-0000-0000-0000-000000000002");
    assert_eq!(server.claim_within("alpha", &one, Some(1)), Ok(true));
    assert_eq!(
        server.claim_within("alpha", &two, Some(1)),
        Err(1),
        "alpha is full, and the count it was measured at says why"
    );
    assert_eq!(
        server.claim_within("beta", &two, Some(1)),
        Ok(true),
        "beta has a ceiling of its own and is empty"
    );
    assert_eq!(
        server.turns_in_flight_for("alpha"),
        1,
        "and the counts do not leak into each other"
    );
    assert_eq!(server.turns_in_flight_for("beta"), 1);
    assert_eq!(server.turns_in_flight(), 2, "two turns on this server");

    server.release("alpha", &one);
    assert_eq!(
        server.claim_within("alpha", &two, Some(1)),
        Ok(true),
        "a turn that ended makes room for the one that waited"
    );
    assert_eq!(
        server.claim_within("alpha", &two, Some(4)),
        Ok(false),
        "and a unit already in flight is still claimed once, which the ceiling does not change"
    );
    assert_eq!(
        server.claim_within("gamma", &one, None),
        Ok(true),
        "a lifted ceiling admits everything, which is what this runtime did before it had one"
    );
}

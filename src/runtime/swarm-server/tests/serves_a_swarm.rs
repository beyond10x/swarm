//! The server, end to end: create a swarm, give it a goal, turn the loop until it stops.
//!
//! This is the path the canvas takes, without the canvas. If it holds here, what is left for the UI
//! is drawing — and if it does not, no amount of Vue will save it.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;
use swarm_server::swarm::{MAX_ATTEMPTS, RETRY_DELAY, Swarm};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A server over a temporary data directory that is cleaned up with the test.
async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-test").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Issues a command the way the HTTP layer would.
async fn issue(swarm: &Arc<Swarm>, command: &str, input: Json) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let request = format!("req-{}", NEXT.fetch_add(1, Ordering::Relaxed));

    swarm
        .issue(None, command, args(input), &request)
        .await
        .unwrap_or_else(|why| panic!("{command}: {why}"))
        .outcome
}

#[tokio::test]
async fn a_swarm_is_created_started_and_runs_its_loop_to_a_reached_goal() {
    let (server, _data) = server().await;
    let swarm = server.open("first").await.expect("a swarm opens");

    // Nothing exists until a command says so. A directory is not a state machine.
    assert!(swarm.instances("swarm.manager.Swarm").await.is_empty());

    issue(
        &swarm,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "First", "tmux_session": "first", "home": "/tmp/first",
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;

    let made = swarm.instances("swarm.manager.Swarm").await;
    assert_eq!(made.len(), 1);
    let id = made[0]["id"].as_str().expect("an identity").to_owned();

    let started = issue(
        &swarm,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": id, "started_at": "2026-09-12T10:01:00Z"}),
    )
    .await;
    assert_eq!(started, "started");

    // A goal, and three turns of the loop driven the way the trigger drives them.
    issue(
        &swarm,
        "swarm.goal.SetGoal",
        json!({"swarm_id": id, "text": "serve this"}),
    )
    .await;
    let goal = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    for turn in 1..=2 {
        issue(
            &swarm,
            "swarm.goal.Pursue",
            json!({"goal_id": goal, "iterations": turn}),
        )
        .await;
        let verdict = issue(
            &swarm,
            "swarm.goal.Evaluate",
            json!({"goal_id": goal, "reached": false}),
        )
        .await;
        assert_eq!(verdict, "not-yet");
    }
    issue(
        &swarm,
        "swarm.goal.Pursue",
        json!({"goal_id": goal, "iterations": 3}),
    )
    .await;
    let verdict = issue(
        &swarm,
        "swarm.goal.Evaluate",
        json!({"goal_id": goal, "reached": true}),
    )
    .await;
    assert_eq!(verdict, "reached");

    // The view the loop reads is now empty: a reached goal is not waiting for a turn.
    let waiting = swarm
        .view("swarm.goal.GoalsAwaitingATick")
        .await
        .expect("the view computes");
    assert!(waiting.is_empty(), "a finished goal wants no more turns");

    // And the served world is a cache of the replay, not a thing of its own.
    swarm.reload().await.expect("the log replays");
    let after = swarm.instances("swarm.goal.Goal").await;
    assert_eq!(after[0]["state"], json!("Reached"));
    assert_eq!(after[0]["fields"]["iterations"], json!(3));
}

#[tokio::test]
async fn a_swarm_survives_the_server_being_restarted() {
    let data = tempdir::TempDir::new("swarm-restart").expect("a scratch directory");

    {
        let spec = Spec::load(kernel()).expect("the kernel resolves");
        let server = Server::start(spec, data.path().to_path_buf())
            .await
            .expect("the server starts");
        let swarm = server.open("persistent").await.expect("a swarm opens");
        issue(
            &swarm,
            "swarm.manager.CreateSwarm",
            json!({"display_name": "Survivor", "tmux_session": "s", "home": "/tmp/s",
                   "created_at": "2026-09-12T10:00:00Z"}),
        )
        .await;
    }
    // The server is gone. Only the directory remains.

    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let restarted = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts again");

    assert_eq!(
        restarted.slugs().await,
        vec!["persistent".to_owned()],
        "a swarm on disk is a swarm the server finds"
    );
    let swarm = restarted.get("persistent").await.expect("it is open");
    let found = swarm.instances("swarm.manager.Swarm").await;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["fields"]["display_name"], json!("Survivor"));
}

#[tokio::test]
async fn the_ui_can_ask_the_server_what_the_system_is() {
    let (server, _data) = server().await;
    let described = server.describe();

    // The UI reads the shape rather than being told it, so a command added to the specification is
    // reachable without the UI being rebuilt.
    assert_eq!(described.system, "swarm");
    assert!(
        described
            .commands
            .iter()
            .any(|command| command.name == "swarm.goal.Evaluate"),
        "the commands are the specification's"
    );
    assert!(
        described
            .entities
            .iter()
            .any(|entity| entity.name == "swarm.goal.Goal"
                && entity.states.iter().any(|state| state == "Reached")),
        "the lifecycles are the specification's"
    );
}

#[tokio::test]
async fn a_command_the_specification_refuses_is_a_bad_request_not_a_crash() {
    let (server, _data) = server().await;
    let swarm = server.open("strict").await.expect("a swarm opens");

    let refused = swarm
        .issue(None, "swarm.goal.NoSuchCommand", args(json!({})), "req-1")
        .await
        .expect_err("the specification declares no such command");
    assert!(refused.to_string().contains("no command"));

    // A real command, missing a required input.
    let incomplete = swarm
        .issue(None, "swarm.goal.SetGoal", args(json!({})), "req-2")
        .await
        .expect_err("text is required");
    assert!(incomplete.to_string().contains("requires input field"));
}

/// A failed at-least-once delivery is kept and re-attempted, not dropped.
///
/// `adopt-activated-config` declares `delivery: at_least_once` with `on_failure: retry`, and the
/// command it carries the event to acts on the swarm the config names. Activating a config for a
/// swarm this log has never seen is therefore a failed delivery of a binding that asked to be
/// retried — which `pump()` hands out as `Routed.result` and `Swarm::issue` used to discard.
#[tokio::test]
async fn a_failed_at_least_once_delivery_is_kept_and_retried_rather_than_dropped() {
    let (server, _data) = server().await;
    let swarm = server.open("retries").await.expect("a swarm opens");

    // No `CreateSwarm`: this identity names a swarm that is not in this world.
    let ghost = "11111111-1111-4111-8111-111111111111";
    issue(
        &swarm,
        "swarm.config.DraftConfig",
        json!({"swarm_id": ghost, "paths": {}, "launch": {}, "schedules": {},
               "budgets": {}, "board": {}}),
    )
    .await;
    let config = swarm.instances("swarm.config.Config").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    issue(
        &swarm,
        "swarm.config.ActivateConfig",
        json!({"config_id": config, "swarm_id": ghost}),
    )
    .await;

    let queued = swarm.deliveries().await;
    assert_eq!(
        queued.len(),
        1,
        "a failed at_least_once delivery is held for another attempt, not dropped"
    );
    assert_eq!(queued[0].binding, "adopt-activated-config");
    assert_eq!(
        queued[0].attempts, 1,
        "a caller can read how many attempts a delivery has had"
    );

    // The delay between attempts is real: a drain before it has passed re-attempts nothing.
    let began = Instant::now();
    assert_eq!(swarm.redeliver(began).await, 0);
    assert_eq!(swarm.deliveries().await[0].attempts, 1);

    // And the attempts are bounded: the delivery is given up rather than retried for ever.
    for attempt in 2..=MAX_ATTEMPTS {
        let at = began + RETRY_DELAY * (attempt - 1);
        assert_eq!(swarm.redeliver(at).await, 1, "attempt {attempt} was made");
        let held = swarm.deliveries().await;
        if attempt < MAX_ATTEMPTS {
            assert_eq!(held[0].attempts, attempt);
        } else {
            assert!(
                held.is_empty(),
                "after {MAX_ATTEMPTS} attempts the delivery is given up, not retried for ever"
            );
        }
    }
}

/// A redelivery that succeeds commits what the first attempt could not — the whole path.
///
/// The transient failure is a stale world, which is the ordinary shape of one: the second handle's
/// world was folded before the first handle wrote the swarm, so the binding's command acts on an
/// instance its world does not hold yet and fails. `reload` is what makes the next attempt see it.
///
/// **Why it is built from two handles on one log, which `store.rs` does not admit.** That module
/// says one writer per file, and this opens two connections to one. It is deliberate and it is the
/// best available proof of the WHOLE path — a real `pump()` failure, owed by `owe()`, drained by
/// `redeliver()` — because this kernel offers no transient failure that one handle can produce:
/// every way a binding's command can fail here is permanent. A missing subject stays missing
/// (identities for `Swarm` and `Config` are minted by the implementation, so no later command can
/// supply the one the delivery wants), and a terminal state is absorbing. A failure that heals must
/// therefore be manufactured, and a stale fold is the smallest manufacture available.
///
/// What would replace it: a seam that makes `apply` or `store.commit` fail once on demand, or a
/// kernel binding whose command has a precondition a later command can satisfy. Either removes the
/// two-connection trick from this file. Until then the narrower claim — a due delivery applies,
/// commits and stops being owed — is proved with one handle in `swarm.rs`'s own unit tests, so if
/// this case is ever deleted for touching a state the store excludes, the queue is not left
/// unproved. It is deterministic rather than flaky: the writes are sequential, awaited one at a
/// time, and no two are in flight together.
#[tokio::test]
async fn a_redelivery_that_succeeds_commits_what_the_first_attempt_could_not() {
    let data = tempdir::TempDir::new("swarm-redeliver").expect("a scratch directory");
    let spec = Arc::new(Spec::load(kernel()).expect("the kernel resolves"));

    let stale = Arc::new(
        Swarm::open(Arc::clone(&spec), data.path(), "stale")
            .await
            .expect("a swarm opens"),
    );
    let writer = Arc::new(
        Swarm::open(Arc::clone(&spec), data.path(), "stale")
            .await
            .expect("the same swarm opens twice"),
    );

    issue(
        &writer,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "Stale", "tmux_session": "s", "home": "/tmp/stale",
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let id = writer.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    assert!(
        stale.instances("swarm.manager.Swarm").await.is_empty(),
        "the second handle has not seen the swarm yet, which is the failure this test needs"
    );

    issue(
        &stale,
        "swarm.config.DraftConfig",
        json!({"swarm_id": id, "paths": {}, "launch": {}, "schedules": {},
               "budgets": {}, "board": {}}),
    )
    .await;
    let config = stale.instances("swarm.config.Config").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        &stale,
        "swarm.config.ActivateConfig",
        json!({"config_id": config, "swarm_id": id}),
    )
    .await;

    assert_eq!(stale.deliveries().await.len(), 1, "the delivery failed");

    stale.reload().await.expect("the log replays");
    assert_eq!(
        stale.redeliver(Instant::now() + RETRY_DELAY).await,
        1,
        "the delivery is attempted again"
    );
    assert!(
        stale.deliveries().await.is_empty(),
        "a delivery that succeeded is no longer owed"
    );

    let adopted = stale.instances("swarm.manager.Swarm").await;
    assert_eq!(
        adopted[0]["fields"]["active_config_id"],
        json!(config),
        "the redelivered command landed in the world"
    );

    // And in the log, not only in the world the retry wrote to.
    stale.reload().await.expect("the log replays");
    assert_eq!(
        stale.instances("swarm.manager.Swarm").await[0]["fields"]["active_config_id"],
        json!(config)
    );
}

/// Two opens of one slug at once are one swarm, not two.
///
/// `Server::open` used to read the map, drop the guard, await `Swarm::open` and then insert with no
/// re-check, so a second open of the same slug replaced the first handle in the map. Whatever the
/// replaced handle still owed — an at-least-once delivery held in memory — went with it, silently.
/// Every caller of one slug must get the one handle.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_opens_of_one_slug_at_once_are_one_swarm() {
    let (server, _data) = server().await;

    let openers: Vec<_> = (0..8)
        .map(|_| {
            let server = Arc::clone(&server);
            tokio::spawn(async move {
                // The store sets no `busy_timeout` (`ess-runtime/src/store.rs:21`), so two opens
                // that reach SQLite at the same instant can have one refused outright. That is a
                // property of the store, not of the map this test is about, so a refusal is waited
                // out rather than asserted on — and whichever open won is then read from the map,
                // which is exactly the identity being checked.
                for _ in 0..50 {
                    match server.open("contested").await {
                        Ok(swarm) => return swarm,
                        Err(why) => {
                            eprintln!("open refused, retrying: {why}");
                            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        }
                    }
                }
                panic!("`contested` never opened");
            })
        })
        .collect();

    let mut handles = Vec::new();
    for opener in openers {
        handles.push(opener.await.expect("the opener does not panic"));
    }

    for (nth, handle) in handles.iter().enumerate() {
        assert!(
            Arc::ptr_eq(&handles[0], handle),
            "opener {nth} got a different handle: a losing open was inserted over a live one"
        );
    }

    // And the one the map kept is that same handle, not a later arrival that replaced it.
    let kept = server.get("contested").await.expect("the swarm is open");
    assert!(
        Arc::ptr_eq(&handles[0], &kept),
        "the map holds a handle no opener was given"
    );
    assert_eq!(server.slugs().await, vec!["contested".to_owned()]);
}

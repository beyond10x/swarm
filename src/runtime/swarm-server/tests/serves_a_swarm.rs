//! The server, end to end: create a swarm, give it a goal, turn the loop until it stops.
//!
//! This is the path the canvas takes, without the canvas. If it holds here, what is left for the UI
//! is drawing — and if it does not, no amount of Vue will save it.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;

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

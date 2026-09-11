//! State as a fold over the log.
//!
//! The claim this file checks is the one event sourcing is for: throw the running state away, replay
//! the log, and get the same thing back. If that ever stops holding, every projection built on the
//! log is quietly wrong, so it is checked against the kernel's own loop rather than a toy stream.

use std::path::PathBuf;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{Spec, Store, World, apply};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Applies a command, commits it to the log under a fresh request key, and advances the world.
///
/// The key identifies one REQUEST, not one command: two turns of the loop are two requests, and
/// reusing a key across them is what `IdempotencyMismatch` exists to catch.
async fn step(
    spec: &Spec,
    store: &Store,
    world: &mut World,
    command: &str,
    input: Json,
    id: Option<&str>,
) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let request = format!("req-{}", NEXT.fetch_add(1, Ordering::Relaxed));

    let done = apply(spec.ir(), world, None, command, &args(input), id)
        .unwrap_or_else(|why| panic!("{command}: {why}"));
    store
        .commit(&done, None, &request)
        .await
        .expect("the log accepts it");
    if let Some(instance) = done.instance.clone() {
        world.insert((instance.entity.clone(), instance.id.clone()), instance);
    }
    done.outcome
}

#[tokio::test]
async fn a_goal_driven_to_completion_replays_to_the_same_goal() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("test-swarm").await.expect("a log");
    let mut world = World::new();

    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.SetGoal",
        json!({"swarm_id": "sw-1", "text": "replay me"}),
        Some("g-1"),
    )
    .await;

    // Two turns that come back along the back-edge, then one that exits.
    for turn in 1..=2 {
        step(
            &spec,
            &store,
            &mut world,
            "swarm.goal.Pursue",
            json!({"goal_id": "g-1", "iterations": turn}),
            None,
        )
        .await;
        step(
            &spec,
            &store,
            &mut world,
            "swarm.goal.Evaluate",
            json!({"goal_id": "g-1", "reached": false}),
            None,
        )
        .await;
    }
    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.Pursue",
        json!({"goal_id": "g-1", "iterations": 3}),
        None,
    )
    .await;
    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.Evaluate",
        json!({"goal_id": "g-1", "reached": true}),
        None,
    )
    .await;

    let live = world[&("swarm.goal.Goal".to_owned(), "g-1".to_owned())].clone();
    assert_eq!(live.state, "Reached");

    // The process dies here. Everything above is gone; only the log remains.
    drop(world);

    let replayed = store
        .rebuild(spec.ir(), "swarm.goal.Goal", "g-1")
        .await
        .expect("the stream replays")
        .expect("the goal existed");

    assert_eq!(replayed.state, live.state, "the loop ended where it ended");
    assert_eq!(replayed.get("text"), live.get("text"));
    assert_eq!(
        replayed.get("iterations"),
        live.get("iterations"),
        "three turns, recovered from the log alone"
    );
    assert_eq!(replayed.identity_field, "goal_id");
}

#[tokio::test]
async fn the_whole_world_rebuilds_from_the_feed() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("test-swarm").await.expect("a log");
    let mut world = World::new();

    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.SetGoal",
        json!({"swarm_id": "sw-1", "text": "first"}),
        Some("g-1"),
    )
    .await;
    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.SetGoal",
        json!({"swarm_id": "sw-1", "text": "second"}),
        Some("g-2"),
    )
    .await;
    step(
        &spec,
        &store,
        &mut world,
        "swarm.goal.Pursue",
        json!({"goal_id": "g-2", "iterations": 1}),
        None,
    )
    .await;

    let rebuilt = store.world(spec.ir()).await.expect("the feed replays");

    assert_eq!(rebuilt.len(), 2, "two goals were created");
    assert_eq!(
        rebuilt[&("swarm.goal.Goal".to_owned(), "g-1".to_owned())].state,
        "Open"
    );
    assert_eq!(
        rebuilt[&("swarm.goal.Goal".to_owned(), "g-2".to_owned())].state,
        "Pursuing",
        "the second goal had a turn started on it"
    );
}

#[tokio::test]
async fn an_empty_log_is_an_empty_world_rather_than_an_error() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("fresh").await.expect("a log");

    // A swarm that has just been created has done nothing, and that is not a failure.
    let world = store.world(spec.ir()).await.expect("the feed replays");
    assert!(world.is_empty());

    let nothing = store
        .rebuild(spec.ir(), "swarm.goal.Goal", "never-existed")
        .await
        .expect("the stream replays");
    assert!(nothing.is_none());
}

#[tokio::test]
async fn one_request_committed_twice_is_recorded_once() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("retries").await.expect("a log");
    let world = World::new();

    let done = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": "once"})),
        Some("g-1"),
    )
    .expect("the goal is set");

    // A caller that does not know whether its first attempt landed retries with the same key. The
    // log answers with the original result rather than writing a second goal.
    store.commit(&done, None, "req-1").await.expect("first");
    store.commit(&done, None, "req-1").await.expect("a retry");

    let goal = store
        .rebuild(spec.ir(), "swarm.goal.Goal", "g-1")
        .await
        .expect("the stream replays")
        .expect("the goal exists");
    assert_eq!(goal.revision, 1, "the retry appended nothing");
}

#[tokio::test]
async fn a_key_means_one_request_within_a_stream() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("clashes").await.expect("a log");
    let world = World::new();

    let first = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": "first"})),
        Some("g-1"),
    )
    .expect("the goal is set");
    store.commit(&first, None, "req-1").await.expect("first");

    let different = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": "different"})),
        Some("g-1"),
    )
    .expect("the goal is set");

    // Same stream, same key, different body. Accepting this would turn a client's bookkeeping
    // error into a lost write, so the log refuses it by name.
    let refused = store
        .commit(&different, None, "req-1")
        .await
        .expect_err("a key means one request");
    assert!(
        refused.to_string().contains("idempotency"),
        "unexpected refusal: {refused}"
    );
}

#[tokio::test]
async fn a_key_is_scoped_to_its_stream_not_to_the_swarm() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("scoping").await.expect("a log");
    let world = World::new();

    // Two instances are two streams, so one key spent on each is two requests and not a clash.
    // Worth pinning: a runtime that assumed swarm-wide keys would refuse honest writes, and one
    // that assumed the opposite would accept dishonest ones.
    for goal in ["g-1", "g-2"] {
        let done = apply(
            spec.ir(),
            &world,
            None,
            "swarm.goal.SetGoal",
            &args(json!({"swarm_id": "sw-1", "text": goal})),
            Some(goal),
        )
        .expect("the goal is set");
        store
            .commit(&done, None, "the-same-key")
            .await
            .unwrap_or_else(|why| panic!("{goal} should be accepted: {why}"));
    }

    let world = store.world(spec.ir()).await.expect("the feed replays");
    assert_eq!(world.len(), 2);
}

#[tokio::test]
async fn one_swarm_cannot_read_another_swarms_history() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mine = Store::in_memory("mine").await.expect("a log");
    let mut world = World::new();

    step(
        &spec,
        &mine,
        &mut world,
        "swarm.goal.SetGoal",
        json!({"swarm_id": "sw-1", "text": "private"}),
        Some("g-1"),
    )
    .await;

    // A different tenant on its own database. The isolation is structural — a StreamId cannot be
    // built without naming a tenant, so reading across one is not a permission that was withheld,
    // it is a call that cannot be written.
    let theirs = Store::in_memory("theirs").await.expect("a log");
    let seen = theirs.world(spec.ir()).await.expect("the feed replays");
    assert!(seen.is_empty(), "a swarm sees only its own history");
}

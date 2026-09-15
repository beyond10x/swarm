//! A replay is work, not a pause, and it must not be taken out of the executor's hands.
//!
//! [`Store::world`] reads the feed — which suspends — and then folds every instance it grouped,
//! which does not. That second half is pure CPU work inside an `async fn`, so before
//! `store.rs:446` it ran from the last page read to the last instance without ever returning to
//! the scheduler: on a current-thread runtime nothing else in the process could make progress
//! for the length of a whole swarm's history, and on a multi-threaded one a worker was occupied
//! for it. Replay is linear in the feed, so the window grows with the log.
//!
//! The claim is checked by counting the replay future's suspensions rather than by timing it. A
//! fold that hands the scheduler back once per instance suspends at least once per instance; a
//! fold that does not suspends only as often as the log reads above it do, which is a handful of
//! pages however many instances the feed holds. Nothing here measures a duration, so nothing here
//! is flaky on a loaded machine.

use std::future::{Future, poll_fn};
use std::path::PathBuf;
use std::pin::pin;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{Issuer, Spec, Store, World, apply};

/// Enough instances that the fold cannot be mistaken for the feed reads above it: the whole feed
/// is one page of 500, so an unyielding `world` has only that page's reads to suspend on.
const INSTANCES: usize = 64;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Writes one goal into the log, which is one instance for the replay to fold.
async fn set_a_goal(spec: &Spec, store: &Store, nth: usize) {
    let world = World::new();
    let done = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": format!("goal {nth}")})),
        Some(&format!("g-{nth}")),
    )
    .expect("the goal is set");
    store
        .commit(&done, None, &Issuer::Runtime, &format!("req-{nth}"))
        .await
        .expect("the log accepts it");
}

#[tokio::test(flavor = "current_thread")]
async fn the_final_fold_gives_the_executor_back_between_instances() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("long-feed").await.expect("a log");

    for nth in 1..=INSTANCES {
        set_a_goal(&spec, &store, nth).await;
    }

    // Drive the replay by hand so its suspensions can be counted. Every `Pending` the future
    // returns is one moment the scheduler was free to run something else; `Ready` ends it.
    let mut replay = pin!(store.world(spec.ir()));
    let mut suspensions = 0usize;
    let rebuilt = poll_fn(|cx| {
        let polled = replay.as_mut().poll(cx);
        if polled.is_pending() {
            suspensions += 1;
        }
        polled
    })
    .await
    .expect("the feed replays");

    assert_eq!(rebuilt.len(), INSTANCES, "every goal came back");
    assert!(
        suspensions >= INSTANCES,
        "the fold of {INSTANCES} instances suspended {suspensions} times: it is holding the \
         executor across the whole replay instead of yielding between instances"
    );
}

/// The yielding must not have cost the replay its result: the same feed still folds to the same
/// world, in the same commit-grouped order, with the state each instance actually reached.
#[tokio::test]
async fn yielding_between_instances_replays_the_same_world() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let store = Store::in_memory("unchanged").await.expect("a log");

    for nth in 1..=3 {
        set_a_goal(&spec, &store, nth).await;
    }

    let rebuilt = store.world(spec.ir()).await.expect("the feed replays");

    assert_eq!(rebuilt.len(), 3);
    for nth in 1..=3 {
        let instance = &rebuilt[&("swarm.goal.Goal".to_owned(), format!("g-{nth}"))];
        assert_eq!(instance.state, "Open");
        assert_eq!(instance.get("text"), Some(&json!(format!("goal {nth}"))));
    }
}

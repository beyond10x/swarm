//! What happens to the commands a caused command causes, driven from `swarm.rs`'s own module doc.
//!
//! That doc was rewritten on 2026-09-12 and says, of `issue`, `tick` and `redeliver`:
//!
//! > All three run the pump over what the command emitted, so a cascade does not depend on which
//! > door the command came in by — that difference is what a fourth path into the world would
//! > really be, and it is the thing to keep out rather than the count of methods.
//!
//! and `Swarm::redeliver`'s own doc says of the pump it runs:
//!
//! > running it anyway is what keeps that a fact about `components.yaml` rather than a difference
//! > between two of this type's methods, and a binding added tomorrow needs no change here.
//!
//! Both sentences are claims about a specification this crate does not contain: `main.rs` says
//! "what this server does is decided by `src/core/`, and a command added there is reachable here
//! without this crate being touched", so the way to test them is to add a binding to a copy of the
//! kernel and run the server against it. That is what `kernel_plus` does. Nothing here edits the
//! kernel the repository ships.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value as Json, json};

use ess_runtime::Spec;
use ess_runtime::route::Occurrence;
use swarm_server::swarm::{RETRY_DELAY, Swarm};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// Copies a tree, shallowly enough for `src/core` — files, and one level of directories.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("the kernel is readable") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copy");
        }
    }
}

/// The kernel specification with one more binding in `components.yaml`.
///
/// The binding is appended to the `bindings:` list, which is the last block of that file.
fn kernel_plus(at: &Path, binding: &str) -> Arc<Spec> {
    copy_tree(&kernel(), at);
    let components = at.join("components.yaml");
    let text = std::fs::read_to_string(&components).expect("components.yaml is readable");
    std::fs::write(&components, format!("{text}\n{binding}\n"))
        .expect("components.yaml is written");
    Arc::new(Spec::load(at).expect("the kernel with one more binding resolves"))
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

async fn issue(swarm: &Swarm, command: &str, input: Json) {
    swarm
        .issue(None, command, args(input), &format!("{command}-once"))
        .await
        .unwrap_or_else(|why| panic!("{command} is accepted: {why}"));
}

/// A command a period caused causes its bindings, exactly as one a caller asked for does.
///
/// The module doc of `swarm.rs` says all three doors run the pump. `turn-the-loop` invokes
/// `swarm.goal.Pursue`, which emits `swarm.goal.GoalPursued`; a binding on that event is a binding
/// on a command a period caused, and this asserts the one thing the doc promises about it — that it
/// is carried.
///
/// The added binding creates a second `Goal`, chosen because a created instance is visible to
/// `instances()` without depending on whether a refused outcome is `Ok` or `Err`. Its `swarm_id`
/// names the goal rather than a swarm, which the kernel already permits: `DraftConfig` in
/// `serves_a_swarm.rs` creates a `Config` naming a swarm that does not exist.
#[tokio::test]
async fn a_command_a_period_caused_is_pumped_like_one_a_caller_asked_for() {
    let spec_at = tempdir::TempDir::new("spec-periodic").expect("a scratch directory");
    let data = tempdir::TempDir::new("swarm-periodic").expect("a scratch directory");
    let spec = kernel_plus(
        spec_at.path(),
        "  - id: goal-of-a-turn\n\
         \x20   summary: A binding on an event a periodic command emits.\n\
         \x20   when: {event: swarm.goal.GoalPursued}\n\
         \x20   invoke: {command: swarm.goal.SetGoal}\n\
         \x20   mapping:\n\
         \x20     swarm_id: event.goal_id\n\
         \x20     text: a turn of the loop happened\n\
         \x20   delivery: at_most_once\n\
         \x20   on_failure: drop",
    );

    let swarm = Swarm::open(spec, data.path(), "periodic")
        .await
        .expect("a swarm opens");
    issue(
        &swarm,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "Periodic", "tmux_session": "s", "home": "/x",
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();
    issue(
        &swarm,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": swarm_id, "started_at": "2026-09-13T10:00:00Z"}),
    )
    .await;
    issue(
        &swarm,
        "swarm.goal.SetGoal",
        json!({"swarm_id": swarm_id, "text": "be adversarially examined"}),
    )
    .await;
    let goal_id = swarm.instances("swarm.goal.Goal").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    let turned = swarm
        .tick(Occurrence {
            binding: "turn-the-loop".to_owned(),
            context: args(json!({"goal_id": goal_id})),
            read: args(json!({"iterations": 1})),
            eligible: true,
        })
        .await
        .expect("the occurrence is accepted");
    assert!(turned, "the loop turned");

    let goals = swarm.instances("swarm.goal.Goal").await;
    assert_eq!(
        goals.len(),
        2,
        "swarm.rs's module doc says all three doors run the pump, so the binding on \
         swarm.goal.GoalPursued should have been carried and created a second goal; \
         Swarm::tick calls no pump at all, so the cascade of a command a period caused is dropped"
    );
}

/// A redelivered command whose cascade fails does not hang the swarm.
///
/// `Swarm::redeliver` holds `self.owed` for the whole drain and then calls `commit_caused` on the
/// cascade; `commit_caused`'s failure arm calls `owe`, which locks `self.owed` again. A tokio
/// `Mutex` is not reentrant, so a cascade that fails for a binding declaring `at_least_once` with
/// `retry` — the only case `owe` reaches its lock in — never returns, and it is still holding
/// `self.world` when it stops, so every later `issue`, `view` and `instances` on that swarm blocks
/// for ever too.
///
/// The transient first failure is the stale-world manufacture `serves_a_swarm.rs` documents at
/// length and uses for the same purpose: the second handle's world was folded before the first
/// handle wrote the swarm.
#[tokio::test]
async fn a_redelivered_commands_failing_cascade_does_not_hang_the_swarm() {
    let spec_at = tempdir::TempDir::new("spec-cascade").expect("a scratch directory");
    let data = tempdir::TempDir::new("swarm-cascade").expect("a scratch directory");
    // On the event `AdoptConfig` — the redelivered command — emits. The two ids are crossed, so
    // the command names a swarm that does not exist and the delivery fails, which is the arm that
    // asks the queue to keep it.
    let spec = kernel_plus(
        spec_at.path(),
        "  - id: adopt-in-a-circle\n\
         \x20   summary: A binding on an event a redelivered command emits.\n\
         \x20   when: {event: swarm.manager.ConfigAdopted}\n\
         \x20   invoke: {command: swarm.manager.AdoptConfig}\n\
         \x20   mapping:\n\
         \x20     swarm_id: event.config_id\n\
         \x20     config_id: event.swarm_id\n\
         \x20   delivery: at_least_once\n\
         \x20   on_failure: retry",
    );

    let stale = Swarm::open(Arc::clone(&spec), data.path(), "cascade")
        .await
        .expect("a swarm opens");
    let writer = Swarm::open(Arc::clone(&spec), data.path(), "cascade")
        .await
        .expect("the same swarm opens twice");

    issue(
        &writer,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "Cascade", "tmux_session": "s", "home": "/x",
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let swarm_id = writer.instances("swarm.manager.Swarm").await[0]["id"]
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
        json!({"swarm_id": swarm_id, "paths": {}, "launch": {}, "schedules": {},
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
        json!({"config_id": config, "swarm_id": swarm_id}),
    )
    .await;
    assert_eq!(
        stale.deliveries().await.len(),
        1,
        "the delivery failed against the stale world and is owed"
    );

    stale.reload().await.expect("the log replays");
    let drained = tokio::time::timeout(
        Duration::from_secs(10),
        stale.redeliver(Instant::now() + RETRY_DELAY),
    )
    .await;
    assert_eq!(
        drained.ok(),
        Some(1),
        "redeliver did not return: it holds self.owed across commit_caused, whose failure arm \
         locks self.owed again (swarm.rs owe()), and a tokio Mutex is not reentrant"
    );
}

/// A delivery the cascade owes survives the drain that caused it.
///
/// Added by the implementor answering the finding above, because "it does not hang" and "it does
/// not lose the delivery" are two different claims and the case above only makes the first. The
/// drain computes what to put back BEFORE the cascade runs; if it then assigns that list to the
/// queue instead of extending it, the delivery the cascade just owed is overwritten — no
/// `give_up`, no `What::Undelivered`, no log line. Measured against exactly that one-word change:
/// assigning makes this case red and leaves the case above green, which is why it is written
/// separately.
#[tokio::test]
async fn a_delivery_the_cascade_owes_survives_the_drain_that_caused_it() {
    let spec_at = tempdir::TempDir::new("spec-owed").expect("a scratch directory");
    let data = tempdir::TempDir::new("swarm-owed-cascade").expect("a scratch directory");
    let spec = kernel_plus(
        spec_at.path(),
        "  - id: adopt-in-a-circle\n\
         \x20   summary: A binding on an event a redelivered command emits.\n\
         \x20   when: {event: swarm.manager.ConfigAdopted}\n\
         \x20   invoke: {command: swarm.manager.AdoptConfig}\n\
         \x20   mapping:\n\
         \x20     swarm_id: event.config_id\n\
         \x20     config_id: event.swarm_id\n\
         \x20   delivery: at_least_once\n\
         \x20   on_failure: retry",
    );

    let stale = Swarm::open(Arc::clone(&spec), data.path(), "owed")
        .await
        .expect("a swarm opens");
    let writer = Swarm::open(Arc::clone(&spec), data.path(), "owed")
        .await
        .expect("the same swarm opens twice");

    issue(
        &writer,
        "swarm.manager.CreateSwarm",
        json!({"display_name": "Owed", "tmux_session": "s", "home": "/x",
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let swarm_id = writer.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    issue(
        &stale,
        "swarm.config.DraftConfig",
        json!({"swarm_id": swarm_id, "paths": {}, "launch": {}, "schedules": {},
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
        json!({"config_id": config, "swarm_id": swarm_id}),
    )
    .await;

    let owed = stale.deliveries().await;
    assert_eq!(owed.len(), 1, "the first delivery is owed");
    assert_eq!(owed[0].binding, "adopt-activated-config");

    // The world catches up, so the re-attempt applies — and its own cascade fails, because
    // `adopt-in-a-circle` crosses the two ids and names a swarm that does not exist.
    stale.reload().await.expect("the log replays");
    assert_eq!(stale.redeliver(Instant::now() + RETRY_DELAY).await, 1);

    let owed = stale.deliveries().await;
    assert_eq!(
        owed.len(),
        1,
        "the cascade's failed at_least_once delivery is owed, not overwritten by the drain's own \
         list: {owed:?}"
    );
    assert_eq!(
        owed[0].binding, "adopt-in-a-circle",
        "the delivery kept is the one the cascade owed"
    );
    assert_eq!(
        owed[0].attempts, 1,
        "its first attempt is the one that failed"
    );

    // And it is a delivery like any other: due later, bounded, and given up in the open. Two more
    // drains at the bound leave the queue empty rather than retrying for ever.
    let mut at = Instant::now() + RETRY_DELAY;
    for _ in 2..=3 {
        at += RETRY_DELAY;
        assert_eq!(stale.redeliver(at).await, 1);
    }
    assert!(
        stale.deliveries().await.is_empty(),
        "the cascade's delivery is bounded by MAX_ATTEMPTS like any other"
    );
}

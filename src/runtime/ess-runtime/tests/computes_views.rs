//! Views, over the kernel's own declarations.
//!
//! `GoalsAwaitingATick` is the one the loop reads to decide what to turn, so it is the one that has
//! to be right: a filter that let the wrong goals through would make the swarm work on goals nobody
//! is waiting on, or miss the ones it should.

use std::path::PathBuf;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{Spec, World, apply, view, views_over};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// A goal in a named state, committed to the world.
fn goal_resting_in(spec: &Spec, world: &mut World, id: &str, text: &str, state: &str) {
    let made = apply(
        spec.ir(),
        world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": text})),
        Some(id),
    )
    .expect("the goal is set");
    let mut instance = made.instance.expect("an instance");
    instance.state = state.to_owned();
    world.insert((instance.entity.clone(), instance.id.clone()), instance);
}

#[test]
fn the_loop_is_offered_only_the_goals_waiting_for_a_turn() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    goal_resting_in(&spec, &mut world, "g-open", "never picked up", "Open");
    goal_resting_in(&spec, &mut world, "g-waiting", "not yet", "Evaluating");
    goal_resting_in(&spec, &mut world, "g-running", "in flight", "Pursuing");
    goal_resting_in(&spec, &mut world, "g-done", "finished", "Reached");

    let waiting =
        view(spec.ir(), &world, "swarm.goal.GoalsAwaitingATick").expect("the view exists");

    let ids: Vec<&str> = waiting
        .rows
        .iter()
        .filter_map(|row| row["goal_id"].as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["g-waiting"],
        "only a goal resting in Evaluating is waiting for a turn"
    );

    // The identity is projected from the instance, not from the field map, because an identity is
    // not a field anybody can write.
    assert_eq!(waiting.rows[0]["goal_id"], json!("g-waiting"));
    assert_eq!(waiting.rows[0]["state"], json!("Evaluating"));
    assert_eq!(waiting.rows[0]["text"], json!("not yet"));
}

#[test]
fn a_view_shows_the_fields_it_declares_and_no_others() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();
    goal_resting_in(&spec, &mut world, "g-1", "a goal", "Evaluating");

    let waiting =
        view(spec.ir(), &world, "swarm.goal.GoalsAwaitingATick").expect("the view exists");
    let row = &waiting.rows[0];

    let mut shown: Vec<&str> = row.keys().map(String::as_str).collect();
    shown.sort_unstable();
    assert_eq!(
        shown,
        vec!["goal_id", "iterations", "state", "swarm_id", "text"]
    );

    // Nothing has turned the loop, so `iterations` is undetermined — present and null, so a reader
    // can tell "no value" from "not promised".
    assert_eq!(row["iterations"], Json::Null);
}

#[test]
fn an_empty_world_gives_an_empty_view_rather_than_an_error() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let waiting =
        view(spec.ir(), &world, "swarm.goal.GoalsAwaitingATick").expect("the view exists");
    assert!(waiting.rows.is_empty(), "a bare swarm has nothing waiting");
}

#[test]
fn every_view_the_kernel_declares_computes() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();
    goal_resting_in(&spec, &mut world, "g-1", "a goal", "Evaluating");

    // 22 views, none of which may panic or refuse on a world that holds one goal.
    let declared = spec.ir().views().len();
    let computed: usize = [
        "swarm.goal.Goal",
        "swarm.manager.Swarm",
        "swarm.agent.Agent",
        "swarm.agent.Assignment",
        "swarm.config.Config",
        "swarm.blackbox.Box",
        "swarm.blackbox.Schema",
        "swarm.blackbox.Connection",
    ]
    .iter()
    .map(|entity| views_over(spec.ir(), &world, entity).len())
    .sum();

    assert_eq!(
        computed, declared,
        "every declared view should compute over some entity"
    );
}

#[test]
fn a_view_reports_the_consistency_it_was_declared_with() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    // Served immediately whatever it says — the runtime does not reproduce lag — but a caller can
    // read what the specification promised and decide for itself.
    let waiting =
        view(spec.ir(), &world, "swarm.goal.GoalsAwaitingATick").expect("the view exists");
    assert_eq!(
        format!("{:?}", waiting.consistency),
        "ReadYourWrites",
        "the loop's view must not lag behind the turn that just ended"
    );
}

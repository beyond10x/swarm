//! Driving the kernel's own loop, against the kernel's own specification.
//!
//! `[loop] -> [coordinator] -> [goal]`, turned until the goal is reached. Nothing here is a fixture:
//! the commands, the guards and the transitions are the ones in `src/core/domains/goal.yaml`, and if
//! that file changes so does what this test does.

use std::path::PathBuf;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{Spec, World, apply};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Applies a command and commits what it produced, the way a store would.
fn commit(
    spec: &Spec,
    world: &mut World,
    actor: &str,
    command: &str,
    input: Json,
    id: Option<&str>,
) -> String {
    let done = apply(spec.ir(), world, Some(actor), command, &args(input), id)
        .unwrap_or_else(|why| panic!("{command} could not be applied: {why}"));
    if let Some(instance) = done.instance.clone() {
        world.insert((instance.entity.clone(), instance.id.clone()), instance);
    }
    done.outcome
}

fn state(world: &World, id: &str) -> String {
    world
        .get(&("swarm.goal.Goal".to_owned(), id.to_owned()))
        .expect("the goal exists")
        .state
        .clone()
}

#[test]
fn the_loop_turns_until_the_goal_is_reached() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();
    let goal = "g-1";

    commit(
        &spec,
        &mut world,
        "swarm.goal.Operator",
        "swarm.goal.SetGoal",
        json!({"swarm_id": "s-1", "text": "make the thing work"}),
        Some(goal),
    );
    assert_eq!(state(&world, goal), "Open");

    // Two turns that report "not yet". The second one is the proof: it can only happen because the
    // lifecycle comes back on itself.
    for turn in 1..=2 {
        let taken = commit(
            &spec,
            &mut world,
            "swarm.goal.Loop",
            "swarm.goal.Pursue",
            json!({"goal_id": goal, "iterations": turn}),
            None,
        );
        assert_eq!(taken, "pursuing");
        assert_eq!(state(&world, goal), "Pursuing");

        let verdict = commit(
            &spec,
            &mut world,
            "swarm.goal.Coordinator",
            "swarm.goal.Evaluate",
            json!({"goal_id": goal, "reached": false}),
            None,
        );
        assert_eq!(verdict, "not-yet", "turn {turn} should not have finished");
        assert_eq!(state(&world, goal), "Evaluating");
    }

    // The third turn reports success, and the guard `reached == true` selects the exit.
    commit(
        &spec,
        &mut world,
        "swarm.goal.Loop",
        "swarm.goal.Pursue",
        json!({"goal_id": goal, "iterations": 3}),
        None,
    );
    let verdict = commit(
        &spec,
        &mut world,
        "swarm.goal.Coordinator",
        "swarm.goal.Evaluate",
        json!({"goal_id": goal, "reached": true}),
        None,
    );

    assert_eq!(verdict, "reached");
    assert_eq!(state(&world, goal), "Reached");

    let ended = &world[&("swarm.goal.Goal".to_owned(), goal.to_owned())];
    assert_eq!(ended.get("iterations"), Some(&json!(3)), "three turns");
    assert_eq!(ended.get("text"), Some(&json!("make the thing work")));
}

#[test]
fn the_loop_may_not_declare_the_goal_met() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    // The separation of actors is the reason the loop is a separate box from the coordinator.
    let refused = apply(
        spec.ir(),
        &world,
        Some("swarm.goal.Loop"),
        "swarm.goal.Evaluate",
        &args(json!({"goal_id": "g-1", "reached": true})),
        None,
    )
    .expect_err("the loop may only tick");

    assert!(
        refused.to_string().contains("may not invoke"),
        "unexpected refusal: {refused}"
    );
}

#[test]
fn a_command_carries_its_facts_on_the_events_it_emits() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let done = apply(
        spec.ir(),
        &world,
        Some("swarm.goal.Operator"),
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "s-1", "text": "ship it"})),
        Some("g-1"),
    )
    .expect("the goal is set");

    let event = done.events.first().expect("one event");
    assert_eq!(event.name, "swarm.goal.GoalSet");
    // check-sets-are-emitted enforces this in the specification; here it is at runtime.
    assert_eq!(event.fields.get("text"), Some(&json!("ship it")));
    assert_eq!(event.fields.get("swarm_id"), Some(&json!("s-1")));
}

#[test]
fn a_goal_nobody_is_pursuing_has_no_turn_to_report_on() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    commit(
        &spec,
        &mut world,
        "swarm.goal.Operator",
        "swarm.goal.SetGoal",
        json!({"swarm_id": "s-1", "text": "anything"}),
        Some("g-1"),
    );

    // Open is a state no move of Evaluate starts from, so the refusing branch answers.
    let taken = commit(
        &spec,
        &mut world,
        "swarm.goal.Coordinator",
        "swarm.goal.Evaluate",
        json!({"goal_id": "g-1", "reached": true}),
        None,
    );

    assert_eq!(taken, "wrong-state");
    assert_eq!(state(&world, "g-1"), "Open", "a refusal moves nothing");
}

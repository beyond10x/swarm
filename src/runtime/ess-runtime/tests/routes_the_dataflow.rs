//! The pump, against the kernel's own bindings.
//!
//! Two causes, both declared in `components.yaml`: `adopt-activated-config` reacts to an event, and
//! `turn-the-loop` fires on a period. Nothing here is a fixture.

use std::path::PathBuf;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{Occurrence, Spec, World, apply, pump, tick};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

fn obj(value: Json) -> Map<String, Json> {
    args(value)
}

/// Ids the interpreter asks for, in order, so a test can name what was minted.
fn minting(prefix: &'static str) -> impl FnMut() -> String {
    let mut n = 0;
    move || {
        n += 1;
        format!("{prefix}-{n}")
    }
}

#[test]
fn an_activated_config_is_adopted_by_the_swarm_it_configures() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    // A swarm, and a config drafted against it.
    let made = apply(
        spec.ir(),
        &world,
        None,
        "swarm.manager.CreateSwarm",
        &args(json!({
            "display_name": "first", "tmux_session": "s", "home": "/tmp/x",
            "created_at": "2026-09-12T10:00:00Z"
        })),
        Some("sw-1"),
    )
    .expect("the swarm is created");
    let swarm = made.instance.expect("an instance");
    world.insert((swarm.entity.clone(), swarm.id.clone()), swarm);

    let drafted = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &args(json!({
            "swarm_id": "sw-1",
            "paths": {}, "launch": {}, "schedules": {}, "budgets": {}, "board": {}
        })),
        Some("cfg-1"),
    )
    .expect("the config is drafted");
    let config = drafted.instance.expect("an instance");
    world.insert((config.entity.clone(), config.id.clone()), config);

    // Activating it emits ConfigActivated, which the binding carries to AdoptConfig.
    let activated = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.ActivateConfig",
        &args(json!({"config_id": "cfg-1", "swarm_id": "sw-1"})),
        None,
    )
    .expect("the config is activated");
    let instance = activated.instance.clone().expect("an instance");
    world.insert((instance.entity.clone(), instance.id.clone()), instance);

    let caused = pump(
        spec.ir(),
        &mut world,
        activated.events,
        &mut minting("routed"),
    )
    .expect("the pump runs");

    let adopted = caused
        .iter()
        .find(|routed| routed.command == "swarm.manager.AdoptConfig")
        .expect("the binding carried the event to AdoptConfig");
    assert_eq!(adopted.binding, "adopt-activated-config");
    assert!(adopted.result.is_ok(), "{:?}", adopted.result);

    // The swarm record learned which config is in force — the whole point of the binding.
    let swarm = &world[&("swarm.manager.Swarm".to_owned(), "sw-1".to_owned())];
    assert_eq!(swarm.get("active_config_id"), Some(&json!("cfg-1")));
}

#[test]
fn a_periodic_occurrence_turns_the_loop() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    let set = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": "do the thing"})),
        Some("g-1"),
    )
    .expect("the goal is set");
    let goal = set.instance.expect("an instance");
    world.insert((goal.entity.clone(), goal.id.clone()), goal);

    let turned = tick(
        spec.ir(),
        &mut world,
        &Occurrence {
            binding: "turn-the-loop".to_owned(),
            context: obj(json!({"goal_id": "g-1"})),
            read: obj(json!({"iterations": 1})),
            eligible: true,
        },
        &mut minting("tick"),
    )
    .expect("the tick runs")
    .expect("an eligible occurrence acts");

    assert_eq!(turned.command, "swarm.goal.Pursue");
    assert!(turned.result.is_ok(), "{:?}", turned.result);

    let goal = &world[&("swarm.goal.Goal".to_owned(), "g-1".to_owned())];
    assert_eq!(goal.state, "Pursuing", "the tick started a turn");
    assert_eq!(goal.get("iterations"), Some(&json!(1)));
}

#[test]
fn an_ineligible_occurrence_does_nothing() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    let set = apply(
        spec.ir(),
        &world,
        None,
        "swarm.goal.SetGoal",
        &args(json!({"swarm_id": "sw-1", "text": "do the thing"})),
        Some("g-1"),
    )
    .expect("the goal is set");
    let goal = set.instance.expect("an instance");
    world.insert((goal.entity.clone(), goal.id.clone()), goal);

    // `eligibility: host_boolean` is where a paused swarm is excluded. The contract puts that
    // decision in the host, and a host that says no must be obeyed silently.
    let quiet = tick(
        spec.ir(),
        &mut world,
        &Occurrence {
            binding: "turn-the-loop".to_owned(),
            context: obj(json!({"goal_id": "g-1"})),
            read: obj(json!({"iterations": 1})),
            eligible: false,
        },
        &mut minting("tick"),
    )
    .expect("the tick runs");

    assert!(quiet.is_none(), "an ineligible occurrence acts on nothing");
    let goal = &world[&("swarm.goal.Goal".to_owned(), "g-1".to_owned())];
    assert_eq!(goal.state, "Open", "nothing moved");
}

#[test]
fn a_host_that_supplies_nothing_is_refused_rather_than_defaulted() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    let refused = tick(
        spec.ir(),
        &mut world,
        &Occurrence {
            binding: "turn-the-loop".to_owned(),
            context: Map::new(),
            read: Map::new(),
            eligible: true,
        },
        &mut minting("tick"),
    )
    .expect_err("an unsupplied mapping is a refusal");

    assert!(
        refused.to_string().contains("supplied no such field"),
        "unexpected refusal: {refused}"
    );
}

#[test]
fn every_binding_this_kernel_declares_is_one_the_pump_serves() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");

    // at_least_once with retry asks for redelivery, which needs a store and a clock. The pump has
    // neither, so a caller should know which bindings it under-serves rather than discover it.
    let owed = ess_runtime::route::needs_redelivery(spec.ir());
    assert_eq!(
        owed,
        vec![
            "adopt-activated-config".to_owned(),
            "note-the-assignment".to_owned(),
            "record-the-assignment".to_owned(),
        ],
        "the set of bindings needing redelivery has changed"
    );
}

/// One event, two bindings — the only fan-out ESS has.
///
/// `AssignmentPosted` is named by `record-the-assignment` and by `note-the-assignment`, and the
/// pump invokes every binding whose cause matches rather than the first. Nothing in ess's own
/// examples exercises this, so until 2026-09-12 it was a property the loop in `route.rs` asserted
/// and nothing checked.
#[test]
fn one_event_reaches_every_binding_that_names_it() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let mut world = World::new();

    // An agent to assign to. `Assign` runs from Spawned.
    let spawned = apply(
        spec.ir(),
        &world,
        None,
        "swarm.agent.Spawn",
        &args(json!({
            "agent_id": "coordinator",
            "swarm_id": "sw-1",
            "role": "Coordinator",
            "harness": "ClaudeCode",
            "display_name": "The coordinator",
            "host": {}
        })),
        Some("ag-1"),
    )
    .expect("the agent is spawned");
    let agent = spawned.instance.expect("an instance");
    // The identity is the role slug the caller supplied, not a minted id: `Spawn` names an event
    // field of type String, and the event carries `input.agent_id`.
    assert_eq!(agent.id, "coordinator");
    world.insert((agent.entity.clone(), agent.id.clone()), agent);

    let assigned = apply(
        spec.ir(),
        &world,
        None,
        "swarm.agent.Assign",
        &args(json!({
            "agent_id": "coordinator",
            "outcome": "the tests are green",
            "signoff": "Timo",
            "forbidden": "do not touch the spec"
        })),
        Some("as-1"),
    )
    .expect("the assignment is posted");
    assert_eq!(assigned.outcome, "assigned");
    let subject = assigned.instance.clone().expect("an instance");
    world.insert((subject.entity.clone(), subject.id.clone()), subject);

    let caused = pump(
        spec.ir(),
        &mut world,
        assigned.events.clone(),
        &mut minting("rec"),
    )
    .expect("the bindings carry it");

    let by_binding: Vec<&str> = caused
        .iter()
        .map(|routed| routed.binding.as_str())
        .collect();
    assert_eq!(
        by_binding,
        vec!["note-the-assignment", "record-the-assignment"],
        "both bindings on AssignmentPosted fire, in the IR's order"
    );
    for routed in &caused {
        assert!(
            routed.result.is_ok(),
            "{} was refused: {:?}",
            routed.binding,
            routed.result.as_ref().err()
        );
    }

    // The record the first binding exists to create, which nothing wrote before 2026-09-12.
    let assignments: Vec<_> = world
        .values()
        .filter(|instance| instance.entity == "swarm.agent.Assignment")
        .collect();
    assert_eq!(assignments.len(), 1, "one assignment record");
    assert_eq!(assignments[0].state, "Assigned");
}

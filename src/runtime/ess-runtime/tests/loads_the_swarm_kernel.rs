//! The interpreter is pointed at this repository's own specification, because a loader that only
//! works on a fixture is a loader nobody has tried.

use std::path::PathBuf;

use ess_runtime::Spec;

fn kernel() -> PathBuf {
    // tests/ -> ess-runtime/ -> runtime/ -> src/ -> swarm/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

#[test]
fn the_kernel_compiles_from_its_manifest() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    assert_eq!(spec.files(), 8, "ess-inputs.yaml names eight files");
    assert_eq!(spec.ir().system().to_string(), "swarm");
}

#[test]
fn the_goal_lifecycle_carries_the_loop() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let goal = spec
        .ir()
        .entities()
        .iter()
        .find(|(name, _)| name.to_string() == "swarm.goal.Goal")
        .map(|(_, entity)| entity)
        .expect("the goal entity is declared");

    let lifecycle = &goal.lifecycle;

    // the back-edge: a goal that was evaluated and not reached is picked up again
    let pursue = lifecycle
        .transitions
        .iter()
        .find(|t| t.name.as_str() == "pursue")
        .expect("`pursue` is declared");
    assert!(
        pursue.from.iter().any(|s| s.as_str() == "Evaluating"),
        "the loop closes from Evaluating, found {:?}",
        pursue.from
    );
    assert_eq!(pursue.to.as_str(), "Pursuing");
}

#[test]
fn the_exit_condition_is_a_parsed_predicate() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let evaluate = spec
        .ir()
        .commands()
        .iter()
        .find(|(name, _)| name.to_string() == "swarm.goal.Evaluate")
        .map(|(_, command)| command)
        .expect("Evaluate is declared");

    let guarded = evaluate
        .outcomes
        .iter()
        .find(|outcome| outcome.name.as_str() == "reached")
        .expect("the reached outcome exists");

    // Not a string this crate has to parse: the compiler already did.
    let rendered = format!("{:?}", guarded.condition);
    assert!(
        rendered.contains("reached"),
        "the exit guard reads `reached`, found {rendered}"
    );

    // ESS-COMMAND-005: exactly one outcome may be unconditional.
    let unconditional = evaluate
        .outcomes
        .iter()
        .filter(|o| format!("{:?}", o.condition).contains("Otherwise"))
        .count();
    assert_eq!(unconditional, 1, "one and only one unconditional outcome");
}

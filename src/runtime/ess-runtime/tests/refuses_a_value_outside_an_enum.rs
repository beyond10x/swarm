//! A value the compiled enum does not declare is refused, by name.
//!
//! Against this repository's own specification, not a fixture: `swarm.config.Harness` declares
//! `[Claude, Codex, B10x]` in `src/core/domains/config.yaml`, and the whole point is that the
//! vocabulary comes from the IR rather than from anything written down twice.
//!
//! The planted value is `"ClaudeCode"` on purpose. It is the one this runtime actually wrote for
//! every agent it spawned until 2026-09-18 — `swarm-server/src/swarm.rs` put it on
//! `swarm.agent.Spawn`, while `coordinator.rs` read a launch line back by matching `Some("Claude")`
//! — so what these cases assert is not a hypothetical: it is the defect, refused.
//!
//! Each case names the mutation that turns it red, because a check nobody has broken on purpose is
//! a check nobody has tested.

use std::path::PathBuf;

use serde_json::{Map, Value as Json, json};

use ess_runtime::{ApplyError, Spec, World, apply};

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// A whole `swarm.config.DraftConfig` input, with the launch line's harness planted by the caller.
///
/// Every other field is filled because the point of the case is the one value: a refusal that
/// fired because half the input was missing would prove nothing about enums.
fn draft_config(harness: Json) -> Map<String, Json> {
    args(json!({
        "swarm_id": "5b6cc1b4-1c3a-4d25-9a0e-1f1e0a9c5a01",
        "paths": {
            "runs_dir": "runs", "board_store": "board", "events_store": "events",
            "agent_state_projection": "agents.json", "planning_store": ".engineering/planning",
            "wake_dir": "wake.d", "roles_dir": ".agents", "rules_file": "WORKING-RULES.md",
            "memory_file": "MEMORY.md", "decisions_file": "decisions.md",
            "scratch_dir": "scratch"
        },
        "launch": {
            "harness": harness,
            "binary": "", "model": "", "args": [], "skip_permissions": false,
            "tool_surface": "Owned", "allow_program": [], "admitted_operations": []
        },
        "schedules": {
            "check_in_cron": "*/30 * * * *", "memory_tick_cron": "0 * * * *", "cron_ttl_days": 7,
            "wake_poll_seconds": 60, "wake_min_interval_seconds": 300,
            "wake_max_idle_seconds": 900
        },
        "budgets": {
            "memory_file_bytes": 8192, "max_defers": 3, "assignment_stale_seconds": 7200,
            "event_stale_seconds": 900, "event_msg_max_chars": 500, "disk_floor_gb": 40,
            "disk_target_gb": 80, "subagent_tool_calls_per_round": 30
        },
        "board": {
            "orchestrator_slug": "orchestrator", "default_inbox": "main",
            "broadcast_prefer": ["main", "orchestrator"]
        },
        "note": null
    }))
}

/// The refusal names the value, the enum and the vocabulary — not "invalid input".
///
/// MUTATION: delete the `require_declared_variants` call from `apply` in `apply.rs`, and this goes
/// red on the `expect_err` — the config is drafted and `ConfigDrafted` carries `"ClaudeCode"`.
#[test]
fn a_planted_config_naming_a_non_variant_harness_is_refused_by_name() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let refused = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &draft_config(json!("ClaudeCode")),
        Some("cfg-1"),
    )
    .expect_err("`ClaudeCode` is not a harness this specification declares");

    assert_eq!(
        refused,
        ApplyError::NotAVariant {
            command: "swarm.config.DraftConfig".to_owned(),
            path: "launch.harness".to_owned(),
            type_name: "swarm.config.Harness".to_owned(),
            value: "\"ClaudeCode\"".to_owned(),
            variants: vec!["Claude".to_owned(), "Codex".to_owned(), "B10x".to_owned()],
        }
    );

    // By name, in the text an operator reads: the path, the value, the type and every variant.
    let said = refused.to_string();
    assert_eq!(
        said,
        "`swarm.config.DraftConfig` input `launch.harness` is \"ClaudeCode\", which is not a \
         variant of `swarm.config.Harness`; it declares `Claude`, `Codex`, `B10x`"
    );
}

/// The inverse, so the case above cannot pass by refusing everything.
///
/// MUTATION: make `in_vocabulary`'s `Enum` arm return the error unconditionally, and this goes red.
#[test]
fn the_same_config_naming_a_declared_harness_is_drafted() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let drafted = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &draft_config(json!("Claude")),
        Some("cfg-1"),
    )
    .expect("`Claude` is declared");

    assert_eq!(drafted.outcome, "drafted");
    assert_eq!(
        drafted
            .instance
            .expect("an instance")
            .get("launch")
            .expect("the launch line was written")["harness"],
        json!("Claude")
    );
}

/// The same refusal one level up, where the enum is the input field itself rather than a field of
/// a struct — the position `swarm-server` was writing `"ClaudeCode"` into.
///
/// MUTATION: delete the `require_declared_variants` call, and this goes red: the agent is spawned
/// and `AgentSpawned` carries a harness nothing can route.
#[test]
fn spawning_an_agent_on_an_undeclared_harness_is_refused() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let refused = apply(
        spec.ir(),
        &world,
        None,
        "swarm.agent.Spawn",
        &args(json!({
            "agent_id": "coordinator",
            "swarm_id": "5b6cc1b4-1c3a-4d25-9a0e-1f1e0a9c5a01",
            "role": "Coordinator",
            "harness": "ClaudeCode",
            "display_name": "The coordinator",
            "host": {}
        })),
        Some("ag-1"),
    )
    .expect_err("`ClaudeCode` is not a harness this specification declares");

    assert!(
        matches!(&refused, ApplyError::NotAVariant { path, value, .. }
            if path == "harness" && value == "\"ClaudeCode\""),
        "found {refused:?}"
    );
}

/// A second enum on the same command, so the check is not `swarm.config.Harness` special-cased.
///
/// MUTATION: replace the `variants.iter().any(..)` test with a comparison against the literal
/// `"Claude"`, and this goes red — `Builder` would be refused for the wrong reason and
/// `Coordinator` would be refused too.
#[test]
fn a_role_outside_its_own_enum_is_refused_by_its_own_vocabulary() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let refused = apply(
        spec.ir(),
        &world,
        None,
        "swarm.agent.Spawn",
        &args(json!({
            "agent_id": "builder",
            "swarm_id": "5b6cc1b4-1c3a-4d25-9a0e-1f1e0a9c5a01",
            "role": "Builder",
            "harness": "Claude",
            "display_name": "The builder",
            "host": {}
        })),
        Some("ag-2"),
    )
    .expect_err("`Builder` is a roster name, not a role this specification declares");

    assert_eq!(
        refused.to_string(),
        "`swarm.agent.Spawn` input `role` is \"Builder\", which is not a variant of \
         `swarm.agent.Role`; it declares `Coordinator`, `Worker`"
    );
}

/// The vocabulary is read from the compiled IR, not from a list in this crate.
///
/// MUTATION: add a fourth variant to `swarm.config.Harness` in `src/core/domains/config.yaml` and
/// this case says so, without a line of Rust changing. That is the property under test: the
/// specification is authoritative over the value, and the interpreter holds no copy of it.
#[test]
fn the_vocabulary_comes_from_the_specification() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let refused = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &draft_config(json!("Codex")),
        Some("cfg-1"),
    );
    assert!(
        refused.is_ok(),
        "`Codex` is declared beside `Claude`, and the check reads the declaration"
    );

    let ApplyError::NotAVariant { variants, .. } = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &draft_config(json!("ClaudeCode")),
        Some("cfg-1"),
    )
    .expect_err("still refused") else {
        panic!("the refusal is the enum's");
    };

    let declared: Vec<String> = spec
        .ir()
        .types()
        .iter()
        .find(|(name, _)| name.to_string() == "swarm.config.Harness")
        .map(|(_, declared)| match &declared.body {
            ess_compiler::ir::ResolvedBody::Enum { variants } => variants.clone(),
            other => panic!("`swarm.config.Harness` is an enum, found {other:?}"),
        })
        .expect("the harness enum is declared");

    assert_eq!(variants, declared, "the refusal quotes the IR, verbatim");
}

/// A value at an enum position that is not even a string is refused the same way, and the message
/// says what arrived rather than rendering it as text.
///
/// MUTATION: change `value.as_str()` to `Some(value.to_string())`-style coercion and this goes red.
#[test]
fn a_harness_that_is_not_a_string_is_refused_too() {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let world = World::new();

    let refused = apply(
        spec.ir(),
        &world,
        None,
        "swarm.config.DraftConfig",
        &draft_config(json!(3)),
        Some("cfg-1"),
    )
    .expect_err("a number is not a variant");

    assert!(
        matches!(&refused, ApplyError::NotAVariant { value, .. } if value == "3"),
        "found {refused}"
    );
}

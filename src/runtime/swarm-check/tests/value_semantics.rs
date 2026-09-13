mod support;
#[test]
fn integer_attempt_claims() {
    support::assert_case("integer_attempt_claims");
}

#[test]
fn numeric_attribution_equality() {
    support::assert_case("numeric_attribution_equality");
}

#[test]
fn counting_is_distinct_from_numeric_equality() {
    support::assert_case("counting_is_distinct_from_numeric_equality");
}

#[test]
fn equivalent_integer_keys_keep_first_spelling() {
    support::assert_case("equivalent_integer_keys_keep_first_spelling");
}

#[test]
fn census_integer_membership_and_unbounded_sum() {
    support::assert_case("census_integer_membership_and_unbounded_sum");
}

#[test]
fn denial_reason_truthiness() {
    support::assert_case("denial_reason_truthiness");
}

#[test]
fn recursive_reason_representation() {
    support::assert_case("recursive_reason_representation");
}

#[test]
fn spawned_role_representation() {
    support::assert_case("spawned_role_representation");
}

#[test]
fn posted_agent_representation() {
    support::assert_case("posted_agent_representation");
}

#[test]
fn taken_agent_representation() {
    support::assert_case("taken_agent_representation");
}

#[test]
fn tool_name_representation() {
    support::assert_case("tool_name_representation");
}

#[test]
fn decider_representation() {
    support::assert_case("decider_representation");
}

#[test]
fn operation_iteration_and_representation() {
    support::assert_case("operation_iteration_and_representation");
}

#[test]
fn event_json_preserves_source_member_order() {
    support::assert_case("event_json_preserves_source_member_order");
}

#[test]
fn ordered_evidence_does_not_change_runtime_json_map_serialization() {
    let raw = r#"{"z": 1, "a": 2}"#;
    let runtime: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(serde_json::to_vec(&runtime).unwrap(), br#"{"a":2,"z":1}"#);
    let evidence: swarm_check::evidence::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(evidence.python(), "{'z': 1, 'a': 2}");
}

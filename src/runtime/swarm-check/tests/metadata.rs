mod support;
#[path = "support/vocabulary.rs"]
mod vocabulary;
use std::{fs, path::PathBuf};
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn store() -> String {
    fs::read_to_string(repo().join("src/runtime/ess-runtime/src/store.rs")).unwrap()
}

#[test]
fn test_adversary_issuer_pass_1_theissuervocabularyguard_test_a_fifth_variant_whose_label_arm_is_a_block_is_caught()
 {
    let source = vocabulary::fifth(
        "Scheduler",
        r#"Self::Scheduler => {
                let mut said = String::from("sched");
                said.push_str("uler");
                said
            }"#,
    );
    assert!(
        vocabulary::guard(&source)
            .unwrap_err()
            .contains("Scheduler")
    );
}

#[test]
fn test_adversary_issuer_pass_1_theissuervocabularyguard_test_the_guard_is_green_against_the_store_it_was_written_for()
 {
    vocabulary::guard(&store()).unwrap();
    vocabulary::guard(vocabulary::FOUR).unwrap();
}

#[test]
fn test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_a_struct_variant_is_a_missing_label_and_not_a_silence()
 {
    let source = vocabulary::fifth(
        "Scheduler { at: String }",
        r#"Self::Scheduler { at } => format!("sched:{at}"),"#,
    );
    assert!(
        vocabulary::guard(&source)
            .unwrap_err()
            .contains("Scheduler")
    );
}

#[test]
fn test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_the_guard_is_green_against_the_four()
 {
    vocabulary::guard(vocabulary::FOUR).unwrap();
    let source = vocabulary::fifth(
        "Scheduler { at: String }",
        "Self::Scheduler { at } => format!(\"sched:{at}\"),",
    );
    assert!(source.contains("Scheduler"));
    assert!(!vocabulary::FOUR.contains("Scheduler"));
}

#[test]
fn test_adversary_pass2_thefixtureagainstthedomain_test_the_fixture_spawns_a_role_the_domain_has_no_variant_for()
 {
    let data = support::corpus();
    let happy = &data["test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven"]
        [0];
    let yaml = fs::read_to_string(repo().join("src/core/domains/agent.yaml")).unwrap();
    let block =
        regex::Regex::new(r"(?m)^  - name: swarm\.agent\.Role\n((?:    .*\n|\n)*)").unwrap();
    let body = block.captures(&yaml).unwrap()[1].to_owned();
    let variants: std::collections::BTreeSet<_> = body
        .lines()
        .filter_map(|line| line.strip_prefix("      - "))
        .collect();
    assert!(!variants.is_empty());
    for event in happy["input"]["events"].as_array().unwrap() {
        if event["name"] == "swarm.agent.AgentSpawned" {
            assert!(variants.contains(event["data"]["role"].as_str().unwrap()));
        }
    }
}

#[test]
fn test_checker_theissuervocabulary_test_a_variant_whose_label_is_computed_is_a_missing_label_and_not_a_silence()
 {
    let source = vocabulary::fifth(
        "Scheduler",
        r#"Self::Scheduler => {
                let mut said = String::from("sched");
                said.push_str("uler");
                said
            }"#,
    );
    assert!(
        vocabulary::guard(&source)
            .unwrap_err()
            .contains("Scheduler")
    );
}

#[test]
fn test_checker_theissuervocabulary_test_a_variant_with_no_label_arm_at_all_is_caught_by_name() {
    let source = vocabulary::fifth("Scheduler", "");
    assert!(!source.contains("Self::Scheduler"));
    assert!(
        vocabulary::guard(&source)
            .unwrap_err()
            .contains("Scheduler")
    );
}

#[test]
fn test_checker_theissuervocabulary_test_the_checker_names_every_value_the_runtime_can_write() {
    vocabulary::guard(&store()).unwrap();
}

#[test]
fn test_checker_theissuervocabulary_test_the_fixture_speaks_the_same_vocabulary() {
    let data = support::corpus();
    let happy = &data["test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven"]
        [0];
    let issuers: std::collections::BTreeSet<_> = happy["input"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["issuer"].as_str().unwrap())
        .collect();
    for issuer in [
        &swarm_check::records::RUNTIME,
        &swarm_check::records::OPERATOR,
    ] {
        assert!(issuers.contains(issuer));
    }
    assert!(
        issuers
            .iter()
            .any(|s| s.starts_with(swarm_check::records::AGENT))
    );
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven",
    );
}

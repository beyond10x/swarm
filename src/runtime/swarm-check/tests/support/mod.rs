mod fixtures;
pub use fixtures::{corpus, materialize};
use serde_json::Value;
use std::process::Command;
pub fn assert_case(id: &str) {
    let mut corpus = corpus();
    let extra: Value =
        serde_json::from_str(include_str!("../fixtures/compatibility.json")).unwrap();
    corpus
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let cases = corpus[id].as_array().expect("mapped case");
    assert!(!cases.is_empty(), "case must execute evidence");
    for (index, case) in cases.iter().enumerate() {
        let tmp = tempdir::TempDir::new("swarm-check").unwrap();
        let root = tmp.path().join("swarm");
        materialize(&case["input"], &root);
        let result = Command::new(env!("CARGO_BIN_EXE_swarm-check"))
            .arg(&root)
            .arg("--json")
            .output()
            .unwrap();
        let text = String::from_utf8(result.stdout)
            .unwrap()
            .replace(root.to_str().unwrap(), "$ROOT");
        assert_eq!(
            result.status.code(),
            Some(if case["expected"]["ok"] == true { 0 } else { 1 }),
            "{id} subcase {index}: {text} {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let actual: Value = serde_json::from_str(&text).expect("CLI emits JSON");
        assert_eq!(actual, case["expected"], "{id} subcase {index}");
        let result = Command::new(env!("CARGO_BIN_EXE_swarm-check"))
            .arg(&root)
            .output()
            .unwrap();
        let text = String::from_utf8(result.stdout)
            .unwrap()
            .replace(root.to_str().unwrap(), "$ROOT");
        assert_eq!(
            result.status.code(),
            Some(if actual["ok"] == true { 0 } else { 1 })
        );
        metadata_claims(id, &actual, &text);
        for clause in actual["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .chain(std::iter::once(&actual["unattended"]))
        {
            assert!(text.contains(clause["title"].as_str().unwrap()));
            assert!(text.contains(clause["found"].as_str().unwrap()));
        }
        assert!(text.contains(&format!(
            "7 clauses: {} met, {} not met",
            actual["met"], actual["not_met"]
        )));
    }
}

fn metadata_claims(id: &str, actual: &Value, text: &str) {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if id.contains("test_the_readme_names_every_clause_no_run_can_meet") {
        let readme = std::fs::read_to_string(repo.join("examples/two-agents/README.md")).unwrap();
        let section = readme
            .split("## What no run can meet yet")
            .nth(1)
            .unwrap()
            .split("\n## ")
            .next()
            .unwrap();
        for clause in actual["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["met"] == false)
        {
            assert!(
                section
                    .to_lowercase()
                    .contains(&format!("clause {}", clause["number"]))
            );
        }
    }
    if id.contains("TheExportedEvidence") {
        let exported =
            std::fs::read_to_string(repo.join(
                "examples/two-agents/evidence/2026-09-13-two-agents-proof/checker-output.txt",
            ))
            .unwrap();
        let previous: Vec<_> = exported
            .lines()
            .filter(|l| l.starts_with("unattended"))
            .map(str::trim_end)
            .collect();
        let current: Vec<_> = text
            .lines()
            .filter(|l| l.starts_with("unattended"))
            .map(str::trim_end)
            .collect();
        assert_eq!(previous.len(), 1);
        assert_eq!(current, previous);
    }
}

#[path = "support/fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};
use std::process::Command;

fn happy() -> Value {
    fixtures::corpus()["test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven"][0]["input"].clone()
}

fn run(input: &Value, name: &str) -> (i32, Value) {
    let tmp = tempdir::TempDir::new(name).unwrap();
    let root = tmp.path().join("swarm");
    fixtures::materialize(input, &root);
    if let Some(retained) = std::env::var_os("SWARM_ADVERSARY_RETAIN") {
        let path = std::path::PathBuf::from(retained).join(name);
        if !path.exists() {
            fixtures::materialize(input, &path);
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-check"))
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.code().unwrap(), report)
}

#[test]
fn boolean_iteration_missing_transcript_retains_legacy_failing_verdict() {
    let mut input = happy();
    let spend = input["files"]["turns/spend.jsonl"]
        .as_str()
        .unwrap()
        .to_owned();
    input["files"]["turns/spend.jsonl"] = json!(format!(
        "{spend}{}\n",
        json!({
            "agent": "builder", "goal": "ffffffff-unrecorded", "iterations": true
        })
    ));
    let (exit, report) = run(&input, "boolean-iteration");
    assert_eq!(
        exit, 1,
        "legacy reader counts this claimed attempt: {report}"
    );
    assert_eq!(report["clauses"][2]["met"], false);
}

#[test]
fn integral_float_iterations_retain_legacy_spend_attribution() {
    let mut input = happy();
    let spend = input["files"]["turns/spend.jsonl"]
        .as_str()
        .unwrap()
        .replace("\"iterations\": 1", "\"iterations\": 1.0")
        .replace("\"iterations\": 2", "\"iterations\": 2.0");
    input["files"]["turns/spend.jsonl"] = json!(spend);
    let (exit, report) = run(&input, "float-iterations");
    assert_eq!(
        exit, 0,
        "legacy reader attributes integral float iterations: {report}"
    );
    assert_eq!(report["clauses"][2]["met"], true);
}

#[test]
fn unsigned_iteration_missing_transcript_retains_legacy_failing_verdict() {
    let mut input = happy();
    let spend = input["files"]["turns/spend.jsonl"]
        .as_str()
        .unwrap()
        .to_owned();
    input["files"]["turns/spend.jsonl"] = json!(format!(
        "{spend}{}\n",
        json!({
            "agent": "builder", "goal": "ffffffff-unrecorded", "iterations": u64::MAX
        })
    ));
    let (exit, report) = run(&input, "unsigned-iteration");
    assert_eq!(
        exit, 1,
        "legacy reader counts this claimed attempt: {report}"
    );
    assert_eq!(report["clauses"][2]["met"], false);
}

#[test]
fn false_denial_reason_retains_legacy_empty_suffix() {
    let mut input = happy();
    let transcript = input["files"]["turns/0002-01-73a4ac74.jsonl"]
        .as_str()
        .unwrap()
        .replace(
            "\"reason\": \"this step only admits `file.read` operations\"",
            "\"reason\": false",
        );
    input["files"]["turns/0002-01-73a4ac74.jsonl"] = json!(transcript);
    let (exit, report) = run(&input, "false-reason");
    assert_eq!(exit, 0);
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl"
    );
}

#[test]
fn structured_denial_reason_retains_legacy_rendering() {
    let mut input = happy();
    let transcript = input["files"]["turns/0002-01-73a4ac74.jsonl"]
        .as_str()
        .unwrap()
        .replace(
            "\"reason\": \"this step only admits `file.read` operations\"",
            "\"reason\": {\"why\": true, \"limits\": [1, 2]}",
        );
    input["files"]["turns/0002-01-73a4ac74.jsonl"] = json!(transcript);
    let (exit, report) = run(&input, "structured-reason");
    assert_eq!(exit, 0);
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {'why': True, 'limits': [1, 2]}"
    );
}

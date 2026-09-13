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
        .arg(root)
        .arg("--json")
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

fn reason_input(raw_reason: &str) -> Value {
    let mut input = happy();
    let before = input["files"]["turns/0002-01-73a4ac74.jsonl"]
        .as_str()
        .unwrap();
    let after = before.replace(
        "\"reason\": \"this step only admits `file.read` operations\"",
        &format!("\"reason\": {raw_reason}"),
    );
    assert_ne!(before, after);
    input["files"]["turns/0002-01-73a4ac74.jsonl"] = json!(after);
    input
}

#[test]
fn nested_combining_unicode_keeps_python_printable_spelling() {
    let input = reason_input(r#"["Cafe\u0301"]"#);
    let (exit, report) = run(&input, "combining-reason");
    assert_eq!(exit, 0);
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['Cafe\u{0301}']"
    );
}

#[test]
fn nan_reason_keeps_legacy_decision_record() {
    let input = reason_input("NaN");
    let (exit, report) = run(&input, "nan-reason");
    assert_eq!(
        exit, 0,
        "the legacy reader retains this tool.decided: {report}"
    );
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — nan"
    );
}

#[test]
fn escaped_lone_surrogate_reason_keeps_legacy_decision_record() {
    let input = reason_input(r#"["\ud800"]"#);
    let (exit, report) = run(&input, "surrogate-reason");
    assert_eq!(
        exit, 0,
        "the legacy reader retains this tool.decided: {report}"
    );
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['\\ud800']"
    );
}

#[test]
fn duplicate_members_keep_first_position_and_last_value() {
    let input = reason_input(r#"{"z": 1, "a": 2, "z": 3}"#);
    let (exit, report) = run(&input, "duplicate-members");
    assert_eq!(exit, 0);
    assert_eq!(
        report["clauses"][4]["found"],
        "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {'z': 3, 'a': 2}"
    );
}

#[test]
fn runtime_json_order_and_wide_number_encoding_stay_local() {
    let raw = r#"{"z": 18446744073709551616, "a": 2}"#;
    let runtime: Value = serde_json::from_str(raw).unwrap();
    let runtime_text = runtime.to_string();
    assert!(
        runtime_text.starts_with("{\"a\":2,\"z\":"),
        "{runtime_text}"
    );
    assert!(
        runtime["z"].is_f64(),
        "arbitrary_precision must stay disabled"
    );
    let evidence: swarm_check::evidence::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(evidence.python(), "{'z': 18446744073709551616, 'a': 2}");
}

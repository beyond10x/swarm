mod support;
#[test]
fn malformed_event_payloads() {
    support::assert_case("malformed_event_payloads");
}

#[test]
fn malformed_json_lines() {
    support::assert_case("malformed_json_lines");
}

#[test]
fn missing_turns() {
    support::assert_case("missing_turns");
}

#[test]
fn legacy_turn_names() {
    support::assert_case("legacy_turn_names");
}

#[test]
fn denial_words() {
    support::assert_case("denial_words");
}

#[test]
fn bare_denial() {
    support::assert_case("bare_denial");
}

#[test]
fn bound_object() {
    support::assert_case("bound_object");
}

#[test]
fn missing_assignment_identity() {
    support::assert_case("missing_assignment_identity");
}

#[test]
fn unknown_issuer_mark() {
    support::assert_case("unknown_issuer_mark");
}

#[test]
fn missing_database() {
    support::assert_case("missing_database");
}

#[test]
fn corrupt_database() {
    support::assert_case("corrupt_database");
}

fn happy(root: &std::path::Path) {
    let corpus = support::corpus();
    support::materialize(
        &corpus["test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven"]
            [0]["input"],
        root,
    );
}

#[test]
fn slug_with_data_root_has_the_same_json_and_exit_status() {
    let tmp = tempdir::TempDir::new("swarm-check-slug").unwrap();
    let root = tmp.path().join("slug");
    happy(&root);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_swarm-check"))
        .args(["slug", "--data"])
        .arg(tmp.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let actual: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        actual,
        serde_json::to_value(swarm_check::check(&root)).unwrap()
    );
}

#[test]
fn absent_and_corrupt_sqlite_are_not_created_or_modified() {
    let tmp = tempdir::TempDir::new("swarm-check-sqlite").unwrap();
    let root = tmp.path();
    let absent = swarm_check::check(root);
    assert!(!absent.ok);
    assert!(!root.join("eventlog.sqlite3").exists());
    std::fs::write(root.join("eventlog.sqlite3"), b"corrupt database").unwrap();
    let corrupt = swarm_check::check(root);
    assert!(!corrupt.ok);
    assert_eq!(
        std::fs::read(root.join("eventlog.sqlite3")).unwrap(),
        b"corrupt database"
    );
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}

#[cfg(unix)]
fn unreadable_file_refuses(name: &str) {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempdir::TempDir::new("swarm-check-unreadable").unwrap();
    let root = tmp.path().join("swarm");
    happy(&root);
    let path = if name == "transcript" {
        std::fs::read_dir(root.join("turns"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("0001"))
            .unwrap()
    } else {
        root.join("turns").join(name)
    };
    if !path.exists() {
        std::fs::write(&path, b"{}").unwrap();
    }
    let previous = std::fs::metadata(&path).unwrap().permissions();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o0)).unwrap();
    let inaccessible = std::fs::read(&path).is_err();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_swarm-check"))
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    std::fs::set_permissions(&path, previous).unwrap();
    assert!(
        inaccessible,
        "test must actually deny the read; run as an unprivileged user"
    );
    assert_eq!(
        result.status.code(),
        Some(1),
        "an inaccessible evidence file must not produce success: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}
#[test]
#[cfg(unix)]
fn unreadable_transcript_fails_closed() {
    unreadable_file_refuses("transcript");
}
#[test]
#[cfg(unix)]
fn unreadable_capped_record_fails_closed() {
    unreadable_file_refuses("capped.jsonl");
}

// This target uses the shared materializer, while its fixtures come from a separate corpus.
#[allow(dead_code)]
#[path = "support/fixtures.rs"]
mod fixtures;
use serde_json::Value;
use std::process::Command;
fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn normalize_path(mut bytes: Vec<u8>, root: &str) -> Vec<u8> {
    let path = root.as_bytes();
    while let Some(at) = bytes.windows(path.len()).position(|part| part == path) {
        bytes.splice(at..at + path.len(), b"$ROOT".iter().copied());
    }
    bytes
}
fn words(bytes: &[u8]) -> Vec<&[u8]> {
    bytes
        .split(|b| b.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .collect()
}
fn assert_group(name: &str) {
    let corpus: Value = serde_json::from_str(include_str!("fixtures/unicode-parser.json")).unwrap();
    let cases = corpus[name].as_array().unwrap();
    assert!(!cases.is_empty());
    for (index, case) in cases.iter().enumerate() {
        let temp = tempdir::TempDir::new("swarm-check-unicode").unwrap();
        let root = temp.path().join("swarm");
        fixtures::materialize(&case["input"], &root);
        let output = Command::new(env!("CARGO_BIN_EXE_swarm-check"))
            .arg(&root)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            case["json_exit"].as_i64().map(|n| n as i32),
            "{name} #{index} {:?}",
            case["reason"]
        );
        let actual = normalize_path(output.stdout, root.to_str().unwrap());
        let actual: swarm_check::evidence::Value = serde_json::from_slice(&actual).unwrap();
        let expected: swarm_check::evidence::Value =
            serde_json::from_str(case["json"].as_str().unwrap()).unwrap();
        assert_eq!(actual, expected, "{name} #{index} {:?}", case["reason"]);
        let output = Command::new(env!("CARGO_BIN_EXE_swarm-check"))
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            case["text_exit"].as_i64().map(|n| n as i32),
            "text {name} #{index} {:?}",
            case["reason"]
        );
        let actual = normalize_path(output.stdout, root.to_str().unwrap());
        let expected = bytes(case["text_hex"].as_str().unwrap());
        if matches!(
            name,
            "sqlite_text_transport_boundaries" | "deep_legacy_readable_evidence"
        ) {
            // These unmet reports wrap a hyphenated word between lines in the baseline.
            // Exact found-string equality is already checked above; wrapping may differ.
            let compact = |b: &[u8]| {
                b.iter()
                    .copied()
                    .filter(|b| !b.is_ascii_whitespace())
                    .collect::<Vec<_>>()
            };
            assert_eq!(compact(&actual), compact(&expected), "text {name} #{index}");
            continue;
        }
        assert_eq!(
            words(&actual),
            words(&expected),
            "text {name} #{index} {:?}",
            case["reason"]
        );
    }
}

#[test]
fn unicode_printable_categories() {
    assert_group("unicode_printable_categories");
}

#[test]
fn unicode_other_and_separator_categories() {
    assert_group("unicode_other_and_separator_categories");
}

#[test]
fn ascii_space_stays_printable() {
    assert_group("ascii_space_stays_printable");
}

#[test]
fn legacy_nonfinite_constants() {
    assert_group("legacy_nonfinite_constants");
}

#[test]
fn nested_unpaired_surrogates() {
    assert_group("nested_unpaired_surrogates");
}

#[test]
fn top_level_surrogates_distinguish_json_and_text() {
    assert_group("top_level_surrogates_distinguish_json_and_text");
}

#[test]
fn surrogateescape_text_bytes() {
    assert_group("surrogateescape_text_bytes");
}

#[test]
fn valid_surrogate_pairs() {
    assert_group("valid_surrogate_pairs");
}

#[test]
fn ordinary_malformed_grammar_remains_rejected() {
    assert_group("ordinary_malformed_grammar_remains_rejected");
}

#[test]
fn sqlite_text_transport_boundaries() {
    assert_group("sqlite_text_transport_boundaries");
}

#[test]
fn deep_legacy_readable_evidence() {
    assert_group("deep_legacy_readable_evidence");
}

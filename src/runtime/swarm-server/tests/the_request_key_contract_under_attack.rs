//! Adversarial cases against the document-checking contract added on 2026-09-12.
//!
//! The unit under attack shipped `tests/the_request_key_contract.rs`, whose premise is that the
//! set of documents about the request key is DERIVED rather than remembered, and that any refuted
//! promise one of them carries must be marked as history. These two cases drive that premise, and
//! the corrected `http.rs` doc, against the tree.
//!
//! This file deliberately does not spell the story id, because the derivation under attack
//! enumerates every `.rs` under `tests/` that does spell it. A file written to measure that
//! derivation must not join the population it measures, or the measurement reports itself. The id
//! is assembled at run time in `story()` instead.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;

/// The story id, assembled so that this file does not literally contain it. See the module doc.
fn story() -> String {
    format!("story:{}-{}", "request-key-is", "not-idempotency")
}

/// The same sentences `tests/the_request_key_contract.rs` calls `REFUTED`, and the same markers it
/// calls `PAST`. Copied rather than shared because an integration test binary exports nothing.
const REFUTED: &[&str] = &[
    "not a second request",
    "writes nothing the second time",
    "safe to resend",
    "cannot double-write",
    "is the same guarantee",
    "guarantee is the same",
];
const PAST: &[&str] = &["used to", "no longer", "an earlier", "had written", "wrote"];

/// A refuted promise wrapped across a comment line is still a refuted promise.
///
/// `no_document_presents_a_refuted_promise_as_current` finds a refuted sentence by searching the
/// file's raw text for the sentence as one substring. Every doc comment in this crate is wrapped
/// at about a hundred columns, so a sentence of twenty or thirty characters is split by a newline
/// and a `///` whenever the wrap happens to fall inside it — and then the search finds nothing and
/// the case passes without having looked.
///
/// That is not hypothetical. It is true of the tree as shipped, in the file that defines the check,
/// in the paragraph that same case's own doc holds up as the example of a stale citation: the
/// quotation of the sentence removed from `http.rs` is split after "not a second", and its
/// paragraph carries no past-tense marker at all. The guard does not catch its own example.
///
/// So this case applies the same rule to the same derived set of documents, over text normalised
/// the way a reader reads it: comment markers stripped and runs of whitespace collapsed, so a
/// sentence is one sentence whatever column it was wrapped at.
#[test]
fn a_refuted_promise_wrapped_across_a_comment_line_is_still_a_refuted_promise() {
    let surfaces = documents_about_the_request_key();
    assert!(
        surfaces.len() >= 4,
        "the derivation should find at least the four known surfaces; it found {:?}",
        surfaces.iter().map(|(path, _)| path).collect::<Vec<_>>()
    );

    let mut unmarked: Vec<String> = Vec::new();
    for (path, text) in &surfaces {
        for paragraph in paragraphs(text) {
            let read = as_a_reader_reads_it(&paragraph);
            let carried: Vec<&str> = REFUTED
                .iter()
                .copied()
                .filter(|claim| read.contains(claim))
                .collect();
            if carried.is_empty() {
                continue;
            }
            let marked = PAST
                .iter()
                .any(|marker| paragraph.to_lowercase().contains(marker));
            if !marked {
                unmarked.push(format!("{path}: {carried:?} in:\n{read}"));
            }
        }
    }

    assert!(
        unmarked.is_empty(),
        "a promise this story measured false is quoted with nothing saying it is history, and the \
         check that exists to catch exactly that missed it because the sentence is wrapped across \
         a comment line. Matching raw file text makes the guard a function of where the wrap fell \
         ({}). Normalise the text before matching. Found:\n\n{}",
        story(),
        unmarked.join("\n\n")
    );
}

/// The log answers only within the window it returns, and both documents say so.
///
/// `http.rs` told a disconnected client that the log "carries the `request` each event was written
/// under, so it answers exactly whether this key's first attempt landed", and `runtime.ts` repeated
/// it and added "resend only if it is absent". `GET /swarms/{slug}/log` takes one parameter,
/// `limit`; `Store::history` reads the whole feed and returns the TAIL. There is no offset, no
/// cursor and no filter by request key, so once the swarm has recorded more than `limit` events
/// the endpoint cannot reach the attempt at all and reports it absent. A client following that
/// sentence resends a command that already landed, which is the failure the story exists to
/// describe.
///
/// Measured at the TypeScript client's own default, `log(slug, limit = 200)`: the attempt is gone
/// and the oldest request in the window is one of the later ones. The measurement is the adversary's
/// and is kept exactly; the assertion faces the way the measurement came out, and the documents are
/// then held to stating the window — with the default and the cap READ OUT OF `http.rs`, so a doc
/// that keeps an old number when the code changes one is red rather than quietly wrong.
#[tokio::test]
async fn the_log_answers_only_within_the_window_it_returns() {
    let (default_limit, cap) = the_window_the_log_offers();
    assert_eq!(
        (default_limit, cap),
        (200, 1000),
        "the log's window is read out of http.rs; if these moved, the documents below move with them"
    );

    let (server, _data) = server().await;
    let swarm = server.open("windowed").await.expect("a swarm opens");

    draft_a_box(&swarm, "LOST").await;

    // Whatever else the swarm goes on to do while the client is disconnected.
    for nth in 0..default_limit {
        draft_a_box(&swarm, &format!("later-{nth}")).await;
    }

    let log = swarm.history(default_limit).await.expect("the log reads");
    assert!(
        !log.iter().any(|event| event.request == "LOST"),
        "an attempt older than the window is unreachable through the log, and this one was still \
         in it — if the endpoint gained a cursor or a filter by request key, these documents may \
         stop hedging ({})",
        story()
    );
    assert_eq!(
        log.first().map(|event| event.request.as_str()),
        Some("later-0"),
        "the window is the newest `limit` events and nothing else ({})",
        story()
    );

    // So neither document may say the log answers outright, and both must state the window.
    for (path, declaration) in [HTTP, TS] {
        let doc = contract_doc(path, declaration);
        assert!(
            !doc.contains("answers exactly whether"),
            "{path}: the doc of `{declaration}` says the log answers outright whether an attempt \
             landed, and the measurement above says it cannot see past {default_limit} events \
             ({}). Read:\n{doc}",
            story()
        );
        for stated in ["window", &default_limit.to_string(), &cap.to_string()] {
            assert!(
                doc.contains(stated),
                "{path}: the doc of `{declaration}` must state the window the log answers within, \
                 \"{stated}\" included — it is read out of http.rs, so a doc holding an old \
                 number is red rather than quietly wrong ({}). Read:\n{doc}",
                story()
            );
        }
    }
}

/// The window `GET /swarms/{slug}/log` offers, read out of `http.rs`: the default `limit` and the
/// cap it is clamped to. Read rather than written down, so the documents are checked against the
/// code and not against a copy of it that ages.
fn the_window_the_log_offers() -> (usize, usize) {
    let source =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/http.rs"))
            .expect("http.rs is readable");

    let default = source
        .split_once("fn default_limit() -> usize {")
        .and_then(|(_, rest)| rest.split('}').next())
        .and_then(|body| body.trim().parse().ok())
        .expect("`default_limit` answers one literal");
    let cap = source
        .split_once("page.limit.min(")
        .and_then(|(_, rest)| rest.split(')').next())
        .and_then(|digits| digits.trim().parse().ok())
        .expect("`read_log` clamps `limit` to one literal");
    (default, cap)
}

/// The two declarations whose documents carry this promise: a path under this crate, and the line
/// whose doc block is the contract. `swarm.rs` is absent — `Delivery::request` is the pump's key
/// and never reaches a disconnected client.
const HTTP: (&str, &str) = ("src/http.rs", "pub request: Option<String>,");
const TS: (&str, &str) = ("../../web/src/runtime.ts", "export function issue(");

/// The doc block above `declaration` in `path`, read the way a reader reads it.
fn contract_doc(path: &str, declaration: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let source = std::fs::read_to_string(&file)
        .unwrap_or_else(|why| panic!("{} is readable: {why}", file.display()));
    let lines: Vec<&str> = source.lines().collect();
    let at: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim() == declaration.trim())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(at.len(), 1, "`{declaration}` must name one line in {path}");

    let mut block: Vec<&str> = Vec::new();
    let mut cursor = at[0];
    let mut in_terminated_comment = false;
    while cursor > 0 {
        cursor -= 1;
        let line = lines[cursor].trim();
        if !in_terminated_comment && (line.starts_with("#[") || line.is_empty() && block.is_empty())
        {
            continue;
        }
        if line.ends_with("*/") {
            in_terminated_comment = true;
        }
        if !(line.starts_with("///") || line.starts_with('*') || in_terminated_comment) {
            break;
        }
        block.push(lines[cursor]);
        if line.starts_with("/*") {
            break;
        }
    }
    block.reverse();
    as_a_reader_reads_it(&block.join("\n"))
}

// ---------------------------------------------------------------------------------------------
// The same derivation the case under attack performs, and the reading normalisation it does not.

/// Every `.rs` and `.ts` file under this crate's `src/`, `tests/` and the web client that names the
/// story, with its text — the population `documents_about_the_request_key` builds.
fn documents_about_the_request_key() -> Vec<(String, String)> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for root in ["src", "tests", "../../web/src"] {
        let mut files = Vec::new();
        collect(&crate_root.join(root), &mut files);
        files.sort();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            if !text.contains(&story()) {
                continue;
            }
            let shown = file
                .strip_prefix(&crate_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .into_owned();
            found.push((shown, text));
        }
    }
    found
}

/// Every source file under `dir` a reader could meet this contract in, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if matches!(
            path.extension().and_then(|kind| kind.to_str()),
            Some("rs") | Some("ts") | Some("vue")
        ) {
            into.push(path);
        }
    }
}

/// The file split into paragraphs on comment-blank and blank lines, the same cut
/// `paragraph_around` makes.
fn paragraphs(text: &str) -> Vec<String> {
    let empty = |line: &str| {
        let line = line
            .trim()
            .trim_start_matches("///")
            .trim_start_matches("//!");
        line.trim().trim_start_matches('*').trim().is_empty()
    };
    let mut out = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if empty(line) {
            if !current.is_empty() {
                out.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        out.push(current.join("\n"));
    }
    out
}

/// One paragraph as the sentence a reader reads: comment markers off, whitespace collapsed,
/// lowercased. A wrapped sentence is one sentence here, which is the whole point.
fn as_a_reader_reads_it(paragraph: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for line in paragraph.lines() {
        let line = line.trim();
        let line = line
            .strip_prefix("///")
            .or_else(|| line.strip_prefix("//!"))
            .or_else(|| line.strip_prefix("//"))
            .or_else(|| line.strip_prefix('*'))
            .unwrap_or(line);
        words.extend(line.split_whitespace().map(|word| word.to_lowercase()));
    }
    words.join(" ")
}

// ---------------------------------------------------------------------------------------------
// The same fixtures the case under attack uses.

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-contract-attack").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

const GHOST: &str = "44444444-4444-4444-8444-444444444444";

/// Puts one box on a canvas under `request`.
async fn draft_a_box(swarm: &Arc<Swarm>, request: &str) {
    let drafted = swarm
        .issue(
            None,
            "swarm.blackbox.DraftBox",
            args(json!({
                "swarm_id": GHOST,
                "name": "a box",
                "kind": "Service",
                "owner_agent": null,
                "summary": null,
                "inputs": [],
                "outputs": [],
                "ref_id": null,
                "props": null,
                "agent_id": null,
                "position": {"x": 1, "y": 1},
            })),
            request,
        )
        .await
        .expect("a box is drafted");
    assert_eq!(drafted.outcome, "drafted");
}

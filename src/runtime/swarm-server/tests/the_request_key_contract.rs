//! What the request key's documents say, and whether the code says the same.
//!
//! The operator chose the doc branch for `story:request-key-is-not-idempotency`: restate the
//! contract, change no behaviour. So the documents ARE the deliverable, and these cases drive the
//! code from them. Nothing here asserts that a request key ought to be idempotent; that is the
//! branch the operator did not take, and it is not what is being measured.
//!
//! Three of the four cases arrived from the adversary as
//! `omitting_the_request_key_is_the_same_guarantee_as_repeating_one`,
//! `the_canvas_carries_the_request_every_record_was_written_under` and
//! `a_quotation_of_http_rs_is_a_quotation_of_what_http_rs_says`. Each was red and each was right to
//! be: the measurements are kept verbatim, and each assertion has been turned to face the truth the
//! measurement established, so that the case stays red until the document agrees with it and goes
//! red again if the document drifts back. The fourth moved here from
//! `tests/redelivery_under_attack.rs`, unchanged in name, because the enumeration of surfaces and
//! the sentences it refuses belong in one file with the cases that refuted them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-contract").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// A key nobody else will use, which is what `http.rs` mints when the field is omitted:
/// `body.request.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())`.
fn minted() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!("minted-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

const GHOST: &str = "33333333-3333-4333-8333-333333333333";

/// Puts one box on a canvas and answers its id.
async fn draft_a_box(swarm: &Arc<Swarm>, request: &str) -> String {
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

    swarm.instances("swarm.blackbox.Box").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned()
}

/// Moves it to the SAME place every time: one command, one input, so the events a second
/// application emits are byte-identical to the first's. This is the only shape in which
/// `store.rs`'s append guard can fire at all.
async fn move_it(swarm: &Arc<Swarm>, box_id: &str, request: &str) -> Result<String, String> {
    swarm
        .issue(
            None,
            "swarm.blackbox.MoveBox",
            args(json!({"box_id": box_id, "position": {"x": 7, "y": 7}})),
            request,
        )
        .await
        .map(|issued| issued.outcome)
        .map_err(|why| why.to_string())
}

/// How many `BoxMoved` events reached the log.
async fn moves_in_the_log(swarm: &Arc<Swarm>) -> usize {
    swarm
        .history(1000)
        .await
        .expect("the log reads")
        .iter()
        .filter(|event| event.name == "swarm.blackbox.BoxMoved")
        .count()
}

/// Omitting the request key is not the same guarantee as repeating one, and both docs say so.
///
/// This contract used to end "Omitting the field is honest — the server mints a fresh key — and
/// is the same guarantee." It is not. The guarantee named immediately above it is
/// "it will not double-APPEND to a stream the key was already spent on", and a minted key has been
/// spent on no stream, so the guard cannot fire. `Issue::request` is `#[serde(default)]`, so every
/// HTTP caller that leaves the field out takes that path.
///
/// Measured by the adversary on `MoveBox`, the one command in the kernel whose second application
/// emits events identical to its first — the only case where a repeated key appends nothing, and
/// so the only case where the two halves of that sentence can be compared at all. The measurement
/// is kept exactly; what changed is which way the assertion points, because the difference is the
/// finding. Both documents must now say that omitting the field gives the guard up.
#[tokio::test]
async fn omitting_the_request_key_gives_up_the_only_guarantee_it_has() {
    let (server, _data) = server().await;

    let kept = server.open("kept").await.expect("a swarm opens");
    let box_id = draft_a_box(&kept, "draft-kept").await;
    assert_eq!(
        move_it(&kept, &box_id, "SAME").await,
        Ok("moved".to_owned())
    );
    assert_eq!(
        move_it(&kept, &box_id, "SAME").await,
        Ok("moved".to_owned())
    );
    let under_one_key = moves_in_the_log(&kept).await;

    let omitted = server.open("omitted").await.expect("a swarm opens");
    let box_id = draft_a_box(&omitted, "draft-omitted").await;
    let first = move_it(&omitted, &box_id, &minted()).await;
    let second = move_it(&omitted, &box_id, &minted()).await;
    assert_eq!(first, Ok("moved".to_owned()));
    assert_eq!(second, Ok("moved".to_owned()));
    let under_minted_keys = moves_in_the_log(&omitted).await;

    assert_eq!(
        (under_one_key, under_minted_keys),
        (1, 2),
        "one key spent twice appends once and two minted keys append twice; that is the whole of \
         what the field buys (story:request-key-is-not-idempotency)"
    );

    // So neither document may call the omission the same guarantee, and both must name the cost.
    for (path, declaration) in [HTTP, TS] {
        let doc = contract_doc(path, declaration).to_lowercase();
        for (_, claim) in refuted_in(&doc) {
            assert!(
                !claim.contains("guarantee"),
                "{path}: the doc of `{declaration}` says omitting the key leaves the guarantee \
                 untouched — \"{claim}\" — and the measurement above says it is \
                 {under_one_key} append against {under_minted_keys} \
                 (story:request-key-is-not-idempotency)"
            );
        }
        assert!(
            doc.contains("gives up")
                || doc.contains("gives it up")
                || doc.contains("gives that up"),
            "{path}: the doc of `{declaration}` must say that omitting the field gives up the \
             append guard, which is the only guarantee it has \
             (story:request-key-is-not-idempotency). Read:\n{doc}"
        );
    }
}

/// Only the log carries the `request` a record was written under, so only the log may be offered
/// for one.
///
/// `runtime.ts` sent a caller to the canvas OR the log "because every record carries the `request`
/// it was written under". The log's do: `Recorded.request` is `event.request_id` (`store.rs`). A
/// canvas record is an instance — `entity`, `id`, `identity_field`, `state`, `fields`, `revision`
/// — and carries no such field, so a caller that followed the sentence there would look for a key
/// that is never present and conclude every first attempt was absent. `http.rs` stated the pair
/// correctly in the same commit, so the two surfaces the check below calls one contract disagreed.
///
/// Both halves are asserted: the measured shape of each, and the rule that a sentence attributing
/// `request` to something a reader can GET names the log and not the canvas — on both surfaces, so
/// neither can drift alone.
#[tokio::test]
async fn only_the_log_carries_the_request_a_record_was_written_under() {
    let (server, _data) = server().await;
    let swarm = server.open("canvas").await.expect("a swarm opens");
    draft_a_box(&swarm, "draft-canvas").await;

    let canvas = swarm.instances("swarm.blackbox.Box").await;
    assert_eq!(canvas.len(), 1, "one box was drafted");
    for record in &canvas {
        assert!(
            record.get("request").is_none(),
            "a canvas record is an instance and carries no request key; this one does, which means \
             the canvas is now a place to look one up and these documents may say so: {record} \
             (story:request-key-is-not-idempotency)"
        );
    }

    let log = swarm.history(1000).await.expect("the log reads");
    assert!(
        log.iter().any(|event| event.request == "draft-canvas"),
        "the log carries the request each event was written under, which is what makes it the \
         answer for a client that lost its response (story:request-key-is-not-idempotency)"
    );

    for (path, declaration) in [HTTP, TS] {
        let doc = contract_doc(path, declaration);
        for sentence in doc.to_lowercase().split(['.', ';']) {
            if !(sentence.contains("carries") && sentence.contains("request")) {
                continue;
            }
            assert!(
                !sentence.contains("canvas"),
                "{path}: the doc of `{declaration}` tells a reader the canvas carries the request \
                 a record was written under, and the canvas record measured above carries no such \
                 field. Only the log does (story:request-key-is-not-idempotency). Read:\n\
                 {sentence}"
            );
        }
    }
}

/// A document that quotes a refuted promise says whose it WAS.
///
/// The unit named its own defect class precisely: one claim about the request key copied to
/// several reader-facing surfaces, corrected one at a time, each correction invisible to the
/// others. Its first check enumerated three paths by hand, and the gap was realised the same day on
/// a fourth: `tests/redelivery_under_attack.rs` said in the present tense that "`http.rs` documents
/// `Issue::request` as \"the idempotency key. Retrying with the same one is a retry, not a second
/// request\"" — the sentence the same commit had removed from `http.rs`. Both corrected surfaces
/// send the reader to that file by name, so it is on the documented path of anybody who wants the
/// measurement.
///
/// So this case does not take a list of paths. It DERIVES one: every source file under this crate
/// and the web client that names the story is a document about the request key, and any refuted
/// sentence it carries must be marked as history within the paragraph that carries it — "used to",
/// "no longer", "an earlier", "had written". That is the one thing a stale citation cannot have,
/// and it costs a quotation nothing.
#[test]
fn no_document_presents_a_refuted_promise_as_current() {
    let surfaces = documents_about_the_request_key();
    for known in [HTTP.0, SWARM.0, TS.0, "tests/redelivery_under_attack.rs"] {
        assert!(
            surfaces.iter().any(|(path, _)| path == known),
            "{known} names story:request-key-is-not-idempotency and must be among the documents \
             this case derived; it found {:?}. A derivation that stopped finding the surfaces is \
             the same defect as a hand-written list that never had them",
            surfaces.iter().map(|(path, _)| path).collect::<Vec<_>>()
        );
    }

    for (path, text) in &surfaces {
        for (at, _) in refuted_in(text) {
            let paragraph = paragraph_around(text, at);
            assert!(
                PAST.iter().any(|marker| paragraph.contains(marker)),
                "{path}: a promise this story measured false is quoted here with nothing saying \
                 it is history, so a reader takes it as what the surface says today. Write it in \
                 the past tense — {PAST:?} — or delete it \
                 (story:request-key-is-not-idempotency). Read:\n{paragraph}"
            );
        }
    }
}

/// Every reader-facing document of the request key states the same contract.
///
/// `tests/redelivery_under_attack.rs` pins the BEHAVIOUR. Nothing there stops a document drifting
/// back to the promise the behaviour does not keep, and drifting back is what happened: `http.rs`
/// used to say "retrying with the same one is a retry, not a second request", `runtime.ts` said
/// the same in other words to a TypeScript caller, and `swarm.rs` rested a redelivery argument on
/// it. Three copies of one claim, corrected one at a time, each correction invisible to the other
/// two.
///
/// So the check is the fix. This case takes the three declarations a caller meets the key through
/// and asserts of each doc block that it names the true scope — the key is spent on one INSTANCE'S
/// STREAM — that it names `story:request-key-is-not-idempotency` so a reader can find the
/// measurement, and that it makes none of the refuted promises. The wider class, a document
/// anywhere that repeats one, is the derived case above.
///
/// `ess-runtime`'s `Store::commit` is deliberately absent: it is the one document that was always
/// right, it is the definition these restate, and it is not this crate's to hold.
#[test]
fn every_document_of_the_request_key_states_the_same_contract() {
    for (path, declaration) in [HTTP, SWARM, TS] {
        let doc = contract_doc(path, declaration);
        let stated = doc.to_lowercase();

        assert!(
            stated.contains("instance's stream"),
            "{path}: the doc of `{declaration}` must say the key is scoped to the instance's \
             stream, which is what it actually guards (story:request-key-is-not-idempotency). \
             Read:\n{doc}"
        );
        assert!(
            stated.contains("story:request-key-is-not-idempotency"),
            "{path}: the doc of `{declaration}` must name \
             story:request-key-is-not-idempotency, where the measurements live. Read:\n{doc}"
        );
        let promised: Vec<&str> = refuted_in(&stated).into_iter().map(|(_, c)| c).collect();
        assert!(
            promised.is_empty(),
            "{path}: the doc of `{declaration}` promises {promised:?}, which the cases in \
             tests/redelivery_under_attack.rs and above measure false \
             (story:request-key-is-not-idempotency). If the BEHAVIOUR changed, change those cases \
             first and this line after. Read:\n{doc}"
        );
    }
}

/// The three declarations a caller meets the request key through: a path under this crate, and the
/// line whose doc block is the contract.
const HTTP: (&str, &str) = ("src/http.rs", "pub request: Option<String>,");
const SWARM: (&str, &str) = ("src/swarm.rs", "request: String,");
const TS: (&str, &str) = ("../../web/src/runtime.ts", "export function issue(");

/// Sentences that used to be on one of these surfaces and are not true of the code. Historical
/// facts, so this list is written down rather than derived; everything else here derives from it.
const REFUTED: &[&str] = &[
    "not a second request",
    "writes nothing the second time",
    "safe to resend",
    "cannot double-write",
    "is the same guarantee",
    "guarantee is the same",
];

/// What marks a quotation as history rather than as what a surface says now.
const PAST: &[&str] = &["used to", "no longer", "an earlier", "had written", "wrote"];

/// Where each refuted sentence sits in `text`, lowercased, and which one it is.
fn refuted_in(text: &str) -> Vec<(usize, &'static str)> {
    let said = text.to_lowercase();
    let mut found = Vec::new();
    for claim in REFUTED {
        let mut from = 0;
        while let Some(at) = said[from..].find(claim) {
            found.push((from + at, *claim));
            from += at + claim.len();
        }
    }
    found
}

/// The comment paragraph `at` sits in: back to the last blank comment line or non-comment line,
/// forward to the next. A quotation and its attribution are one paragraph.
fn paragraph_around(text: &str, at: usize) -> String {
    let empty = |line: &str| {
        let line = line
            .trim()
            .trim_start_matches("///")
            .trim_start_matches("//!");
        line.trim().trim_start_matches('*').trim().is_empty()
    };
    let mut start = 0;
    let mut end = text.len();
    let mut cursor = 0;
    for line in text.split_inclusive('\n') {
        let next = cursor + line.len();
        if empty(line) {
            if next <= at {
                start = next;
            } else if cursor >= at {
                end = cursor;
                break;
            }
        }
        cursor = next;
    }
    text[start..end].to_string()
}

/// Every source file under this crate and the web client that names the story, with its text. The
/// enumeration is derived so that a fifth document joins it by being written, not by being
/// remembered.
fn documents_about_the_request_key() -> Vec<(String, String)> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for root in ["src", "tests", "../../web/src"] {
        let mut files = Vec::new();
        collect(&crate_root.join(root), &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            if !text.contains("story:request-key-is-not-idempotency") {
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

/// Every `.rs` and `.ts` file under `dir`, recursively.
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
            Some("rs") | Some("ts")
        ) {
            into.push(path);
        }
    }
}

/// The doc block above `declaration` in `path`, which is the contract that declaration carries.
fn contract_doc(path: &str, declaration: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let source = std::fs::read_to_string(&file)
        .unwrap_or_else(|why| panic!("{} is readable: {why}", file.display()));
    doc_block_above(&source, declaration)
        .unwrap_or_else(|| panic!("{path} declares `{declaration}` with a doc block above it"))
}

/// The comment block immediately above `declaration`, attributes skipped. Handles `///` and
/// `/** … */`, which is every form these files use.
///
/// The declaration is matched on a whole trimmed line and is required to be UNIQUE. A short one
/// like `request: String,` could otherwise match an earlier field of another struct and silently
/// redirect the whole check at a doc block nobody meant — a green run against the wrong text,
/// which is the one failure a check of documents cannot afford.
fn doc_block_above(source: &str, declaration: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let at: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim() == declaration.trim())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        at.len(),
        1,
        "`{declaration}` must name one line to hang a contract on, and it names {} (lines {:?}). \
         Make it longer until it is unique",
        at.len(),
        at.iter().map(|index| index + 1).collect::<Vec<_>>()
    );
    let at = at[0];

    let mut block: Vec<&str> = Vec::new();
    let mut cursor = at;
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
        let is_comment = line.starts_with("///") || line.starts_with("*") || in_terminated_comment;
        if !is_comment {
            break;
        }
        block.push(lines[cursor]);
        if line.starts_with("/*") {
            break;
        }
    }
    if block.is_empty() {
        return None;
    }
    block.reverse();
    Some(block.join("\n"))
}

//! Adversarial cases against the redelivery queue added on 2026-09-12.
//!
//! Both cases were written by the adversary against two documents the implementor had written:
//! `http.rs`'s "retrying with the same one is a retry, not a second request", and `swarm.rs`'s
//! claim on `Delivery::request` that "at most one attempt can append ... so a duplicate delivery
//! cannot double-write". Both were red, and both were right to be.
//!
//! The `swarm.rs` claim was wrong and has been corrected in place rather than defended: an
//! idempotency key is scoped to one instance's stream (`store.rs`), so it cannot refuse an attempt
//! that writes to a stream it was never spent on. The `http.rs` contract is not kept by the code
//! either, and that is older and wider than the redelivery queue — it is filed as
//! `story:request-key-is-not-idempotency`.
//!
//! So each case below now asserts what the code DOES, with the story named in the assertion
//! message. They are characterisation, not approval: the day somebody makes a request key mean
//! what both documents say, these two go red and point at the line that says so. Leaving them red
//! today was the alternative, and a red case is read by the suite's exit status, which would have
//! deleted every result after it.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};

use ess_runtime::Spec;
use swarm_server::state::Server;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-adversary").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// What a client retry under the same idempotency key actually does today: a different answer.
///
/// `http.rs` documents `Issue::request` as "the idempotency key. Retrying with the same one is a
/// retry, not a second request." A client whose connection dropped after the server committed does
/// exactly this. The key reaches the log, where it makes the APPEND idempotent — but `Swarm::issue`
/// applies the command again first, against the world the first one left, and the specification
/// answers the second `ActivateConfig` with `wrong-state` because the config it names is already
/// Active. The caller is told its request failed when its request succeeded.
///
/// The delivery queue survives this by luck rather than by design: no second `ConfigActivated` is
/// emitted, so the pump does not run, so nothing is owed twice. Against a command the model lets
/// through twice, two deliveries would be owed and — for a binding whose command CREATES — both
/// could land, because the shared key would be spent on two different streams.
///
/// Asserted as it stands, under `story:request-key-is-not-idempotency`. When that is fixed this
/// case fails on `wrong-state`, which is the notice that this comment needs deleting.
#[tokio::test]
async fn a_retry_under_the_same_request_key_answers_wrong_state_today() {
    let (server, _data) = server().await;
    let swarm = server.open("retried").await.expect("a swarm opens");

    // A config naming a swarm this log has never seen: `adopt-activated-config` will fail, and it
    // declares at_least_once/retry.
    let ghost = "11111111-1111-4111-8111-111111111111";
    swarm
        .issue(
            None,
            "swarm.config.DraftConfig",
            args(
                json!({"swarm_id": ghost, "paths": {}, "launch": {}, "schedules": {},
                        "budgets": {}, "board": {}}),
            ),
            "draft",
        )
        .await
        .expect("a config is drafted");
    let config = swarm.instances("swarm.config.Config").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    let activate = json!({"config_id": config, "swarm_id": ghost});

    let first = swarm
        .issue(
            None,
            "swarm.config.ActivateConfig",
            args(activate.clone()),
            "K",
        )
        .await
        .map(|issued| issued.outcome)
        .map_err(|why| why.to_string());
    assert_eq!(first, Ok("activated".to_owned()));
    assert_eq!(
        swarm.deliveries().await.len(),
        1,
        "the failed at_least_once delivery is owed once"
    );

    // The same request, retried by a client that did not hear the answer. Same key, by contract.
    let second = swarm
        .issue(None, "swarm.config.ActivateConfig", args(activate), "K")
        .await
        .map(|issued| issued.outcome)
        .map_err(|why| why.to_string());

    assert_eq!(
        second,
        Ok("wrong-state".to_owned()),
        "a request key is not idempotency: the command is re-applied and the model refuses it \
         (story:request-key-is-not-idempotency — fixing that contract turns this red)"
    );

    // The one thing the queue must hold whatever the answer was: one delivery of one binding under
    // one request key is owed once. This is the redelivery queue's own property and it stands.
    let owed = swarm.deliveries().await;
    assert_eq!(
        owed.len(),
        1,
        "a retry of one request owes one delivery, not {} (owed: {owed:?})",
        owed.len()
    );
    assert_eq!(owed[0].binding, "adopt-activated-config");
    assert_eq!(owed[0].attempts, 1);
}

/// The premise `Delivery::request` used to rest on, measured: a spent key refuses nothing.
///
/// The implementor wrote of the key every attempt of a delivery commits under: "a key spent with
/// the same events is a no-op and a key spent with different ones is refused, so a duplicate
/// delivery cannot double-write." `store.rs` scopes the key to the INSTANCE'S STREAM. A command
/// that creates an instance mints a fresh id on every attempt, so every attempt writes to a stream
/// where the key has never been spent and nothing refuses anything. `record-the-assignment` invokes
/// `RecordAssignment`, which creates an `Assignment` — so the claim did not hold for one of the
/// three bindings this queue exists to serve. The comment has been corrected; this case is what
/// corrected it, kept so the correction cannot quietly rot back.
///
/// Demonstrated on the smallest creating command in the kernel. Two Configs from one key is the
/// current behaviour and is asserted as such under `story:request-key-is-not-idempotency`.
#[tokio::test]
async fn one_request_key_spent_twice_on_a_creating_command_creates_two_instances_today() {
    let (server, _data) = server().await;
    let swarm = server.open("keyed").await.expect("a swarm opens");

    let ghost = "22222222-2222-4222-8222-222222222222";
    let draft = json!({"swarm_id": ghost, "paths": {}, "launch": {}, "schedules": {},
                       "budgets": {}, "board": {}});

    let first = swarm
        .issue(
            None,
            "swarm.config.DraftConfig",
            args(draft.clone()),
            "same",
        )
        .await
        .map(|issued| issued.outcome)
        .map_err(|why| why.to_string());
    let second = swarm
        .issue(None, "swarm.config.DraftConfig", args(draft), "same")
        .await
        .map(|issued| issued.outcome)
        .map_err(|why| why.to_string());
    assert_eq!(first, Ok("drafted".to_owned()));
    assert_eq!(second, Ok("drafted".to_owned()));

    let configs = swarm.instances("swarm.config.Config").await;
    assert_eq!(
        configs.len(),
        2,
        "a key spent on a creating command refuses nothing, because the second attempt is a new \
         stream (story:request-key-is-not-idempotency — fixing that contract turns this red)"
    );

    // Two streams, two identities. The key is the same on both, which is the whole point.
    let ids: Vec<&str> = configs
        .iter()
        .filter_map(|config| config["id"].as_str())
        .collect();
    assert_ne!(ids[0], ids[1], "each attempt minted its own instance");
}

/// Every reader-facing document of the request key says the same true thing about it.
///
/// The two cases above pin the BEHAVIOUR. They cannot stop a document drifting back to the promise
/// the behaviour does not keep, and drifting back is exactly what happened: `http.rs` said
/// "retrying with the same one is a retry, not a second request", `runtime.ts` said the same in
/// other words to a TypeScript caller, and `swarm.rs` rested a redelivery argument on it. Three
/// copies of one claim, corrected one at a time, each correction invisible to the other two.
///
/// So the check is the fix. This case enumerates the surfaces a caller reads the request key
/// through and asserts of each doc block that it (a) names the true scope — the key is spent on one
/// INSTANCE'S STREAM — (b) names `story:request-key-is-not-idempotency`, so a reader who wants the
/// measurement can find it, and (c) repeats none of the promises measured false. Adding a fourth
/// surface means adding it here; that is the point.
///
/// `ess-runtime`'s `Store::commit` is deliberately absent: it is the one document that was always
/// right, it is the definition the three restate, and it is not this crate's to hold.
#[test]
fn every_document_of_the_request_key_states_the_same_contract() {
    // path, relative to this crate — and the declaration whose doc block is the contract.
    let surfaces = [
        ("src/http.rs", "pub request: Option<String>,"),
        ("src/swarm.rs", "request: String,"),
        ("../../web/src/runtime.ts", "export function issue("),
    ];

    // Sentences that were on one of these surfaces and are not true of the code.
    let refuted = [
        "not a second request",
        "writes nothing the second time",
        "safe to resend",
        "cannot double-write",
    ];

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (path, declaration) in surfaces {
        let file = root.join(path);
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|why| panic!("{} is readable: {why}", file.display()));
        let doc = doc_block_above(&source, declaration)
            .unwrap_or_else(|| panic!("{path} declares `{declaration}` with a doc block above it"));
        let said = doc.to_lowercase();

        assert!(
            said.contains("instance's stream"),
            "{path}: the doc of `{declaration}` must say the key is scoped to the instance's \
             stream, which is what it actually guards (story:request-key-is-not-idempotency). \
             Read:\n{doc}"
        );
        assert!(
            said.contains("story:request-key-is-not-idempotency"),
            "{path}: the doc of `{declaration}` must name \
             story:request-key-is-not-idempotency, where the two measurements live. Read:\n{doc}"
        );
        for claim in refuted {
            assert!(
                !said.contains(claim),
                "{path}: the doc of `{declaration}` promises \"{claim}\", which the two cases \
                 above measure false (story:request-key-is-not-idempotency). If the BEHAVIOUR \
                 changed, change those cases first and this line after. Read:\n{doc}"
            );
        }
    }
}

/// The comment block immediately above `declaration`, attributes skipped, or `None` if there is
/// none. Handles `///` and `/** … */`, which is every form these three files use.
fn doc_block_above(source: &str, declaration: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == declaration.trim())?;

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

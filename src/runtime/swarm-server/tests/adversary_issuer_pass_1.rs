//! Adversary, wave 2026-09-13b unit A, pass 1 — acceptance clause 1 against the `agent:?` fallback.
//!
//! The story's clause 1 is: "An event records **which agent instance** caused it, distinguishably
//! from another agent of the same actor type." The unit's own test file covers ONE member whose
//! name the envelope's identity column cannot hold
//! (`an_event_says_which_agent_acted.rs::a_name_the_log_cannot_hold_is_neither_refused_nor_
//! recorded_as_somebody_else`) and asserts only that it is not recorded as `operator` or
//! `runtime`. The boundary it does not walk is TWO.
//!
//! `Issuer::UnnameableAgent` has one label, `agent:?` (`store.rs`, `const UNNAMEABLE`). Every
//! member whose id carries a space or an `@` maps onto it — `eventlog_core::validate_identity`
//! refuses both, while `StreamId` accepts both through `validate_field`, which is exactly why the
//! variant exists. So two such members of the same actor type record the same envelope, and the
//! only thing left distinguishing them is the caller-supplied payload the story names as the
//! reason it was filed ("`agent.yaml:434` makes `TakeAssignment`'s `agent_id` caller-supplied
//! input").
//!
//! These go through the real socket, exactly as the unit's own cases do.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use ess_runtime::Spec;
use swarm_server::http;
use swarm_server::state::Server;
use swarm_server::swarm::Swarm;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

async fn serving() -> (Arc<Swarm>, SocketAddr, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-adversary-issuer").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Arc::new(
        Server::start(spec, data.path().to_path_buf())
            .await
            .expect("the server starts"),
    );
    let swarm = server.open("agents").await.expect("the swarm opens");

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("a port");
    let at = listener.local_addr().expect("an address");
    let app = http::routes(server);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (swarm, at, data)
}

async fn post(at: SocketAddr, path: &str, body: Json) -> (u16, String) {
    let payload = body.to_string();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    );
    call(at, &request).await
}

async fn get(at: SocketAddr, path: &str) -> (u16, String) {
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    call(at, &request).await
}

async fn call(at: SocketAddr, request: &str) -> (u16, String) {
    let mut socket = tokio::net::TcpStream::connect(at).await.expect("connects");
    socket
        .write_all(request.as_bytes())
        .await
        .expect("the request is sent");
    let mut answer = String::new();
    socket
        .read_to_string(&mut answer)
        .await
        .expect("the response is read");

    let status = answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line in {answer:?}"));
    let body = answer
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

async fn log(at: SocketAddr) -> Vec<Json> {
    let (status, body) = get(at, "/swarms/agents/log").await;
    assert_eq!(status, 200, "the log is served: {body}");
    serde_json::from_str::<Json>(&body)
        .unwrap_or_else(|why| panic!("the log is JSON: {why}: {body}"))
        .as_array()
        .expect("an array of events")
        .clone()
}

fn named<'a>(recorded: &'a [Json], name: &str) -> Vec<&'a Json> {
    recorded
        .iter()
        .filter(|event| event["name"].as_str() == Some(name))
        .collect()
}

fn issuer(event: &Json) -> Option<&str> {
    event.get("issuer").and_then(Json::as_str)
}

/// A `swarm.agent.Note` at the command door, issued as `agent`.
fn note(agent: &str, about: &str, msg: &str) -> Json {
    json!({
        "input": {"agent_id": about, "msg": msg, "ref": null},
        "actor": "swarm.agent.Worker",
        "request": format!("note-{msg}"),
        "agent": agent,
    })
}

/// Two members of ONE actor type whose ids the envelope's identity column cannot hold.
///
/// `two words` carries a space and `who@example` carries an `@`;
/// `eventlog_core::validate_identity` refuses both and `StreamId` accepts both, which is the state
/// `Issuer::UnnameableAgent`'s own doc comment says is reachable and worth keeping working.
///
/// The story's acceptance clause 1, verbatim: an event records WHICH AGENT INSTANCE caused it,
/// distinguishably from another agent of the same actor type. These two are the same actor type
/// and the same envelope.
#[tokio::test]
async fn two_members_whose_names_the_log_cannot_hold_are_not_the_same_record() {
    let (swarm, at, _data) = serving().await;

    swarm
        .issue(
            None,
            "swarm.manager.CreateSwarm",
            args(json!({"display_name": "agents", "tmux_session": "agents",
                        "home": "swarms/agents", "created_at": "2026-09-13T10:00:00Z"})),
            "create",
        )
        .await
        .expect("the swarm record is made");
    let swarm_id = swarm.instances("swarm.manager.Swarm").await[0]["id"]
        .as_str()
        .expect("an identity")
        .to_owned();

    // Both members exist: the specification's `agent_id` is a caller-supplied `String`, and
    // `StreamId` takes a space and an `@` in one. `Issuer::UnnameableAgent` exists because of it.
    for agent in ["two words", "who@example"] {
        swarm
            .issue(
                Some("swarm.agent.Coordinator"),
                "swarm.agent.Spawn",
                args(
                    json!({"agent_id": agent, "swarm_id": swarm_id, "role": "Worker",
                            "harness": "ClaudeCode", "display_name": agent, "host": {}}),
                ),
                &format!("spawn-{agent}"),
            )
            .await
            .unwrap_or_else(|why| {
                panic!("a member called `{agent}` can be spawned, per `Issuer`'s own doc: {why}")
            });
    }

    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note("two words", "two words", "first-spoke"),
    )
    .await;
    assert_eq!(status, 200, "the first member's note is accepted: {body}");
    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note("who@example", "who@example", "second-spoke"),
    )
    .await;
    assert_eq!(status, 200, "the second member's note is accepted: {body}");

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 2, "two notes were recorded: {recorded:#?}");

    // One actor type, twice — that part is the specification's and is not in question.
    assert_eq!(notes[0]["actor"], notes[1]["actor"]);

    // Clause 1. The envelope has to say which of the two acted, and both say `agent:?`.
    assert_ne!(
        issuer(notes[0]),
        issuer(notes[1]),
        "two members of one actor type are two records, not the same record twice: \
         first={:?} second={:?}",
        issuer(notes[0]),
        issuer(notes[1])
    );
}

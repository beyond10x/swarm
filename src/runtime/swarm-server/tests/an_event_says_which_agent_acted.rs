//! `story:an-event-cannot-say-which-agent-acted`: the record says WHICH agent acted.
//!
//! Measured on `data/swarms/two-agents-proof/eventlog.sqlite3` before any of this existed: seq 13
//! is `swarm.agent.AssignmentTaken` with actor `swarm.agent.Worker`, and seq 20 is
//! `swarm.agent.AgentIdle` with the same actor string. `apply.rs`'s `permitted` matches the actor
//! against the specification's actor TYPES, so every member of a swarm records the same word, and
//! `store.rs` wrote that one word into both the `subject` and the `actor` column — two columns
//! holding one fact. A single-agent swarm could write a log a reader cannot tell from a two-agent
//! one, and an operator's `curl` was recorded exactly as the loop's own command.
//!
//! These go **through a real socket**, because the fact this file is about is put on the envelope
//! at the HTTP door: who issued a command is a host fact, like the clock, and nothing that calls
//! `Swarm::issue` in-process can see the door at all. The two doors that write events are
//! `POST /swarms/{slug}/commands/{command}` and `POST /swarms/{slug}/mail`, and both are here.
//!
//! What is NOT here, deliberately: any check that the claim is true. Naming the issuer and proving
//! the name are different problems and the second is worth nothing without the first — the story's
//! own Out of Scope. A `curl` may still say it is the coordinator; what it may no longer do is say
//! nothing and be recorded as the runtime.

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

/// A server over a temporary data directory, and the route table on a real port.
async fn serving() -> (Arc<Swarm>, SocketAddr, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-issuer").expect("a scratch directory");
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

/// One POST with a JSON body, and the response body it answered with.
///
/// `Connection: close` so the read ends at EOF rather than at a keep-alive that never closes.
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

/// The log as a reader reads it: `GET /swarms/{slug}/log`, oldest first.
async fn log(at: SocketAddr) -> Vec<Json> {
    let (status, body) = get(at, "/swarms/agents/log").await;
    assert_eq!(status, 200, "the log is served: {body}");
    serde_json::from_str::<Json>(&body)
        .unwrap_or_else(|why| panic!("the log is JSON: {why}: {body}"))
        .as_array()
        .expect("an array of events")
        .clone()
}

/// Every recorded event of this name, in order.
fn named<'a>(recorded: &'a [Json], name: &str) -> Vec<&'a Json> {
    recorded
        .iter()
        .filter(|event| event["name"].as_str() == Some(name))
        .collect()
}

/// What the record says about who issued the command that produced this event.
///
/// A row written before this story closed carries no such fact at all, and the absent key is what
/// says so — a reader that wants to cope with an old log asks for this and gets `None`.
fn issuer(event: &Json) -> Option<&str> {
    event.get("issuer").and_then(Json::as_str)
}

/// The setup every case shares: a swarm record, and two members of the SAME actor type.
///
/// Both are `Worker`s, which is the whole point: `swarm.agent.Worker` is what the specification
/// permits `Note` to, so both members record that same word in the `actor` column and the record
/// has to say which of them acted by some other means.
async fn two_workers(swarm: &Arc<Swarm>) -> String {
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

    for agent in ["alpha", "beta"] {
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
            .expect("the member is spawned");
    }
    swarm_id
}

/// A `swarm.agent.Note` at the command door, issued as `agent` when one is named.
fn note(agent: Option<&str>, about: &str, msg: &str) -> Json {
    let mut body = json!({
        "input": {"agent_id": about, "msg": msg, "ref": null},
        "actor": "swarm.agent.Worker",
        "request": format!("note-{msg}"),
    });
    if let Some(agent) = agent {
        body["agent"] = json!(agent);
    }
    body
}

/// Clause 1: two agents of one actor type are two records, not the same record twice.
#[tokio::test]
async fn two_members_of_one_actor_type_are_two_records() {
    let (swarm, at, _data) = serving().await;
    two_workers(&swarm).await;

    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(Some("alpha"), "alpha", "alpha-spoke"),
    )
    .await;
    assert_eq!(status, 200, "alpha's note is accepted: {body}");
    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(Some("beta"), "beta", "beta-spoke"),
    )
    .await;
    assert_eq!(status, 200, "beta's note is accepted: {body}");

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 2, "two notes were recorded: {recorded:#?}");

    // The actor column still carries the specification's actor TYPE, and it is the SAME for both:
    // that is the fact the story measured, and nothing here widens `permitted`.
    assert_eq!(
        notes[0]["actor"], notes[1]["actor"],
        "one actor type, twice"
    );
    assert_eq!(notes[0]["actor"], json!("swarm.agent.Worker"));

    // And beside it, the thing the record could not say before.
    assert_eq!(issuer(notes[0]), Some("agent:alpha"), "{:#?}", notes[0]);
    assert_eq!(issuer(notes[1]), Some("agent:beta"), "{:#?}", notes[1]);
    assert_ne!(
        issuer(notes[0]),
        issuer(notes[1]),
        "two members of one actor type are two records"
    );
}

/// Clause 2: a command issued about an agent by something that is not that agent.
///
/// Not refused — the coordinator legitimately issues `Spawn` and `Assign` for members that are not
/// itself, and refusing here would be authorisation, which this story excludes. Recorded so a
/// reader can tell it apart from the member's own action, which is the whole of the ask.
#[tokio::test]
async fn a_command_issued_about_another_member_is_not_that_members_own_action() {
    let (swarm, at, _data) = serving().await;
    two_workers(&swarm).await;

    // beta issues a note ABOUT alpha. The payload names alpha; the envelope names beta.
    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(Some("beta"), "alpha", "beta-spoke-for-alpha"),
    )
    .await;
    assert_eq!(status, 200, "it is recorded rather than refused: {body}");

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 1, "one note: {recorded:#?}");
    assert_eq!(
        notes[0]["fields"]["agent_id"],
        json!("alpha"),
        "the note is about alpha"
    );
    assert_eq!(
        issuer(notes[0]),
        Some("agent:beta"),
        "and beta is what issued it: {:#?}",
        notes[0]
    );
    assert_ne!(
        issuer(notes[0]),
        Some("agent:alpha"),
        "a note about alpha that alpha did not write must not read as alpha's own"
    );
}

/// Clause 3: an operator at the door, and the runtime issuing to itself, are two different things.
///
/// Before this, both were the word `system`, which is why "no operator touched this" could not be
/// read off a log at all — `check-two-agents.py` reported the condition UNDETERMINABLE and said so.
#[tokio::test]
async fn an_operator_at_the_door_is_not_the_runtime_issuing_to_itself() {
    let (swarm, at, _data) = serving().await;
    two_workers(&swarm).await;

    // A curl with nothing to say about who it is. It is not refused, and it is not the runtime.
    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(None, "alpha", "by-hand"),
    )
    .await;
    assert_eq!(status, 200, "an unnamed caller is recorded: {body}");

    // And the runtime's own command, in-process, through no door at all.
    swarm
        .issue(
            Some("swarm.agent.Worker"),
            "swarm.agent.Note",
            args(json!({"agent_id": "alpha", "msg": "by-the-loop", "ref": null})),
            "note-by-the-loop",
        )
        .await
        .expect("the runtime issues its own");

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 2, "two notes: {recorded:#?}");
    assert_eq!(issuer(notes[0]), Some("operator"), "{:#?}", notes[0]);
    assert_eq!(issuer(notes[1]), Some("runtime"), "{:#?}", notes[1]);

    // The spawns of the setup went through no door either: the runtime issued them.
    let spawned = named(&recorded, "swarm.agent.AgentSpawned");
    assert_eq!(spawned.len(), 2);
    for event in spawned {
        assert_eq!(issuer(event), Some("runtime"), "{event:#?}");
    }
}

/// A claimed name the log's identity column cannot hold is not silently recorded as somebody else.
///
/// `swarm.agent.Spawn`'s `agent_id` is a caller-supplied `String` and `StreamId` accepts a space
/// in one, but the envelope's identity columns do not (`eventlog_core::validate_identity`: no
/// space, no `@`, because an append-only log outlives every request to erase an address). So a
/// member called `two words` exists and its claim cannot be written verbatim. What must NOT happen
/// is the two failures either side of that: refusing the command, which would take a working
/// member's every command away, or writing `operator` or `runtime`, which would put a falsehood in
/// the record.
#[tokio::test]
async fn a_name_the_log_cannot_hold_is_neither_refused_nor_recorded_as_somebody_else() {
    let (swarm, at, _data) = serving().await;
    two_workers(&swarm).await;

    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(Some("two words"), "alpha", "unnameable"),
    )
    .await;
    assert_eq!(status, 200, "the command is not refused: {body}");

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 1, "one note: {recorded:#?}");
    let said = issuer(notes[0]).unwrap_or_else(|| panic!("an issuer: {:#?}", notes[0]));
    assert!(
        said.starts_with("agent:"),
        "an agent claimed it, and the record says an agent claimed it: {said}"
    );
    assert_ne!(said, "operator", "it was not an operator");
    assert_ne!(said, "runtime", "it was not the runtime");
}

/// The other half of that, and the half the first pass got wrong: the mark is PER NAME, and it is
/// the same mark every time for one name.
///
/// The first pass gave every unnameable member one label, `agent:?`, which made two such members
/// the same record twice — acceptance clause 1 verbatim, negated, and the adversary of correction
/// round 1 negated it (`tests/adversary_issuer_pass_1.rs`). The fix is a digest of the claim, and a
/// digest buys nothing unless it is STABLE: two commands by one member that recorded two different
/// marks would read as two members, which is the same defect wearing the opposite sign.
///
/// So the digest is written out (FNV-1a, in `store.rs`) rather than taken from `DefaultHasher`,
/// whose algorithm std does not specify and may change between releases. A log outlives a
/// toolchain upgrade; a member's mark has to outlive one too.
#[tokio::test]
async fn one_member_the_log_cannot_name_keeps_one_mark_across_its_commands() {
    let (swarm, at, _data) = serving().await;
    two_workers(&swarm).await;

    for msg in ["said-once", "said-twice"] {
        let (status, body) = post(
            at,
            "/swarms/agents/commands/swarm.agent.Note",
            note(Some("two words"), "alpha", msg),
        )
        .await;
        assert_eq!(status, 200, "{msg} is accepted: {body}");
    }
    // And a second member whose name the log also cannot hold.
    let (status, body) = post(
        at,
        "/swarms/agents/commands/swarm.agent.Note",
        note(Some("who@example"), "alpha", "someone-else"),
    )
    .await;
    assert_eq!(
        status, 200,
        "the second unnameable member is accepted: {body}"
    );

    let recorded = log(at).await;
    let notes = named(&recorded, "swarm.agent.NoteEmitted");
    assert_eq!(notes.len(), 3, "three notes: {recorded:#?}");
    assert_eq!(
        issuer(notes[0]),
        issuer(notes[1]),
        "one member, one mark, twice"
    );
    assert_ne!(
        issuer(notes[0]),
        issuer(notes[2]),
        "two members, two marks: {:?} and {:?}",
        issuer(notes[0]),
        issuer(notes[2])
    );
    // Still marked as unreadable rather than passed off as a name the log could hold: a reader
    // that compares the mark against `agent_id` must not conclude they match.
    for event in &notes {
        let said = issuer(event).expect("an issuer");
        assert!(said.starts_with("agent:?"), "{said}");
    }

    // And the VALUE, pinned. Everything above holds of any deterministic injective function, which
    // `DefaultHasher` also is — and not using `DefaultHasher` is the whole reason `mark` is
    // written out, because std does not specify its algorithm and a mark that moved under a
    // toolchain upgrade would split one member into two. Nothing above would have noticed.
    //
    // Derived rather than copied from the run: FNV-1a 64-bit over `two words`, taken to 12 hex
    // characters. The same arithmetic reproduces the algorithm's canonical vectors —
    // `""` → cbf29ce484222325, `"a"` → af63dc4c8601ec8c, `"foobar"` → 85944171f73967e8 — so what
    // is pinned here is FNV-1a and not merely what this build happened to print.
    assert_eq!(
        issuer(notes[0]),
        Some("agent:?ccb500272beb"),
        "the mark is FNV-1a of the claim, and this is the value it has"
    );
}

/// The other door that writes events: mail.
///
/// `POST /swarms/{slug}/mail` reaches `PostMessage` through `Swarm::post`, and an operator can
/// reach it exactly as an agent can. The body already carries a `sender`, and a sender is a
/// CLAIM IN THE PAYLOAD — the same kind of thing as `TakeAssignment`'s `agent_id`. Who issued it
/// is the envelope's, and the two are allowed to disagree.
#[tokio::test]
async fn the_mail_door_says_which_agent_posted() {
    let (swarm, at, _data) = serving().await;
    let swarm_id = two_workers(&swarm).await;

    for agent in ["alpha", "beta"] {
        swarm
            .issue(
                Some("swarm.mailbox.SwarmAgent"),
                "swarm.mailbox.OpenMailbox",
                args(
                    json!({"swarm_id": swarm_id, "agent_id": agent, "name": "main",
                            "created_at": "2026-09-13T10:01:00Z"}),
                ),
                &format!("mailbox-{agent}"),
            )
            .await
            .expect("the mailbox opens");
    }

    let (status, body) = post(
        at,
        "/swarms/agents/mail",
        json!({"to": "beta", "sender": "alpha", "subject": "one", "body": "hello",
               "agent": "alpha"}),
    )
    .await;
    assert_eq!(status, 200, "alpha posts: {body}");

    // And an operator posting the same mail, claiming in the payload to be alpha.
    let (status, body) = post(
        at,
        "/swarms/agents/mail",
        json!({"to": "beta", "sender": "alpha", "subject": "two", "body": "hello"}),
    )
    .await;
    assert_eq!(status, 200, "an operator posts: {body}");

    let recorded = log(at).await;
    let posted = named(&recorded, "swarm.mailbox.MessagePosted");
    assert_eq!(posted.len(), 2, "two messages: {recorded:#?}");
    assert_eq!(issuer(posted[0]), Some("agent:alpha"), "{:#?}", posted[0]);
    assert_eq!(
        issuer(posted[1]),
        Some("operator"),
        "a hand at the mail door is a hand: {:#?}",
        posted[1]
    );
    // Both claim alpha in the payload. That claim is not what distinguishes them.
    for event in &posted {
        assert_eq!(event["fields"]["sender_id"], json!("alpha"));
    }
}

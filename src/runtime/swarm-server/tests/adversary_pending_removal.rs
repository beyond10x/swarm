//! Pass 2 against `DELETE /swarms/{slug}`: the deferred unlink, and what a SECOND removal does to
//! a slug the first one could only park.
//!
//! Written before anything was run. The claim under attack is the one `state.rs` makes for itself:
//!
//! > Unlinking under a live handle is not a filesystem problem on this platform — it succeeds — it
//! > is a HONESTY problem: the holder's writes keep landing in the unlinked inode and keep
//! > answering `created`, so a client is told a command was applied that no reader of that slug can
//! > ever see. (`state.rs`, the `pending` field)
//!
//! and the one `http.rs` makes on the wire:
//!
//! > `204` when the place is gone. `202` when the slug is no longer served but something else was
//! > still holding its handle, so the files go when that holder lets go — saying `204` there would
//! > be a claim about the disk that is not true yet. (`http.rs:148-150`)
//!
//! `Server::remove` reads `pending` only to decide that the slug is not `NotHere`
//! (`state.rs:311-316`). Everything after that branches on `held`, which is the `swarms` map — and
//! a parked slug is not in the `swarms` map. So the second removal of a parked slug takes the
//! `held.is_none()` path, skips the `live_state` refusal, and falls through to `unlink`: it erases
//! the files out from under a handle the `pending` map is holding precisely because somebody is
//! still writing through it, and answers `Removed::Now` / `204` for a disk state it just made
//! false for that writer.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value as Json, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use ess_runtime::Spec;
use swarm_server::http;
use swarm_server::state::{Removal, Removed, Server};
use swarm_server::swarm::Swarm;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-adversary-pending").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

async fn serving(server: Arc<Server>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("a port");
    let at = listener.local_addr().expect("an address");
    let app = http::routes(server);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    at
}

async fn call(at: SocketAddr, method: &str, path: &str) -> u16 {
    let mut socket = tokio::net::TcpStream::connect(at).await.expect("connects");
    let request =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    socket
        .write_all(request.as_bytes())
        .await
        .expect("the request is sent");
    let mut answer = String::new();
    socket
        .read_to_string(&mut answer)
        .await
        .expect("the response is read");
    answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line in {answer:?}"))
}

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Makes one `swarm.manager.Swarm` record through the given handle.
async fn create_record(swarm: &Arc<Swarm>, slug: &str, session: &str, request: &str) -> String {
    swarm
        .issue(
            None,
            "swarm.manager.CreateSwarm",
            args(json!({"display_name": slug, "tmux_session": session,
                        "home": format!("swarms/{slug}"),
                        "created_at": "2026-09-12T10:00:00Z"})),
            request,
        )
        .await
        .unwrap_or_else(|why| panic!("CreateSwarm: {why}"))
        .outcome
}

fn swarm_dir(data: &tempdir::TempDir, slug: &str) -> PathBuf {
    data.path().join("swarms").join(slug)
}

/// A removal that could only park the handle is retried, and the retry unlinks under the holder.
///
/// `202 Accepted` is an invitation to retry — that is what the code means. The first removal parks
/// `parked`'s handle in `pending` because a command handler (here, `held`) is still holding it. The
/// second removal finds nothing in the `swarms` map, so `held` is `None`, so the `outstanding`
/// branch that parks is never reached, and `unlink` runs while the `pending` map is still holding a
/// live `Arc` over that log.
///
/// The assertion is the one thing the whole `pending` mechanism exists to make true: while a holder
/// is alive, that swarm's files are on disk.
#[tokio::test]
async fn a_retried_removal_does_not_unlink_under_a_holder_the_first_one_parked() {
    let (server, data) = server().await;
    server.open("parked").await.expect("a place is made");

    // What an in-flight command handler — or one tick of the trigger, which holds every handle
    // `Server::all` gave it for the whole turn — is holding right now.
    let held = server
        .get("parked")
        .await
        .expect("the handle is handed out");

    let first = server
        .remove("parked")
        .await
        .expect("the first removal runs");
    assert_eq!(
        first,
        Removed::WhenReadersLetGo,
        "with a holder alive the removal has to be the deferred one"
    );
    assert!(
        swarm_dir(&data, "parked").exists(),
        "a parked removal leaves the files alone until the holder lets go"
    );

    // The client retries, exactly as `202` invites it to.
    let second = server.remove("parked").await;

    assert!(
        swarm_dir(&data, "parked").exists(),
        "the second removal answered `{second:?}` and unlinked `{}` while a handle over that log \
         is still alive and still answering `created` to whoever holds it — the deferred unlink \
         `pending` exists for is skipped because `remove` branches on the `swarms` map, which a \
         parked slug is not in",
        swarm_dir(&data, "parked").display(),
    );

    drop(held);
}

/// The second removal never asks the model, because it never has a handle to ask.
///
/// `remove` runs `live_state` only `if let Some(swarm) = &held` (`state.rs:319-326`), and `held` is
/// the `swarms` map. A slug whose handle is parked in `pending` is not in that map, so a live
/// `swarm.manager.Swarm` record written through the parked handle cannot refuse the removal. The
/// unit's own doc for `remove` says the opposite: *"ANY live record refuses it, not 'the' state."*
#[tokio::test]
async fn a_parked_slug_holding_a_live_record_still_refuses_removal() {
    let (server, data) = server().await;
    server.open("revived").await.expect("a place is made");
    let held = server
        .get("revived")
        .await
        .expect("the handle is handed out");

    assert_eq!(
        server.remove("revived").await.expect("the removal runs"),
        Removed::WhenReadersLetGo,
        "with a holder alive the removal has to be the deferred one"
    );

    // The holder is still writing, which is the entire reason the files were left alone.
    create_record(&held, "revived", "revived-session", "adv2-revive").await;
    let live = held.instances("swarm.manager.Swarm").await;
    assert!(
        !live.is_empty(),
        "the parked handle still writes to its log; that is why the files are still there"
    );

    let second = server.remove("revived").await;

    match second {
        Err(Removal::StateConflict { .. }) => {}
        other => panic!(
            "a slug holding a live `swarm.manager.Swarm` record answered `{other:?}` instead of \
             SwarmStateConflict, because the record was reachable only through the handle parked \
             in `pending` and `remove` asks only the `swarms` map; the directory is now {}",
            if swarm_dir(&data, "revived").exists() {
                "still there"
            } else {
                "erased"
            },
        ),
    }

    drop(held);
}

/// The same thing on the wire, which is where a client meets it.
///
/// `DELETE` twice. The first answers `202` — "the files go when that holder lets go". The second
/// answers `204`, which `http.rs:148-150` defines as *"the place is gone"*, while the holder is
/// alive and the promise `202` made has been broken rather than kept.
#[tokio::test]
async fn a_retried_delete_does_not_answer_204_while_the_holder_is_alive() {
    let (server, data) = server().await;
    let at = serving(Arc::clone(&server)).await;
    server.open("wired").await.expect("a place is made");
    let held = server.get("wired").await.expect("the handle is handed out");

    let first = call(at, "DELETE", "/swarms/wired").await;
    assert_eq!(first, 202, "a holder is alive, so the removal is deferred");

    let second = call(at, "DELETE", "/swarms/wired").await;
    assert!(
        swarm_dir(&data, "wired").exists(),
        "the retried DELETE answered {second} and the directory is gone while the handle that was \
         the reason for the first answer's `202` is still alive"
    );

    drop(held);
}

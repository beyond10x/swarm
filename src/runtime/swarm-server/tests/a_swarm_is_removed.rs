//! Removing a swarm: the verb the server never had, over the wire it will be called on.
//!
//! These go through a real socket rather than calling the handler, because the defect is a MISSING
//! VERB in the route table (`http.rs`) and nothing that calls a function can see one. A router with
//! `get` and `post` on `/swarms/{slug}` answers `DELETE` with `405 Method Not Allowed`, and that is
//! the red this file starts from.
//!
//! What removal must be true of, all three documented in `state.rs` before any of it was written:
//!
//! 1. the per-slug `opening` gate is taken, so no concurrent open reconstructs the handle mid-removal;
//! 2. the handle leaves the map and the `Arc` is dropped — closing the SQLite handle — BEFORE the
//!    directory goes, since nothing else ever evicts from `swarms`;
//! 3. `eventlog.sqlite3`, `-wal` and `-shm` go together, or the next open runs WAL recovery against
//!    a fresh empty database.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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

/// A server over a temporary data directory that is cleaned up with the test.
async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-removal").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

/// The route table, on a real port. Returns where to reach it.
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

/// One request, one response: the status and the body.
///
/// `Connection: close` so the read ends at EOF rather than at a keep-alive that never closes.
async fn call(at: SocketAddr, method: &str, path: &str) -> (u16, String) {
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

fn args(value: Json) -> serde_json::Map<String, Json> {
    value.as_object().expect("an object").clone()
}

/// Issues a command the way the HTTP layer would.
async fn issue(swarm: &Arc<Swarm>, command: &str, input: Json) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let request = format!("req-{}", NEXT.fetch_add(1, Ordering::Relaxed));

    swarm
        .issue(None, command, args(input), &request)
        .await
        .unwrap_or_else(|why| panic!("{command}: {why}"))
        .outcome
}

/// Makes the `swarm.manager.Swarm` record, and answers with its id.
async fn create_record(swarm: &Arc<Swarm>, slug: &str) -> String {
    issue(
        swarm,
        "swarm.manager.CreateSwarm",
        json!({"display_name": slug, "tmux_session": slug, "home": format!("swarms/{slug}"),
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    let made = swarm.instances("swarm.manager.Swarm").await;
    assert_eq!(made.len(), 1, "one record was created");
    made[0]["id"].as_str().expect("an identity").to_owned()
}

fn swarm_dir(data: &tempdir::TempDir, slug: &str) -> PathBuf {
    data.path().join("swarms").join(slug)
}

/// The log and everything SQLite keeps beside it.
fn log_files(dir: &Path) -> Vec<PathBuf> {
    vec![
        dir.join("eventlog.sqlite3"),
        dir.join("eventlog.sqlite3-wal"),
        dir.join("eventlog.sqlite3-shm"),
    ]
}

/// A place with no swarm in it — `POST /swarms` and no `CreateSwarm` — is exactly what the operator
/// could not remove. There is no record, so no state can refuse it.
#[tokio::test]
async fn a_place_with_no_swarm_in_it_is_removed() {
    let (server, data) = server().await;
    server.open("empty").await.expect("a place is made");
    let at = serving(Arc::clone(&server)).await;
    assert!(swarm_dir(&data, "empty").is_dir());

    let (status, body) = call(at, "DELETE", "/swarms/empty").await;
    assert_eq!(status, 204, "DELETE /swarms/empty answered {body}");

    assert!(
        !swarm_dir(&data, "empty").exists(),
        "the directory is still on disk",
    );
    assert!(
        !server.slugs().await.contains(&"empty".to_owned()),
        "the handle is still in the map",
    );
    assert!(
        server.get("empty").await.is_err(),
        "the handle is still handed out after its log was removed",
    );
}

/// The whole log goes, WAL and shm with it — and what proves it is that re-opening the slug finds
/// an empty world rather than recovering the one that was removed.
#[tokio::test]
async fn a_deleted_swarm_takes_its_log_and_its_wal_with_it() {
    let (server, data) = server().await;
    let swarm = server.open("retired").await.expect("a place is made");
    let id = create_record(&swarm, "retired").await;
    let deleted = issue(&swarm, "swarm.manager.DeleteSwarm", json!({"swarm_id": id})).await;
    assert_eq!(deleted, "deleted");
    drop(swarm);

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "DELETE", "/swarms/retired").await;
    assert_eq!(status, 204, "DELETE /swarms/retired answered {body}");

    for file in log_files(&swarm_dir(&data, "retired")) {
        assert!(!file.exists(), "{} survived the removal", file.display());
    }

    // The decisive one: a fresh open of the same slug is a fresh swarm. A `-wal` left behind, or a
    // handle still holding the database open when the file went, both show up here.
    let again = server.open("retired").await.expect("the slug opens again");
    assert!(
        again.instances("swarm.manager.Swarm").await.is_empty(),
        "re-opening the slug recovered the log that was supposed to be gone",
    );
}

/// Anything that is not absent-or-terminal is refused, and the refusal is the error the
/// specification already declares for exactly this.
#[tokio::test]
async fn a_running_swarm_is_refused_with_the_specification_s_own_error() {
    let (server, data) = server().await;
    let swarm = server.open("busy").await.expect("a place is made");
    let id = create_record(&swarm, "busy").await;
    let started = issue(
        &swarm,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": id, "started_at": "2026-09-12T10:01:00Z"}),
    )
    .await;
    assert_eq!(started, "started");
    drop(swarm);

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "DELETE", "/swarms/busy").await;
    assert_eq!(status, 409, "a Running swarm answered {status}: {body}");
    assert!(
        body.contains("swarm.manager.SwarmStateConflict"),
        "the refusal does not name the declared error: {body}",
    );
    assert!(
        body.contains("Running"),
        "the refusal does not say which state refused it: {body}",
    );

    assert!(
        swarm_dir(&data, "busy").is_dir(),
        "the directory was removed"
    );
    assert!(
        server.slugs().await.contains(&"busy".to_owned()),
        "the handle was evicted by a refused removal",
    );
}

/// A slug the server does not hold is a `404`, not a `204` and not a `500`.
#[tokio::test]
async fn a_slug_the_server_does_not_hold_is_not_found() {
    let (server, _data) = server().await;
    let at = serving(server).await;

    let (status, body) = call(at, "DELETE", "/swarms/never-existed").await;
    assert_eq!(status, 404, "an unknown slug answered {status}: {body}");
}

/// `manager.yaml:326` — "it no longer appears in the swarms list". Six swarms in `Deleted` still
/// appeared, because the list is the keys of the handle map.
#[tokio::test]
async fn a_swarm_in_a_terminal_state_is_not_in_the_list() {
    let (server, _data) = server().await;
    let gone = server.open("gone").await.expect("a place is made");
    let kept = server.open("kept").await.expect("a place is made");
    let id = create_record(&gone, "gone").await;
    create_record(&kept, "kept").await;
    issue(&gone, "swarm.manager.DeleteSwarm", json!({"swarm_id": id})).await;

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "GET", "/swarms").await;
    assert_eq!(status, 200);
    let listed: Vec<String> = serde_json::from_str(&body).expect("a list of slugs");
    assert!(
        listed.contains(&"kept".to_owned()),
        "a live swarm fell out of the list: {listed:?}",
    );
    assert!(
        !listed.contains(&"gone".to_owned()),
        "a Deleted swarm still appears in the list: {listed:?}",
    );
}

/// Hazard 1, measured: a removal and an open of the same slug racing each other never leave a
/// handle in the map for a directory that is not on disk.
///
/// The slug's place is made on disk WITHOUT opening it, so both tasks have real work to do and the
/// window is `Swarm::open`'s whole store-open-and-replay rather than a few syscalls. Without the
/// per-slug gate the removal unlinks the directory while the open is constructing over it, and the
/// open's insert lands afterwards: a live handle in the map for a swarm that is gone. With the gate
/// the two are serialised either way round, and both orders leave the map agreeing with the disk.
///
/// Measured against a copy of `state.rs` with the gate taken out: red in round 0, three runs of
/// three, on `assert!(opened.is_ok())`. Two earlier versions of this case stayed GREEN against that
/// same mutant — one pre-opened the slug, leaving almost no window; the other started the removal
/// first, so it was over before the open reached the store — and both are the reason this one
/// asserts what it asserts in the order it does.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_removal_racing_an_open_leaves_the_map_agreeing_with_the_disk() {
    let (server, data) = server().await;

    for round in 0..20 {
        let slug = format!("raced-{round}");
        std::fs::create_dir_all(swarm_dir(&data, &slug)).expect("the place is made");

        // The open is started FIRST and the removal lands while it is inside the store open and
        // the replay. Started the other way round the removal is over in microseconds and the open
        // simply remakes the place afterwards, which is consistent whether or not anything is
        // serialised — the first version of this case did that and could not tell the two apart.
        let opening = {
            let server = Arc::clone(&server);
            let slug = slug.clone();
            tokio::spawn(async move { server.open(&slug).await })
        };
        let removing = {
            let server = Arc::clone(&server);
            let slug = slug.clone();
            tokio::spawn(async move { server.remove(&slug).await })
        };
        let opened = opening.await.expect("the open task finishes");
        let _ = removing.await.expect("the removal task finishes");

        // An open of a slug is never broken by a removal of that slug running beside it. Ungated,
        // the removal unlinks the database out from under the store that is opening it and the
        // caller gets a `500` for a swarm nobody said anything was wrong with.
        assert!(
            opened.is_ok(),
            "round {round}: an open racing a removal failed: {:?}",
            opened.err(),
        );

        let held = server.slugs().await.contains(&slug);
        let on_disk = swarm_dir(&data, &slug).is_dir();
        assert_eq!(
            held, on_disk,
            "round {round}: the map holds it: {held}; the disk has it: {on_disk}",
        );
    }
}

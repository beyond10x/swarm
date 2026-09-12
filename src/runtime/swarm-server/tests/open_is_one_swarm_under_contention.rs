//! `Server::open` against the contract this wave's change wrote for it.
//!
//! `src/state.rs:75-80` now says, of `Server::open`: "Every call for one slug returns the same
//! handle, however many run at once." These cases read that sentence as the specification it
//! claims to be and drive the implementation against it. They do not assert the implementation's
//! behaviour; they assert the document's.

use std::path::PathBuf;
use std::sync::Arc;

use ess_runtime::Spec;
use swarm_server::state::Server;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

/// A server over a temporary data directory that is cleaned up with the test.
async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-adversary").expect("a scratch directory");
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    let server = Server::start(spec, data.path().to_path_buf())
        .await
        .expect("the server starts");
    (Arc::new(server), data)
}

/// No concurrent open of one slug is refused.
///
/// The doc on `Server::open` promises every call for one slug the same handle, "however many run at
/// once". A `Result::Err` is not that handle. `Swarm::open` is awaited outside every guard, so N
/// simultaneous callers all run it against one SQLite file, the store sets no `busy_timeout`
/// (`ess-runtime/src/store.rs:22-24`), and a loser gets `SQLITE_BUSY` back — which `Server::open`
/// returns to its caller with no look at the map, even though the winner's handle is sitting in it.
///
/// Reached from outside this crate by `POST /swarms` (`http.rs:99-104`): two clients creating one
/// slug at the same moment, one gets `201`, the other `500 {"kind":"store"}` for a swarm that is
/// open and healthy.
///
/// `tests/serves_a_swarm.rs:391-402` waits this refusal out in a retry loop rather than asserting
/// on it. This case is that loop's `else` branch, promoted to the assertion the doc asks for.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_concurrent_open_of_one_slug_is_refused() {
    let (server, _data) = server().await;

    let mut refusals: Vec<String> = Vec::new();
    let mut attempts = 0_usize;

    // Fresh slug per round: once a slug is in the map every later open takes the read fast path and
    // never reaches `Swarm::open` at all, so one round measures the contended open exactly once.
    for round in 0..40 {
        let slug = format!("contested-{round}");
        let openers: Vec<_> = (0..8)
            .map(|_| {
                let server = Arc::clone(&server);
                let slug = slug.clone();
                tokio::spawn(async move { server.open(&slug).await.map_err(|why| why.to_string()) })
            })
            .collect();

        for opener in openers {
            attempts += 1;
            match opener.await.expect("the opener does not panic") {
                Ok(_) => {}
                Err(why) => refusals.push(format!("{slug}: {why}")),
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} of {attempts} concurrent opens were refused, but `Server::open`'s doc promises every \
         call for one slug the same handle however many run at once: {refusals:#?}",
        refusals.len()
    );
}

/// A swarm that genuinely cannot be opened is still refused.
///
/// The answer to the case above is that a refused open consults the map before it gives up. That
/// must not become "a refused open never refuses": when there is no handle to hand back, the error
/// is the answer, and swallowing it would turn a broken store into a silent success.
#[tokio::test]
async fn an_open_with_no_handle_to_fall_back_on_is_still_refused() {
    let (server, data) = server().await;

    // An event log that cannot be a file, so the store cannot open it, and nothing is in the map
    // for this slug to fall back on.
    let broken = data.path().join("swarms").join("broken");
    std::fs::create_dir_all(broken.join("eventlog.sqlite3")).expect("the obstruction is placed");

    let why = match server.open("broken").await {
        Ok(swarm) => panic!(
            "a store that cannot open was not refused: Ok({})",
            swarm.slug()
        ),
        Err(why) => why.to_string(),
    };
    assert!(
        server.get("broken").await.is_err(),
        "a refused open leaves nothing in the map: {why}"
    );
}

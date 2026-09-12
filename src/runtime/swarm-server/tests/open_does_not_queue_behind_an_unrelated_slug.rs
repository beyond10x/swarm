//! `Server::open` against the contract this wave's correction wrote for it.
//!
//! `src/state.rs:25-26` now says of the new `opening` mutex: "Held across the construction of a
//! swarm that is not yet in `swarms`, so at most one `Swarm::open` runs at a time in this process."
//! `src/state.rs:86-87` promises the caller: "a caller that arrives while another is still
//! constructing waits for it rather than being refused."
//!
//! The second sentence is written about one slug. The lock is not: it is one mutex for every slug,
//! held across the whole of `Swarm::open` — which is `Store::open` plus a full replay of the event
//! log (`ess-runtime/src/store.rs:273-299`). So a caller opening slug B waits for slug A's replay,
//! and A and B have nothing to do with each other. This case asserts the thing the document does
//! not claim and the change quietly gave up: an open of one slug does not wait for an open of a
//! different slug.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

use ess_runtime::Spec;
use swarm_server::state::Server;

/// How many commands go into the slow swarm's log. Replay is linear in this
/// (`ess-runtime/src/store.rs:273-299`), and it is the only knob that separates "slow open" from
/// "fast open" without touching an implementation file.
const HEAVY_COMMANDS: usize = 4_000;

fn kernel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/core")
        .canonicalize()
        .expect("the kernel specification is beside this crate")
}

async fn server_over(root: &std::path::Path) -> Arc<Server> {
    let spec = Spec::load(kernel()).expect("the kernel resolves");
    Arc::new(
        Server::start(spec, root.to_path_buf())
            .await
            .expect("the server starts"),
    )
}

/// An open of one slug must not wait for an unrelated slug's open.
///
/// `heavy` has a log worth replaying; `light` is a slug nothing has ever opened, so its own open is
/// a directory creation and an empty replay. They share no file, no directory and no handle. With
/// construction serialised on one process-wide `opening` mutex (`src/state.rs:113`), `light`'s open
/// cannot begin until `heavy`'s replay has finished.
///
/// Reached from outside this crate by `POST /swarms` (`http.rs:102`), which is the only caller of
/// `Server::open`: two clients creating two different swarms at the same moment are now served one
/// after the other rather than together.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_open_does_not_wait_for_an_unrelated_slugs_open() {
    let data = tempdir::TempDir::new("swarm-adversary-queue").expect("a scratch directory");
    let root = data.path().to_path_buf();

    // The server under test, started before anything is on disk, so its map is empty and both opens
    // below are real opens rather than map hits.
    let server = server_over(&root).await;

    // A second, throwaway server builds `heavy`'s log and then gives up its handle, so the log is on
    // disk and `server` has never seen it.
    {
        let builder = server_over(&root).await;
        let heavy = builder.open("heavy").await.expect("`heavy` opens");
        for nth in 0..HEAVY_COMMANDS {
            heavy
                .issue(
                    None,
                    "swarm.manager.CreateSwarm",
                    json!({"display_name": format!("Heavy {nth}"),
                           "tmux_session": format!("heavy-{nth}"),
                           "home": format!("/tmp/heavy-{nth}"),
                           "created_at": "2026-09-12T10:00:00Z"})
                    .as_object()
                    .expect("an object")
                    .clone(),
                    &format!("heavy-{nth}"),
                )
                .await
                .expect("the command applies");
        }
        drop(heavy);
        drop(builder);
    }

    let origin = Instant::now();

    let slow = {
        let server = Arc::clone(&server);
        tokio::spawn(async move {
            server.open("heavy").await.expect("`heavy` re-opens");
            origin.elapsed()
        })
    };

    // Long enough that the slow open is inside `Swarm::open` holding `opening`, and short enough
    // that it is nowhere near done: the assertion below refuses to conclude anything if it is.
    tokio::time::sleep(Duration::from_millis(75)).await;
    let light_started = origin.elapsed();

    server.open("light").await.expect("`light` opens");
    let light_done = origin.elapsed();

    let heavy_done = slow.await.expect("the slow opener does not panic");

    assert!(
        heavy_done > light_started,
        "inconclusive, not a result: `heavy` finished at {heavy_done:?}, before `light` was even \
         asked for at {light_started:?}. Raise HEAVY_COMMANDS until the slow open outlasts the \
         75ms head start."
    );

    assert!(
        light_done < heavy_done,
        "`light` opened at {light_done:?}, after `heavy` finished at {heavy_done:?}: the open of a \
         slug with no log, no directory and no handle in common with `heavy` waited for `heavy`'s \
         replay. One process-wide `opening` mutex serialises every slug's construction, not just \
         the contended one, so two clients creating two different swarms at once (`http.rs:102`) \
         are served one after the other."
    );
}

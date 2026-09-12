//! Adversarial cases against `DELETE /swarms/{slug}` — the destructive verb added 2026-09-12.
//!
//! Written before anything was run. Each one drives the implementation against a claim the unit
//! itself makes, over the wire the route is called on.

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

async fn server() -> (Arc<Server>, tempdir::TempDir) {
    let data = tempdir::TempDir::new("swarm-adversary").expect("a scratch directory");
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

async fn issue(swarm: &Arc<Swarm>, command: &str, input: Json) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let request = format!("adv-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    swarm
        .issue(None, command, args(input), &request)
        .await
        .unwrap_or_else(|why| panic!("{command}: {why}"))
        .outcome
}

/// Makes one `swarm.manager.Swarm` record with the given session name, and answers its id.
async fn create_record(swarm: &Arc<Swarm>, slug: &str, session: &str) -> String {
    let before: Vec<String> = swarm
        .instances("swarm.manager.Swarm")
        .await
        .iter()
        .map(|made| made["id"].as_str().expect("an identity").to_owned())
        .collect();
    issue(
        swarm,
        "swarm.manager.CreateSwarm",
        json!({"display_name": slug, "tmux_session": session, "home": format!("swarms/{slug}"),
               "created_at": "2026-09-12T10:00:00Z"}),
    )
    .await;
    swarm
        .instances("swarm.manager.Swarm")
        .await
        .iter()
        .map(|made| made["id"].as_str().expect("an identity").to_owned())
        .find(|id| !before.contains(id))
        .expect("a new record")
}

fn swarm_dir(data: &tempdir::TempDir, slug: &str) -> PathBuf {
    data.path().join("swarms").join(slug)
}

/// A slug is joined onto the data root and handed to `remove_dir_all` without being checked for
/// being a single path component. `story:slug-is-not-validated` is open and unfixed, and this is
/// the first verb in the server that destroys anything, so the gap is now an erase-arbitrary-
/// directory primitive reachable from an unauthenticated request.
///
/// `..%2Fescaped` is one path segment on the wire — axum matches `{slug}` against the encoded
/// segment and percent-decodes it in the extractor — so the route is reached with `../escaped`.
#[tokio::test]
async fn a_removal_does_not_reach_outside_the_swarms_directory() {
    let (server, data) = server().await;
    // A server with at least one swarm in it, so `swarms/` is on disk: it is the intermediate
    // component every escaping path walks through, and a deployment that has ever held a swarm
    // has it. A fresh empty root does not, which is the only thing that hid this.
    server.open("ordinary").await.expect("a place is made");
    // Somebody else's data, a sibling of `swarms/` under the same root. `coordinator` records live
    // beside the swarms; this stands in for any of them.
    let outside = data.path().join("escaped");
    std::fs::create_dir_all(&outside).expect("the place is made");
    std::fs::write(outside.join("precious.json"), b"{}").expect("a file is written");

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "DELETE", "/swarms/..%2Fescaped").await;

    assert!(
        outside.join("precious.json").exists(),
        "DELETE /swarms/..%2Fescaped answered {status} ({body}) and erased {}, which is not \
         under the swarms directory at all",
        outside.display(),
    );
}

/// The refusal surface is keyed on the slug STRING in the handle map, and the directory is keyed
/// on the slug PATH. `./busy` and `busy` are the same directory and different map keys, so the
/// state check finds no handle, skips itself entirely, and the files of a `Running` swarm go —
/// while the live handle for `busy` stays in the map over a log that no longer exists.
///
/// The unit's own test asserts a `Running` swarm answers 409. It does, under its own name.
///
/// **The expected status moved from `409` to `400` on 2026-09-12, by the coordinator's decision,
/// and the first assertion below did not move.** What this case exists to prove is that an alias
/// of a live swarm's slug does not erase its directory; that is asserted first and unchanged.
///
/// `409` would require the server to RESOLVE `./busy` to the swarm `busy` — to normalise a
/// caller-supplied path and then answer about whatever it resolved to. That is how the traversal
/// this file's first case found comes back: it puts a path-manipulation step in front of every
/// security decision, and the next person to add a door has to remember to call it. `400` refuses
/// the name and resolves nothing (`state::check`, one rule at `open`, `get`, `remove` and the boot
/// scan, enforced by a test that reads the route table). `409` would be the server admitting it
/// had understood the alias.
///
/// So this is not an expectation bent to fit an implementation: it is the same refusal, taken one
/// step earlier, and the destructive claim it guards is untouched.
#[tokio::test]
async fn a_running_swarm_is_not_removed_under_an_alias_of_its_slug() {
    let (server, data) = server().await;
    let swarm = server.open("busy").await.expect("a place is made");
    let id = create_record(&swarm, "busy", "busy").await;
    let started = issue(
        &swarm,
        "swarm.manager.StartSwarm",
        json!({"swarm_id": id, "started_at": "2026-09-12T10:01:00Z"}),
    )
    .await;
    assert_eq!(started, "started");
    drop(swarm);

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "DELETE", "/swarms/.%2Fbusy").await;

    assert!(
        swarm_dir(&data, "busy").is_dir(),
        "an alias of a Running swarm's slug answered {status} ({body}) and erased its \
         directory without ever consulting the state that refuses it",
    );
    assert_eq!(
        status, 400,
        "an alias of a Running swarm's slug answered {status}: {body} — it has to be refused as a \
         name, before anything resolves it to a swarm",
    );
}

/// `Server::remove` asks `Swarm::summary()` for "the" state, and `summary` keeps the LAST
/// `swarm.manager.Swarm` instance it walks — one state for a world that may hold several records.
/// A slug whose log holds a `Deleted` record and a `Running` one is removable exactly when the
/// `Deleted` one happens to sort last by minted id.
///
/// Two records in one slug is reachable with the commands the server already routes:
/// `POST /swarms/{slug}/commands/swarm.manager.CreateSwarm` twice with different `tmux_session`
/// values, which `SwarmSessionTaken` does not refuse.
///
/// The ordering is not left to chance here: slugs are built until one lands with the `Deleted`
/// record sorting last, so the case is deterministic rather than a coin flip.
#[tokio::test]
async fn a_slug_holding_a_running_record_is_refused_however_its_records_sort() {
    let (server, data) = server().await;

    let mut chosen = None;
    for attempt in 0..40 {
        let slug = format!("mixed-{attempt}");
        let swarm = server.open(&slug).await.expect("a place is made");
        let running = create_record(&swarm, &slug, &format!("{slug}-live")).await;
        let retired = create_record(&swarm, &slug, &format!("{slug}-old")).await;
        issue(
            &swarm,
            "swarm.manager.StartSwarm",
            json!({"swarm_id": running, "started_at": "2026-09-12T10:01:00Z"}),
        )
        .await;
        issue(
            &swarm,
            "swarm.manager.DeleteSwarm",
            json!({"swarm_id": retired}),
        )
        .await;
        drop(swarm);
        // `summary()` walks a map keyed by id and keeps the last one it sees.
        if retired > running {
            chosen = Some(slug);
            break;
        }
    }
    let slug = chosen.expect("a slug whose retired record sorts last");

    let at = serving(Arc::clone(&server)).await;
    let (status, body) = call(at, "DELETE", &format!("/swarms/{slug}")).await;

    assert!(
        swarm_dir(&data, &slug).is_dir(),
        "`{slug}` holds a Running record and a Deleted one; DELETE answered {status} ({body}) \
         and erased the log the Running record lives in",
    );
    assert_eq!(
        status, 409,
        "a slug holding a Running record answered {status}: {body}",
    );
}

/// Diagnostic for the case above: what the wire actually answers for an escaping slug, and what
/// the method answers when called with the same string directly. Kept because the difference
/// between "the router refuses it" and "the unlink refuses it" is the whole reachability question.
#[tokio::test]
async fn what_an_escaping_slug_answers() {
    let (server, data) = server().await;
    server.open("ordinary").await.expect("a place is made");
    let outside = data.path().join("escaped");
    std::fs::create_dir_all(&outside).expect("the place is made");
    std::fs::write(outside.join("precious.json"), b"{}").expect("a file is written");

    let at = serving(Arc::clone(&server)).await;
    for path in [
        "/swarms/..%2Fescaped",
        "/swarms/..%2f..%2fescaped",
        "/swarms/%2e%2e%2fescaped",
    ] {
        let (status, body) = call(at, "DELETE", path).await;
        eprintln!("wire {path} -> {status} {body}");
    }
    eprintln!(
        "direct ../escaped -> {:?}",
        server.remove("../escaped").await
    );
    eprintln!("still there: {}", outside.join("precious.json").exists());
}

/// The third interleaving, the one the race case does not cover: `Server::get` hands out the
/// handle with no gate at all, so a command request that got its `Arc` before the removal started
/// runs after the removal finished — against a log that has been unlinked.
///
/// Both halves are routes: `POST /swarms/{slug}/commands/{command}` and `DELETE /swarms/{slug}`,
/// and the place with no record in it — the one removal exists for — is exactly the place a
/// `CreateSwarm` is racing towards. On Linux the write lands in an unlinked inode and reports
/// success, so the client is told the command was applied and nothing on disk ever held it.
///
/// Either answer is defensible; being told `created` and then finding nothing is not.
#[tokio::test]
async fn a_command_on_a_handle_taken_before_a_removal_is_not_told_it_succeeded() {
    let (server, _data) = server().await;
    server.open("racy").await.expect("a place is made");

    // What a command request holds for the whole of its work.
    let held = server.get("racy").await.expect("the handle is handed out");

    // The removal runs to completion in between, exactly as a DELETE arriving now would.
    server.remove("racy").await.expect("the place is removed");

    let applied = held
        .issue(
            None,
            "swarm.manager.CreateSwarm",
            args(json!({"display_name": "racy", "tmux_session": "racy",
                        "home": "swarms/racy", "created_at": "2026-09-12T10:00:00Z"})),
            "adv-after-removal",
        )
        .await;

    let Ok(outcome) = &applied else {
        // Refused is the honest answer; nothing was claimed that did not happen.
        return;
    };

    // It said `created`. So the record has to be findable by the next open of this slug.
    let again = server.open("racy").await.expect("the slug opens again");
    let made = again.instances("swarm.manager.Swarm").await;
    assert!(
        !made.is_empty(),
        "a command issued on a handle taken before the removal answered `{}` and the record it \
         created is in no log any reader of this slug can reach",
        outcome.outcome,
    );
}

/// `GET /swarms` was the keys of a map. It is now one `summary()` per swarm, each of which takes
/// that swarm's world mutex — the same mutex `issue` and `tick` hold across their commits — and it
/// takes them one after another. A list of ten therefore waits behind ten locks it never touched
/// before.
///
/// Measured, not asserted tightly: the bound is deliberately loose, so a red here is a real stall
/// and not a slow machine.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn listing_does_not_wait_on_every_swarm_s_world() {
    use std::time::Instant;

    let (server, _data) = server().await;
    let mut busy = Vec::new();
    for n in 0..10 {
        let slug = format!("listed-{n}");
        let swarm = server.open(&slug).await.expect("a place is made");
        create_record(&swarm, &slug, &slug).await;
        busy.push(swarm);
    }

    // Every swarm's world under constant command traffic, the way a turning loop holds it.
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut load = Vec::new();
    for swarm in busy.clone() {
        let stop = Arc::clone(&stop);
        load.push(tokio::spawn(async move {
            let mut n = 0u64;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                n += 1;
                let _ = swarm
                    .issue(
                        None,
                        "swarm.manager.UpdateSwarm",
                        args(json!({"swarm_id": swarm.instances("swarm.manager.Swarm").await[0]["id"],
                                    "display_name": format!("n{n}")})),
                        &format!("load-{n}"),
                    )
                    .await;
            }
        }));
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let keys = Instant::now();
    let _ = server.slugs().await;
    let keys = keys.elapsed();

    let shown = Instant::now();
    let listed = server.listed().await;
    let shown = shown.elapsed();

    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for task in load {
        let _ = task.await;
    }

    eprintln!(
        "slugs() {keys:?}  listed() {shown:?}  ({} shown)",
        listed.len()
    );
    assert!(
        shown < std::time::Duration::from_millis(500),
        "listing ten swarms under command traffic took {shown:?} against {keys:?} for the keys \
         it used to be: the list now waits on every swarm's world lock in turn",
    );
}

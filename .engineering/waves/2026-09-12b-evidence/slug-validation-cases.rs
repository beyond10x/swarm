// Two adversary cases from correction round 1 of unit B, parked unapplied.
//
// They belong to the two stories the coordinator is filing (slug validation in `ess-runtime`'s
// `Store::open`, and directory-identity rather than string-identity for the swarm map). Both were
// RED as written, against `swarm-server` at 8cabdd7 — see the round-1 report. They are not unit B's
// to fix, and leaving them in the package's default suite would make the gate red for a defect that
// is not this unit's, so they are lifted out here verbatim, minus nothing but their file header.
//
// To restore: append to a test target in `src/runtime/swarm-server/tests/` that carries the
// `kernel()` and `server()` helpers from `open_is_one_swarm_under_contention.rs`, and
// `use std::sync::Arc;`.

/// An open never creates a directory outside the `swarms` directory.
///
/// `Server::open` passes its slug straight to `Store::open`, which does
/// `root.join("swarms").join(slug)` and `create_dir_all` on the result *before* anything validates
/// the slug (`ess-runtime/src/store.rs:95-106`). A slug containing `..` therefore makes a directory
/// the module doc says cannot exist — "a swarm that exists is a directory that exists" under
/// `swarms/` (`state.rs:47-48`) — and the refusal that follows does not remove it. One `..` is
/// asserted on here because it stays inside the test's own scratch directory; the depth is the
/// caller's choice, and two would be above the data root.
///
/// Reached from outside this crate by `POST /swarms` with `{"slug": "../escaped"}`
/// (`http.rs:99-104`): the body's `slug` is an unvalidated `String`.
#[tokio::test]
async fn an_open_never_creates_a_directory_outside_the_swarms_directory() {
    let (server, data) = server().await;
    let root = data.path().to_path_buf();

    let outcome = match server.open("../escaped").await {
        Ok(swarm) => format!("Ok({})", swarm.slug()),
        Err(why) => format!("Err({why})"),
    };

    let escaped = root.join("escaped");
    assert!(
        !escaped.exists(),
        "`open(\"../escaped\")` created {}, which is not under {}, and returned {outcome}",
        escaped.display(),
        root.join("swarms").display()
    );
}

/// Two slugs naming one directory are one swarm, not two handles over one log.
///
/// The change under attack dedupes on the slug *string*: `swarms.entry(slug.to_owned())`. A swarm
/// is not a string — `state.rs:47-48` says "a swarm that exists is a directory that exists", and
/// `Store::open` resolves the directory with `root.join("swarms").join(slug)`. `"alias"` and
/// `"./alias"` are two keys and one directory, so both miss the map, both open the one
/// `eventlog.sqlite3`, and both are inserted. That is two live `Swarm` handles over one log — the
/// state the doc says "nothing can observe" — reached without any concurrency at all.
///
/// The harm is the one the change's own doc names: each handle holds an in-memory world and an
/// `owed` queue of its own, so what one handle applies the other cannot see and will not redeliver.
///
/// Reached from outside this crate by `POST /swarms` (`http.rs:99-104`): `NewSwarm.slug` is a
/// `String` with no validation on any path between the socket and `Store::open`.
#[tokio::test]
async fn two_slugs_naming_one_directory_are_one_swarm() {
    let (server, data) = server().await;

    let plain = server.open("alias").await.expect("`alias` opens");
    let dotted = server.open("./alias").await.expect("`./alias` opens");

    // One directory, by construction: this is what both opens resolved to.
    let directory = data.path().join("swarms").join("alias");
    assert!(directory.is_dir(), "the one swarm directory exists");

    let input = serde_json::json!({"display_name": "Alias", "tmux_session": "alias",
                                   "home": "/tmp/alias", "created_at": "2026-09-12T10:00:00Z"});
    plain
        .issue(
            None,
            "swarm.manager.CreateSwarm",
            input.as_object().expect("an object").clone(),
            "adversary-1",
        )
        .await
        .expect("the command applies");

    let seen_by_dotted = dotted.instances("swarm.manager.Swarm").await.len();
    let seen_by_plain = plain.instances("swarm.manager.Swarm").await.len();
    assert!(
        Arc::ptr_eq(&plain, &dotted),
        "`alias` and `./alias` are one directory ({}) but two handles: the second holds a world \
         without the swarm the first just created ({seen_by_dotted} instances against \
         {seen_by_plain}), and an `owed` delivery queued on either is invisible to the other",
        directory.display()
    );
}

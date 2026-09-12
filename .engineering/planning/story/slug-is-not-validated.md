---
format: aep.planning-md/1
id: story:slug-is-not-validated
kind: story
status: draft
title: A swarm slug is not validated before a directory is made
summary: open("../escaped") returns Ok, and alias and ./alias are two handles over one log.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/ess-runtime/src/store.rs
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs
revision: 4
---
## What

`Store::open` runs `create_dir_all` on `root/swarms/<slug>` before anything validates the slug, and
`POST /swarms` takes the slug from the request body as an unvalidated `String`
(`src/runtime/swarm-server/src/http.rs:99-104`). Two measured consequences:

- `open("../escaped")` returns **`Ok`** and leaves a working, registered swarm outside the swarms
  directory.
- `alias` and `./alias` are one directory and two live `Swarm` handles over one `eventlog.sqlite3`,
  because `Server::open` dedupes on the slug string and not on the directory. Their in-memory worlds
  diverge — 0 instances against 1 — and an `owed` delivery queued on either is invisible to the
  other.

Measured 2026-09-12 by the adversary of unit B of the 2026-09-12b bug wave
(`review-result:adversary-2026-09-12b-unit-b-pass-1`, findings 2 and 3, both `CONFIRMED`, both
`origin: pre-existing`), in `tests/open_is_one_swarm_under_contention.rs:94` and `:114`.

The second is the same harm `story:open-is-a-read-then-insert` was filed for — a handle holding the
only copy of an owed delivery — reached with no concurrency at all. That story fixed the race; this
one is the other door into the same loss.

## Acceptance

A slug that is not a single path component is refused before any directory is created, and the
refusal is the answer `POST /swarms` gives. `alias` and `./alias` cannot both name a swarm. A test
asserts both.

## Out of scope

Any change to how a swarm is opened once its slug is accepted.

## Scope

- **Files:** `src/runtime/ess-runtime/src/store.rs:95` (`Store::open`, the `create_dir_all`) — cited, quoted by the adversary
- **Files:** `src/runtime/swarm-server/src/state.rs:88` (`Server::open`, the `entry(slug.to_owned())`) — cited
- **Files:** `src/runtime/swarm-server/src/http.rs:99` (the unvalidated body field) — cited
- **Files:** `src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs` — inferred, the adversary's two cases live there and are red today
- **Confidence:** high — every path was opened and every claim measured by a test that ran
- **Would collide with:** any unit touching `state.rs`, `http.rs` or `ess-runtime`'s `store.rs`

## Notes

The two cases that measured this are kept, verbatim, at
`.engineering/waves/2026-09-12b-evidence/slug-validation-cases.rs`. Unit B lifted them out of its
package suite rather than weakening them: they cannot stay red inside `swarm-server`'s default
suite without `#[ignore]`, and the only alternative — a `[[test]] test = false` target in
`Cargo.toml` — was not a file that unit owned. No assertion in them was relaxed. They need the
`kernel()` and `server()` helpers from `tests/open_is_one_swarm_under_contention.rs` and
`use std::sync::Arc;` to compile.

Both were red at `8cabdd7`:

- `open("../escaped")` returned `Ok(../escaped)` and created `<root>/escaped`, outside `<root>/swarms`.
- `alias` and `./alias` gave two handles over one directory; after `CreateSwarm` on the first, the
  second saw 0 instances against 1.

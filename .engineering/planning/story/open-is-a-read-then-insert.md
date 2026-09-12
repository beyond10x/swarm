---
format: aep.planning-md/1
id: story:open-is-a-read-then-insert
kind: story
status: active
title: Server::open can replace a live Swarm handle
summary: A read-then-insert across an await lets a second open drop the first handle and whatever it owed.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/serves_a_swarm.rs
revision: 5
---
## What

`Server::open` in `src/runtime/swarm-server/src/state.rs:75-85` reads the swarm map under a read
lock, drops the guard, awaits `Swarm::open`, then inserts under a write lock with no re-check:

```
if let Some(open) = self.swarms.read().await.get(slug) { return Ok(...) }
let swarm = Arc::new(Swarm::open(...).await?);
self.swarms.write().await.insert(slug.to_owned(), Arc::clone(&swarm));
```

Two concurrent opens of one slug both miss the read, both construct a handle, and the second
insert replaces the first. The replaced handle may hold the only copy of an owed delivery, which
is then lost with no `give_up`, no `What::Undelivered` and no log line.

Found 2026-09-12 by the adversary of unit A of that day's wave
(`review-result:adversary-unit-a-pass-1`, `origin: pre-existing`, `verdict: INFEASIBLE`) and
re-verified on `main` at `b66ef29` on 2026-09-12. INFEASIBLE then and now: the only caller drops
its handle without issuing, so nothing in the shipped surface reaches the loss. The race itself is
reachable; what nothing reaches today is a lost delivery from it.

## Acceptance

Two concurrent `Server::open` calls for one slug return the same `Arc<Swarm>`, and a test asserts
it. The losing handle is dropped rather than inserted.

## Out of scope

Any change to `Swarm::open` itself, and any cross-process coordination. This is one process.

## Scope

- **Files:** `src/runtime/swarm-server/src/state.rs:75` (`Server::open`) — cited, read on `main` at `b66ef29`
- **Files:** `src/runtime/swarm-server/tests/serves_a_swarm.rs` — inferred, the verifier named it or a `state.rs` unit module as where a regression test would go
- **Confidence:** high — one function, read in the tree, and the fix named is `entry(slug).or_insert` or a re-check under the write guard
- **Would collide with:** any unit touching `state.rs`, which is five of the seven other stories

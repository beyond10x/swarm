---
format: aep.planning-md/1
id: story:open-is-a-read-then-insert
kind: story
status: implemented
title: Server::open can replace a live Swarm handle
summary: A read-then-insert across an await lets a second open drop the first handle and whatever it owed.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/open_does_not_queue_behind_an_unrelated_slug.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/serves_a_swarm.rs
revision: 8
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

Rewritten 2026-09-12 from the implementor's confirmation after the unit merged.

**What the unit actually touched**, read from `git diff 0c24ac0...wave/2026-09-12b/open-race`:

| path | the mark it carried | what it turned out to be |
|---|---|---|
| `src/runtime/swarm-server/src/state.rs` | cited | touched. `Server::open`, the `opening` per-slug lock map, and three doc corrections |
| `src/runtime/swarm-server/tests/serves_a_swarm.rs` | **inferred** | **right.** The concurrent-open case belongs there and not in a new file; its retry loop was removed |
| `src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs` | not scoped | **new file**, the adversary's, kept. Two cases; two more were lifted out to `.engineering/waves/2026-09-12b-evidence/slug-validation-cases.rs` because they belong to `story:slug-is-not-validated` |
| `src/runtime/swarm-server/tests/open_does_not_queue_behind_an_unrelated_slug.rs` | not scoped | **new file**, the adversary's, kept. It measures the 253 ms an unrelated slug waited under the first, global lock |

The one inferred line was right. No path outside `swarm-server` was touched: the fix is one
function and its lock map.

- **Confidence:** settled. Read from the merged diff.
- **Collides with:** any unit touching `state.rs` or `swarm-server`'s test directory. `ess-runtime`
  is **not** this story's surface — the SQLITE_BUSY it answers is answered by not producing the
  contention, and `store.rs`'s no-`busy_timeout` policy is untouched.

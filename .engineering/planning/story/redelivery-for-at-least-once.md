---
format: aep.planning-md/1
id: story:redelivery-for-at-least-once
kind: story
status: implemented
title: Redeliver an at-least-once binding
summary: A binding that declares retry is retried.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/runtime/ess-runtime/src/route.rs
- confidence: inferred
  path: src/runtime/ess-runtime/src/store.rs
- confidence: cited
  path: src/runtime/ess-runtime/tests/routes_the_dataflow.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
revision: 14
---
## What

Three bindings declare `delivery: at_least_once` with `on_failure: retry`, and the pump does
nothing for them: it records that the binding wanted redelivery and drops the work. The gap is
deliberate and named — `needs_redelivery()` exists so a caller can find out which bindings it is
under-serving — and it is a promise the specification makes that the runtime does not keep.

## Acceptance

A binding declaring `at_least_once` with `retry` whose command fails is retried, with a bounded
number of attempts and a delay between them, and a caller can read how many attempts a delivery has
had. `needs_redelivery()` returns empty for the kernel, or the test that asserts its contents says
which bindings are still under-served and why.

## Out of scope

Exactly-once. The specification does not offer it and neither should this.

## Notes

The story said this belongs beside the trigger because "the pump has no clock and no store by
design". Both halves were checked on 2026-09-12 and both are wrong in a way that matters.

`ess-runtime` owns `store.rs` — the SQLite eventlog — so "no store by design" is module-scoped to
`route.rs`'s purity, not a property of the crate.

More importantly the trigger never sees these bindings. All three that declare
`at_least_once`/`retry` are event-caused, and `Server::periods()` filters on
`binding.cause.periodic()`, so a failed delivery arises only on the `Swarm::issue()` path. The
defect site is `swarm.rs`, where `pump()` hands the failure out as `Routed.result` and the host
writes `if let Ok(applied)` with no `else`. `route.rs`'s comment "retrying is the caller's business"
is already satisfied; the caller declines the business.

Two things this story must decide and does not: whether the attempts record lands in the shared
eventlog or a server-local table, and whether a pending delivery held in memory satisfies
"at least once" when the process dies.

## Scope

Rewritten 2026-09-12 from the implementor's confirmation, after the work landed. The pre-build
scope is in `review-result:adversary-unit-a-pass-1`'s history; what follows is where it went.

**Landed on, all confirmed by building:**

- `src/runtime/swarm-server/src/swarm.rs` — the defect and the fix. `issue()` wrote `if let Ok(applied)` with no `else`. It now holds a per-swarm queue, and `owe`, `take_due`, `keep_owed` and `deliveries` are the only four methods that name it.
- `src/runtime/swarm-server/src/trigger.rs` — the drain, in its own pass over every swarm before any is fired.
- `src/runtime/ess-runtime/src/route.rs` — `Routed` gained `input`. The scope guessed this file for a different reason; the real one is that `Routed` carried no input either, so a host could not re-invoke at all without re-deriving the mapping.
- `src/runtime/ess-runtime/tests/routes_the_dataflow.rs`, `swarm-server/tests/serves_a_swarm.rs`, and two new test files.

**Guessed and wrong, which is the useful half:**

- `src/runtime/ess-runtime/src/store.rs` — **not touched.** The scope expected an attempts table in the shared eventlog. `store.rs` writes only what an outcome emitted, and the local rule requires every written field to appear on a declared event; there is no ESS event for "a delivery was attempted", so writing one would be the runtime inventing a record the specification does not declare. Attempts are server-local and die with the process, which is stated rather than hidden.
- `src/runtime/swarm-server/src/state.rs` — **not touched.** The scope expected an accessor, because `periods()` filters to periodic causes and would not serve a retry queue. True, and irrelevant: the queue is per-`Swarm`, and the trigger already walks every swarm.

**What the work cost beyond the story.** Two adversary passes found eleven findings. The first correction resolved all seven and introduced a deadlock — `redeliver` held the queue's lock across a call that took the same lock, hanging the swarm's world with it. The second pass found it. No shipped binding reaches it; the documented `components.yaml` extension point does.

`tick` now runs the pump, which is a behaviour change and not a doc fix: an event a periodic command emits reaches every binding that names it. Invisible against the shipped kernel, because nothing binds what `Pursue` emits.

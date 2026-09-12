---
format: aep.planning-md/1
id: story:redelivery-for-at-least-once
kind: story
status: active
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
revision: 12
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

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/swarm-server` — the failure is produced and discarded here, not beside the trigger. See Notes: the story's first reading of this was wrong.
- **Files (cited):** `swarm.rs` (`issue()` loops the pump's results and handles only `Ok`; the `Err` arm is dropped with no record), `trigger.rs` (the only clock, from which a delayed re-attempt would drain), `ess-runtime/tests/routes_the_dataflow.rs` (the assertion the acceptance names).
- **Files (inferred):** `ess-runtime/src/store.rs` (an attempts table; the existing API has no pending-delivery surface), `route.rs` (`Routed` carries no failure policy, so the host must cross-reference `needs_redelivery`), `state.rs` (`periods()` filters to periodic causes and will not serve a retry queue).
- **Confidence:** high for the files, medium for the split — whether the attempts record belongs in the shared eventlog or a server-local table is an open design call.
- **Would collide with:** any unit touching the post-pump commit loop, the trigger's driver loop, the store schema, or the `needs_redelivery` assertion.
- **Not established:** whether the acceptance's first branch is reachable. `needs_redelivery()` is a pure function of the IR and cannot observe that a host now serves those bindings.

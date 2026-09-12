---
format: aep.planning-md/1
id: story:honour-view-consistency
kind: story
status: draft
title: Honour or declare view consistency
summary: Eventual views are served immediately; make that true or make it stated.
relations:
- decomposes: epic:executable-boxes
scope:
- confidence: cited
  path: src/runtime/ess-runtime/src/views.rs
- confidence: inferred
  path: src/runtime/ess-runtime/tests/computes_views.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/web/src/runtime.ts
revision: 8
---
## What

The specification declares each view `read_your_writes` or `eventual`, and the runtime serves both
immediately from the current world. That is a deliberate deviation and it is recorded in a comment,
which is the weakest place a deviation can live: nothing fails when the two drift apart, and a
reader of the specification is told something the runtime does not do.

## Acceptance

Either the runtime honours the declared consistency for `eventual` views, or the deviation is a
first-class fact: the specification says the runtime serves every view immediately, a test asserts
it, and `/spec` reports the declared consistency per view so a caller can see what it is not getting.

## Out of scope

A projection store. `eventual` in this system means "may lag", not "lives elsewhere".

## Notes

Six of the kernel's twenty-seven views declare `eventual`, not the nine this story first claimed —
counted 2026-09-12 across the six domain files, one each in `agent.yaml` (twice), `blackbox.yaml`,
`config.yaml`, `mailbox.yaml` and `manager.yaml`.

`views.rs:101` is the whole of the decision: `consistency: declared.consistency` copies the
declaration onto the result and nothing consults it again. The prose form of the deviation is the
module comment at `views.rs:15-19`.

`Consistency` comes from `ess-domain`, a git-pinned crate, so it is read here and not editable.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/ess-runtime/src/views.rs` — cited; the deviation is its module comment and the decision is one line, `consistency: declared.consistency`, which copies the declaration onto the result and never consults it.
- **Files (cited):** `swarm-server/src/state.rs` (`/spec` reports views as bare strings; reporting consistency means a structured entry), `src/web/src/runtime.ts` (the typed client mirrors that shape and changes with it).
- **Files (inferred):** `ess-runtime/tests/computes_views.rs` (the acceptance requires a test), the domain files if the specification is amended to state that every view is served immediately.
- **Confidence:** high. The file, the deviation comment and the decision line were all read.
- **Would collide with:** any unit touching view computation or the `/spec` payload shape and its web mirror.
- **Not established:** whether `Consistency` has variants beyond the two. It comes from a git-pinned crate that is not in this tree.

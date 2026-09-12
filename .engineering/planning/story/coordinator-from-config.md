---
format: aep.planning-md/1
id: story:coordinator-from-config
kind: story
status: draft
title: Read the coordinator from the swarm's config
summary: HarnessLaunch decides which coordinator a swarm runs, not an environment variable.
relations:
- decomposes: epic:executable-boxes
scope:
- confidence: inferred
  path: examples/coordinator-manual.sh
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: inferred
  path: src/web/src/components/runtime/LoopPanel.vue
- confidence: inferred
  path: src/web/src/components/runtime/RuntimeBar.vue
- confidence: inferred
  path: src/web/src/runtime.ts
revision: 12
---
## What

Which program runs as a swarm's coordinator is read from the `SWARM_COORDINATOR` environment
variable, so every swarm in one runtime runs the same one and a swarm cannot say what it wants.
`swarm.config.HarnessLaunch` already declares `harness`, `binary`, `model`, `args`,
`tool_surface` and `allow_program`, and nothing reads it.

## Acceptance

A swarm with an activated `Config` runs the coordinator that config names, and two swarms in one
runtime with different configs run different coordinators. A swarm with no activated config falls
back to the environment, and the status endpoint says which of the two it used.

## Out of scope

Drafting configs from the UI.

## Notes

The shortcut is named in `coordinator.rs`'s module documentation. `ActivateConfig` and
`AdoptConfig` already exist and the binding between them is declared.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/swarm-server` — cited; the story names the shortcut's site.
- **Files (cited):** `coordinator.rs` (the module doc names `SWARM_COORDINATOR`, and `configured()` reads it), `state.rs` (`Server::status` calls `configured()` and fills the reply the acceptance requires to name its source).
- **Files (inferred):** `trigger.rs` (where a per-swarm launch resolves), `swarm.rs` (the activated config is read from the swarm's own world), the two web files that read `status.coordinator.program`, `examples/coordinator-manual.sh`.
- **The specification is read, not changed.** `HarnessLaunch` already declares every field, `Swarm.active_config_id` exists, and `adopt-activated-config` already sets it. Nothing in the acceptance needs a new field, command, event or binding.
- **Confidence:** high for the runtime surface, medium for the web and test files, which follow from a status-shape change rather than from the story's words.
- **Not established:** whether the runtime can read an Active config per swarm at all — no `swarm.config` reference exists under `swarm-server/src` except the shortcut comment — so the read path may be new code rather than an edit.

## Observed

**Observed running, 2026-09-12, not inferred.** The operator created a swarm, gave it a coordinator,
and the UI answered *"no coordinator is configured, so nothing can say whether the goal is met"*.

Three measurements, in order:

1. `coordinator::configured()` at `src/runtime/swarm-server/src/coordinator.rs:249` reads
   `std::env::var("SWARM_COORDINATOR")` and returns `None` when it is unset. It consults no `Config`
   record and no `HarnessLaunch` field. The function's own doc comment says so and calls it a
   shortcut.
2. The running server, pid 1805682, has **no `SWARM_*` variable in its environment** —
   `tr '\0' '\n' < /proc/1805682/environ | grep ^SWARM` returns nothing.
3. `GET /status` on that process answers
   `"coordinator": {"configured": false, "program": null, "in_flight": 0}`.

So the message is true of the runtime and false of the swarm. `swarm.config.HarnessLaunch` declares
`harness`, `binary`, `model`, `args`, `tool_surface` and `allow_program`, and nothing reads any of
them; a swarm can carry a coordinator in its own record and still be told there is none.

**The gap is not only that the environment wins. It is that the two disagree silently.** A swarm
whose config names a coordinator gets the same words as a swarm that never had one, so the operator
cannot tell a missing setting from an ignored one. Whatever this story does about reading the
config, the refusal has to say which of the two it is.

Until it is fixed, the coordinator comes from the environment of the server process:
`SWARM_COORDINATOR=metaharness cargo run -p swarm-server`, or
`SWARM_COORDINATOR=examples/coordinator-manual.sh cargo run -p swarm-server` for a loop that turns
without spending model budget.

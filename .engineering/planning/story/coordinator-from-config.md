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
revision: 11
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

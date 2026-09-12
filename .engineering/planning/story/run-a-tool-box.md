---
format: aep.planning-md/1
id: story:run-a-tool-box
kind: story
status: draft
title: Run a tool box
summary: A drawn Tool box executes its program when an event reaches it.
relations:
- decomposes: epic:executable-boxes
scope:
- confidence: inferred
  path: src/core/components.yaml
- confidence: cited
  path: src/core/domains/blackbox.yaml
- confidence: inferred
  path: src/runtime/swarm-server/src/box_runner.rs
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: cited
  path: src/runtime/swarm-server/src/lib.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/trigger.rs
revision: 10
---
## What

A `swarm.blackbox.Box` of kind `Tool` or `Service` is declared, drawn, connected and executed by
nothing. A swarm can describe reaching the world and cannot reach it.

Give the runtime a box runner: when an event arrives at a Live `Connection` whose target box is a
Tool, run the box's program with the event as input and publish what it returned.

The first two boxes worth having are the ones the operator named: an OS notification
(`notify-send`), and an arbitrary CLI command.

## Acceptance

A `Box{kind: Tool}` whose `ref_id` names a program, connected from an event-producing box, runs that
program when the event occurs; its exit status and output are recorded as an event on the swarm's
log and visible in the UI's event panel. A box whose program is not on an allow list is refused and
the refusal is recorded.

## Out of scope

Confinement. The runner spawns on the host, the way the coordinator already does; saying so is the
deliverable, not sandboxing it.

## Notes

`Connection` already carries `delivery` and `on_failure`, so the retry policy is declared rather
than invented. `blackbox.yaml` declares `BoxKind` with `Tool`, `Service` and `Mcp` already.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/swarm-server` — cited; the story says the runner spawns on the host the way the coordinator already does, and host spawning lives here.
- **Files (cited):** `src/runtime/swarm-server/src/lib.rs` (the `pub mod` list a new module registers in), `src/runtime/swarm-server/src/coordinator.rs` (the existing spawn path and its environment allowlist), `src/core/domains/blackbox.yaml` (declares `BoxKind`, `Box.ref_id`, `Connection.delivery`/`on_failure`; holds no run command and no ran/refused event, so the new ones land here).
- **Files (inferred):** a new `src/runtime/swarm-server/src/box_runner.rs`, `swarm.rs` (`announce`/`issue` are the only ways a host effect reaches the log and the stream), `components.yaml` (the event-to-command binding), `trigger.rs`.
- **Confidence:** medium. The crate and the spec files are cited; nothing in the Rust tree references `Connection` or box execution yet, so module placement and binding shape are worked out rather than read.
- **Would collide with:** any unit touching swarm-server's module list or host-process path, or editing `blackbox.yaml` or `components.yaml`.
- **Not established:** where the program allow list lives — the only allowlist in the tree is the seven-key environment one — and no run/ran/refused surface exists in the specification, so those names are entirely new.

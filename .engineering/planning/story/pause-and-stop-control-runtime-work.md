---
format: aep.planning-md/1
id: story:pause-and-stop-control-runtime-work
kind: story
status: draft
title: Pause and stop control every runtime-owned turn
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/core/domains/manager.yaml
- confidence: cited
  path: src/runtime/swarm-server
- confidence: cited
  path: src/runtime/swarm-server/Cargo.toml
- confidence: cited
  path: src/runtime/swarm-server/src/control.rs
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
- confidence: cited
  path: src/runtime/swarm-server/src/lib.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/adversary_control_pass_1.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/adversary_control_pass_2.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/pause_and_stop_control_runtime_work.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_cascade_of_a_caused_command.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
revision: 15
---
## Context

`src/core/domains/manager.yaml:84` declares Created, Running, Paused, Stopped and Deleted; PauseSwarm says gates are not evaluated and StopSwarm tears down the session while preserving records (`:239`, `:283`). `src/runtime/swarm-server/src/trigger.rs:215` dispatches OpenAssignments without checking the swarm state. Its coordinator dispatch also starts Pursuing goals independently of the Running check on periodic firing. `coordinator.rs:1145` spawns a child and `:1233` waits for it without a lifecycle cancellation path. A prior local review reproduced AssignmentDone in both Paused and Stopped using a fake worker, without a model call.

## Outcome

Pause and stop govern every runtime-owned coordinator and worker turn, including already-running turns and races with dispatch.

## Proposed semantics

Both successful PauseSwarm and StopSwarm close admission and cancel/reap runtime-owned active turn processes for that swarm before reporting successful host completion. Pause preserves resumable goal/assignment state, frames, transcripts, and spend; Resume permits eligible work to continue. Stop preserves records and work directories; only a later StartSwarm permits another launch. The server and canvas remain available in both states. This is the proposed process-host interpretation of the existing lifecycle; align the legacy tmux/crons descriptions with it without inventing states or events.

Cancellation owns the directly launched process and its ordinary process-group descendants. It does not promise containment of a deliberately escaped daemon. Use a bounded termination policy, expose a termination failure rather than claim a successful stop while a child survives, and never silently discard already-observed usage. Keep the generic ESS interpreter independent of swarm lifecycle rules.

## Acceptance

Through the real HTTP command path, a running swarm becomes quiescent on successful pause or stop, with no runtime-owned process or late verdict surviving the acknowledged cancellation, as demonstrated by deterministic fake-process integration tests.

## Required evidence

- Created, Paused, Stopped and Deleted do not launch waiting assignments or Pursuing goals; Running still does.
- Barrier-controlled tests race lifecycle commands against claim acquisition, child launch and result publication; no stale completion becomes GoalReached or AssignmentDone after acknowledged cancellation, including after a rapid restart.
- A fake launcher with an ordinary child process is terminated and reaped for both coordinator and worker paths; another swarm continues unaffected. Tests use process/pipe handshakes, with timeouts only as failure bounds, and spend no model budget.
- Resume/start eventually dispatches each still-eligible unit with at most one concurrent continuation per unit; observed partial transcript and usage survive cancellation, and termination errors are visible.
- Replayed non-running swarms remain non-running on server restart. Repeated lifecycle commands keep the specification's existing wrong-state outcomes.

## Scope boundaries

Existing Swarm, Agent, Assignment and Goal declarations are in `src/core/domains/manager.yaml`, `agent.yaml` and `goal.yaml`. No new entity is proposed. Expected edits: the manager's host descriptions; `src/runtime/swarm-server/src/{trigger,coordinator,state,swarm,http}.rs`; a focused integration test. A new server-local cancellation module is allowed if the implementation justifies it. Root manifests, README, AGENTS.md and derived website facts are coordinator-owned integration changes, not implementor writes.

## Out of Scope

Credential-based authorization, preventing commands deliberately submitted by external clients, OS containment, aggregate spending policy, fixing unrelated accounting I/O failures, changing goal acceptance policy, and UI issuer display. Cancellation must not widen the tool frame.

## Failure semantics

The specification transition may already be committed when termination fails. Keep admission closed, return an explicit host error naming the failed quiescence, and retain a retryable cancellation handle; do not roll back the event or claim host completion. Existing repeated-command wrong-state outcomes remain intact, while cancellation cleanup remains retryable independently of repeating the domain transition. A stopped/paused process generation never publishes a late verdict into a resumed generation. Exact timeout values and the retry mechanism must be documented by the implementor and verified with a forced-failure test.

## Scope

Final scope confirmed against unit 6d84959 and integration merge bb44d5b.

- Cited implementation surfaces: src/runtime/swarm-server/src/{control,coordinator,http,lib,swarm,trigger}.rs and manager.yaml host descriptions. Cancellation admission, process ownership, result publication and HTTP acknowledgement are host behavior; the kernel's entities/events remain unchanged.
- Planning inferred a new module declaration and focused HTTP/process target. Confirmed: control.rs, lib.rs and tests/pause_and_stop_control_runtime_work.rs now exist in the diff. Pass-1 and pass-2 adversary targets are retained as tests/adversary_control_pass_{1,2}.rs.
- Planning inferred a signal dependency. Confirmed at coordinator bootstrap c935d85: Unix libc was added to the server manifest before dispatch. The implementor needed no further manifest change.
- state.rs was inspected as a possible ownership surface but did not change; authoritative cancellation registration lives on Swarm. Its existing scheduling counter remains intact. It is removed from the landed typed scope, preserving this correction to the original inference.
- Two existing fixtures in the_cascade_of_a_caused_command.rs and the_turn_cap_under_attack.rs gained valid Running setup; their assertions remain intact.
- Coordinator-owned integration surfaces are AGENTS.md, README.md, root lock/workspace wiring and website derived facts. No generic ESS runtime behavior changed for this story.
- All declared landed paths are cited from the final diff. Linux is the verified process host; ordinary process-group descendants are included, intentionally escaped groups and other platforms are not asserted as verified.

## Implementation evidence

Implemented locally in wave/2026-09-13c/control through 6d84959; merged into the integration branch at bb44d5b. Main publication and the full integration gate are still pending. Do not implement this story again merely because its lifecycle remains draft; only the operator moves it.

Pause/stop acknowledgement waits for runtime-owned turns and ordinary descendants to quiesce. Host failure is HTTP500 kind host with closed admission; POST /swarms/{slug}/quiesce retries cleanup without repeating a domain transition. Running refuses that retry route. Start/resume finish old cleanup before reopening. Result generation checks prevent old verdicts from reaching a resumed generation. Worker FinishAssignment and GoIdle share lifecycle exclusion, without claiming a database transaction across both commands.

Final unit package: 135 passed (including three child-process entrypoint tests); formatting and strict Clippy passed. Original specification checks passed after the manager summary edits. Adversary finding trend: 1 → 0, carried0/new0/resolved1. Reports are review-result:adversary-2026-09-13c-unit-a-pass-{1,2}; the one finding has a fixed outcome. The final adversary's Python fake process was replaced by a Rust child fixture; the coordinator inspected the exact fixture-only diff, and the behavioral body, assertions and queue order compare byte-for-byte unchanged. Both contention tests and the package reran green. No third attack occurred.

Retained logs/reports: /home/timo/.cache/swarm-wave-2026-09-13c/control/{report.md,correction-1-report.md,fixture-correction-report.md,adversary-1/report.md,adversary-2/report.md}. Wave notes: .engineering/waves/2026-09-13c-control-and-rust-checker.md.

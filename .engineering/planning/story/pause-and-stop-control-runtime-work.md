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
- confidence: inferred
  path: src/runtime/swarm-server/Cargo.toml
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/lib.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/pause_and_stop_control_runtime_work.rs
revision: 12
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

Derived 2026-09-13 by `story-scoper`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/runtime/swarm-server` — cited; runtime dispatch, turn processes, claims and HTTP command completion live here.
- **Files:** `src/runtime/swarm-server/src/trigger.rs` — cited; coordinator and assignment dispatch claim work without checking the swarm lifecycle, and spawn detached turn tasks.
- **Files:** `src/runtime/swarm-server/src/coordinator.rs` — cited; `run_turn` owns process launch, streaming evidence and usage, child waiting and verdict parsing; `take_a_turn` and `work_an_assignment` publish final domain commands.
- **Files:** `src/runtime/swarm-server/src/state.rs` — cited; `Server::claim`, `release` and `turns_in_flight` currently track unit keys without cancellation or a lifecycle generation.
- **Files:** `src/runtime/swarm-server/src/swarm.rs` — cited; `Swarm::issue_as` serializes domain command application and persistence; lifecycle admission and late-result rejection must agree with its state changes.
- **Files:** `src/runtime/swarm-server/src/http.rs` — cited; `issue_command` currently returns immediately after `issue_as`, so successful host completion needs a termination/reaping barrier.
- **Files:** `src/core/domains/manager.yaml` — cited; PauseSwarm and ResumeSwarm describe crons/windows, while StopSwarm describes session teardown; the story explicitly requires aligning these descriptions without changing states or events.
- **Tests:** `src/runtime/swarm-server/tests/pause_and_stop_control_runtime_work.rs` — inferred; proposed focused HTTP and fake-process integration target for dispatch races, cancellation, continuation, evidence retention and swarm isolation.
- **Also likely:** `src/runtime/swarm-server/src/lib.rs` — inferred; an optional server-local cancellation module needs a module declaration.
- **Also likely:** `src/runtime/swarm-server/Cargo.toml` — inferred; process-group termination may require a server-local signal dependency; the existing manifest has Tokio process support but no direct signal-management dependency.
- **Symbols:** `trigger::ask_the_coordinator`, `trigger::work_the_assignments`, `trigger::one_turn`, `trigger::one_assignment`, `coordinator::run_turn`, `coordinator::take_a_turn`, `coordinator::work_an_assignment`, `Server::claim`, `Server::release`, `Swarm::issue_as`, `http::issue_command` — cited.
- **Documents:** manager command descriptions are the only implementor-owned prose explicitly requested; README, AGENTS.md and derived website facts belong to integration — cited.
- **Confidence:** high — cited; the story identifies the defect sites and reading the implementation confirms the missing admission, cancellation and result-publication barriers.
- **Would collide with:** other changes to server dispatch, claims, process execution, command application or the HTTP command handler, and manager lifecycle descriptions — inferred.

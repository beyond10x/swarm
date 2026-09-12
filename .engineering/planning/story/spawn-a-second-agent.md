---
format: aep.planning-md/1
id: story:spawn-a-second-agent
kind: story
status: draft
title: Spawn a second agent that actually runs
summary: A role other than Coordinator, and a spawned agent the runtime runs.
relations:
- decomposes: epic:executable-boxes
scope:
- confidence: cited
  path: src/core/domains/agent.yaml
- confidence: cited
  path: src/runtime/swarm-server/src/budget.rs
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: inferred
  path: src/web/src/components/runtime/TranscriptPanel.vue
- confidence: inferred
  path: src/web/src/runtime.ts
revision: 11
---
## What

`swarm.agent.Role` declares exactly one variant, `Coordinator`, so a coordinator that spawns a
second agent must call it a coordinator. And a spawned agent is a record: nothing runs it, so the
roster a swarm builds cannot do any work.

## Acceptance

A coordinator can spawn an agent with a role that is not `Coordinator`, that agent gets a mailbox,
and the runtime runs it as its own metaharness session against an assignment — so two agents in one
swarm do work at the same time and each one's transcript is recorded separately.

## Out of scope

Agents spawning agents. Depth one first.

## Notes

`Assign`, `TakeAssignment` and `FinishAssignment` already exist with their lifecycle, and
`AssignmentPosted` already fans out to two bindings. The caps apply per goal today and would need
to apply per agent.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/swarm-server` — cited; the runtime is what must run an agent as its own metaharness session.
- **Files (cited):** `coordinator.rs` (the only thing that starts a session), `trigger.rs` (holds `COORDINATOR` and iterates goals only), `budget.rs` (caps are per goal), `swarm.rs` (`ensure_coordinator` hardcodes the role and the one mailbox; the spend row has no agent field), `src/core/domains/agent.yaml` (`Role` declares one variant).
- **Files (inferred):** `state.rs` (`in_flight` claims are keyed on the goal, so two agents on one goal serialise on one claim), the transcript surface in the web app.
- **Confidence:** high. The story names the enum, the caps and the runtime gap, and each was found.
- **About a third of `coordinator.rs` is coordinator-specific, and it is the load-bearing third.** Generic already: usage absorption, the launch, the streaming and turn-file recording, the verdict parser, the mail section, and the settings writer, which already takes an agent. Not reusable: the prompt, which opens "you are the coordinator" and closes on a verdict; the `Asked` shape, which carries a goal rather than an assignment; the four `SWARM_COORDINATOR_*` knobs; the tail, which issues `swarm.goal.Evaluate`; and `turn_file()`, whose name carries no agent — so two agents on one goal would overwrite each other's transcript, contradicting the acceptance directly.
- **This story opens a hole in the caps.** `Caps::exceeded` is called only with `spend_on(goal_id)`, and the spend row has no agent field. A worker's spend is either filed under a goal, where it eats the coordinator's budget and inflates the attempt count that trips the turn cap, or it is not filed and is unbounded. Making the caps per-agent needs an agent key in the spend row, an agent-filtered fold, and a claim keyed on the unit of work.

---
format: aep.planning-md/1
id: story:spawn-a-second-agent
kind: story
status: implemented
title: Spawn a second agent that actually runs
summary: A role other than Coordinator, and a spawned agent the runtime runs.
relations:
- decomposes: epic:executable-boxes
- depends_on: story:a-turn-is-confined-by-a-frame
- serves: vision:swarm-builds-itself
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
revision: 17
---
## What

`swarm.agent.Role` declares exactly one variant, `Coordinator`, so a coordinator that spawns a
second agent must call it a coordinator. And a spawned agent is a record: nothing runs it, so the
roster a swarm builds cannot do any work.

## Acceptance

Five clauses, each independently checkable. The story's own Scope section names a live risk against
clause 5 — `turn_file()`'s name carries no agent, so two agents on one goal would overwrite each
other's transcript — and under one chained sentence that risk could only register as a vote on the
whole. Numbered, it registers as clause 5.

1. **A role that is not `Coordinator` exists.** `swarm.agent.Role` declares a second variant and
   `ess specify validate --path src/core` passes.
2. **A spawned agent with that role gets its own mailbox**, distinct from the coordinator's, and
   `MailboxOpened` names it.
3. **The runtime runs that agent** as its own metaharness session against an assignment —
   `AssignmentTaken` fires with that agent as its actor, which has never happened in this repository.
4. **Two agents in one swarm work at the same time.** The in-flight claim is keyed on the unit of
   work rather than on the goal, so two agents on one goal do not serialise on one claim.
5. **Each agent's transcript is recorded separately.** `turn_file()` carries the agent, and two turns
   by different agents on one goal produce two files, neither overwriting the other.

Clause 4 is the one the 2026-09-12d wave did **not** pay for. That wave delivered the agent key on
the spend row and the agent-filtered fold, so the caps are ready; the claim is still keyed on the
goal.

## Out of scope

Agents spawning agents. Depth one first.

## Notes

`Assign`, `TakeAssignment` and `FinishAssignment` already exist with their lifecycle, and
`AssignmentPosted` already fans out to two bindings.

**The caps prerequisite this story named is now met.** It read: *"The caps apply per goal today and
would need to apply per agent."* The 2026-09-12c wave made the agent mandatory on a spend row
(`record_spend` takes it, and the unattributed door is gone), and the 2026-09-12d wave added
`Swarm::spend_by_agent` and applied `Caps` to it in `trigger::bounded`. So the agent key in the spend
row and the agent-filtered fold both exist. What this story still owes the caps is the third item of
that list: **a claim keyed on the unit of work rather than on the goal**, because
`state.rs`'s in-flight claims are keyed on the goal and two agents on one goal would serialise on one
claim.

**Measured 2026-09-13, across all 11 logs under `data/swarms/*/eventlog.sqlite3`:** the swarm
`mail-check-1789197540` spawned four agents — `coordinator`, `reviewer`, `builder`, `tester` — and
**every one of them carries role `Coordinator`**, which is this story's first sentence observed in a
real event log rather than argued from the enum. `AssignmentPosted` has fired once in the history of
this repository, `AssignmentRecorded` once, and `AssignmentTaken`, `StepDone`, `GateGreen`,
`GateRed`, `FinishAssignment`, `AgentBlocked` and `DecisionParked` have fired **zero times, ever**.
The working half of `agent.yaml` has never run.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/runtime/swarm-server` — cited; the runtime is what must run an agent as its own metaharness session.
- **Files (cited):** `coordinator.rs` (the only thing that starts a session), `trigger.rs` (holds `COORDINATOR` and iterates goals only), `budget.rs` (caps are per goal), `swarm.rs` (`ensure_coordinator` hardcodes the role and the one mailbox; the spend row has no agent field), `src/core/domains/agent.yaml` (`Role` declares one variant).
- **Files (inferred):** `state.rs` (`in_flight` claims are keyed on the goal, so two agents on one goal serialise on one claim), the transcript surface in the web app.
- **Confidence:** high. The story names the enum, the caps and the runtime gap, and each was found.
- **About a third of `coordinator.rs` is coordinator-specific, and it is the load-bearing third.** Generic already: usage absorption, the launch, the streaming and turn-file recording, the verdict parser, the mail section, and the settings writer, which already takes an agent. Not reusable: the prompt, which opens "you are the coordinator" and closes on a verdict; the `Asked` shape, which carries a goal rather than an assignment; the four `SWARM_COORDINATOR_*` knobs; the tail, which issues `swarm.goal.Evaluate`; and `turn_file()`, whose name carries no agent — so two agents on one goal would overwrite each other's transcript, contradicting the acceptance directly.
- **This story opens a hole in the caps.** `Caps::exceeded` is called only with `spend_on(goal_id)`, and the spend row has no agent field. A worker's spend is either filed under a goal, where it eats the coordinator's budget and inflates the attempt count that trips the turn cap, or it is not filed and is unbounded. Making the caps per-agent needs an agent key in the spend row, an agent-filtered fold, and a claim keyed on the unit of work.

## Collides with

**This story and `story:a-turn-is-confined-by-a-frame` both edit `metaharness_argv`**
(`src/runtime/swarm-server/src/coordinator.rs:544-580`): this one restructures the four
`SWARM_COORDINATOR_*` knobs around an agent, that one changes the `"--decisions", "observe"` literal
three lines above them. Thirty-five lines of one function, so there is no split by symbol.
**Sequence them, that story first** — rebasing a one-literal change onto a restructure is cheaper
than the reverse.

The `src/core/domains/agent.yaml` change this story makes is also what feeds
`website/src/data/spec-facts.json`, which `npm run spec:check` fails on when stale. That file is
never hand-edited and is nobody's surface; it is regenerated by `website/scripts/spec-facts.mjs`, and
step 11 of the gate is what catches it.

---
format: aep.planning-md/1
id: epic:executable-boxes
kind: epic
status: draft
title: Boxes that do something
summary: A drawn box runs, renders or reaches a service, and an agent can create one.
relations:
- serves: vision:swarm-builds-itself
revision: 2
---
# Epic: Boxes that do something

## Outcome

A drawn box runs, renders or reaches a service, and an agent can create one. Today every box is a
declaration: a Tool box executes nothing, a Service box reaches nothing, an agent the coordinator
spawns is a record that never runs. The change this delivers is that **the swarm's coordinator can
extend the swarm itself and a person watching can see it happen and stop it** — which is
`vision:swarm-builds-itself` narrowed to the one capability everything else waits on.

Anybody would know it had happened by reading an event log: `AssignmentTaken`, `StepDone`,
`GateGreen` and `FinishAssignment` have fired **zero times** in this repository's history, across all
11 swarm logs, measured 2026-09-13. The day any of them fires from work a coordinator handed to
another agent, this epic has delivered something.

## Why Now

The operator's standing goal, 2026-09-13, is **"autonomous, sandboxed multi-agent swarm proven"** —
recorded here verbatim because until now it existed in no artifact, which the scope critic of
round 1 named as a defect: a set cannot be judged against a promise nobody wrote down.

Three of those four words are open, and one is not:

| word | state, measured 2026-09-13 | where it is owned |
|---|---|---|
| **autonomous** | **already demonstrated for one agent** — 109 `GoalPursued` / 106 `GoalNotReached`; `dsfsdf` ran 38 unattended turns of real `claude-opus-5` sessions thirty seconds apart | nothing owns it; it works |
| **multi-agent** | **never happened.** `mail-check-1789197540` spawned `reviewer`, `builder` and `tester` and all three carry role `Coordinator`, because `swarm.agent.Role` declares one variant | `story:spawn-a-second-agent` |
| **sandboxed** | **not attempted.** Turns launch `--decisions observe`, which metaharness's own help calls "allow every call and record every call" | `story:a-turn-is-confined-by-a-frame` |
| **proven** | **partly** — 3 `GoalReached`, one by a real turn of 16 tool calls costing $0.44 | `story:a-recorded-run-shows-two-agents-working` |

What makes it now rather than later: the coordinator's default became `Launch::Metaharness` on
2026-09-12, so a server started with no environment spends real money on every `Pursuing` goal every
thirty seconds, with the whole native tool surface and nothing narrowing it. The 2026-09-12d wave
bounded the spending. Nothing bounds the reach.

## Scope

Three capabilities, in the order their dependencies allow:

1. **A role that is not `Coordinator`, and a runtime that runs it.** The agent domain's working half
   — `Assign`, `TakeAssignment`, `FinishAssignment` — exists in the specification and has never
   executed.
2. **A turn narrowed by a frame it cannot widen.** `--frame` with `--decisions frame`, admitted
   operations and a subject scope sealed into a digest.
3. **A demonstration anybody can re-run**, with a checker that reads the log and reports each clause
   by number.

Surfaces: `src/core/domains/agent.yaml` and `config.yaml`; `swarm-server`'s `coordinator.rs`,
`trigger.rs`, `swarm.rs`, `state.rs`; `examples/`; and the published prose in `README.md` and
`AGENTS.md`.

## Out of Scope

- **Kernel confinement.** `--substrate`, `--substrate-embedded` and `--cgroup-root` are **`b10x`
  only** and metaharness refuses them by name for the `claude` arm. `AGENTS.md`'s rule — *"Nothing
  confines a coordinator"* — is **not** retired by this epic, and no artifact under it may describe
  its outcome as containment in the kernel sense.
- **Agents spawning agents**, and more than two agents. Depth one, breadth two.
- **A concurrency ceiling.** `turns_in_flight()` refuses nothing; that is a different bound and a
  different decision, and no story here claims it.
- **The UI box library and the Service box**, which belong to this epic's name but not to this goal.

## Risks

- **A demonstration costs real money** — two concurrent `claude-opus-5` sessions. `dsfsdf` spent 38
  turns discovering its goal was the placeholder string `2342342`.
- **`metaharness_argv` is thirty-five lines that two stories both need**, so two of the three cannot
  be worked at once. Both bodies now say so.
- **The frame is metaharness's document, not ours.** Its format, digest rule and refusals are pinned
  to metaharness 0.7.0; a version bump can invalidate a frame the runtime writes.

## Ambiguities

- Whether the subject scope is a per-swarm config field or a constant the runtime derives from the
  swarm's work directory. The second cannot be widened by a config a coordinator writes for itself,
  which is an argument for it. Nobody has decided; marked `UNMAPPED:` in
  `story:a-turn-is-confined-by-a-frame`.

## Done When

`story:spawn-a-second-agent`, `story:a-turn-is-confined-by-a-frame` and
`story:a-recorded-run-shows-two-agents-working` are implemented, and a `verification-report` records
a run against the third's six numbered clauses — including the ones it failed.

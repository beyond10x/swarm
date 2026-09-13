---
format: aep.planning-md/1
id: epic:executable-boxes
kind: epic
status: draft
title: Boxes that do something
summary: A drawn box runs, renders or reaches a service, and an agent can create one.
relations:
- serves: vision:swarm-builds-itself
revision: 3
---
# Epic: Boxes that do something

## Outcome

A drawn box runs, renders or reaches a service, and an agent can create one. The coordinator can extend the swarm itself and a person watching can see it happen and stop it, serving `vision:swarm-builds-itself`.

## Current evidence — 2026-09-13

The historical draft predated the first two-agent run. Its statements that workers never ran, AssignmentTaken never fired, and turns used decisions observe are superseded by the evidence below; the CLI journal retains the earlier body.

- `story:spawn-a-second-agent`, `story:a-turn-is-confined-by-a-frame`, and `story:a-recorded-run-shows-two-agents-working` are implemented in v0.2.0. A coordinator and one Worker completed one assignment under sealed frames. `verification-report:two-agents-under-a-frame-2026-09-13` records seven met clauses; exported evidence is under `examples/two-agents/evidence/2026-09-13-two-agents-proof/`.
- The published attribution follow-up at `5331fe8` records claimed issuer identity beside actor type. The story remains draft because AGENTS.md reserves status moves for the operator; implementation evidence is already attached. Do not schedule that implementation again based on the status alone.
- The recorded demonstration predates issuer envelopes. It cannot prove the unattended condition, and its seven-clause success must not be reported as proof of unattended operation.
- Frame refusal is demonstrated; operating-system containment is not. Multi-agent evidence remains depth one and breadth two. Tool and Service execution remain separate open capabilities.

## Why now

The operator's recorded goal was "autonomous, sandboxed multi-agent swarm proven". Current evidence supports a narrower statement: two agents ran, a frame refused a call, usage was attributed, and a budget guard refused another turn. A fresh demonstration with issuers would be needed to establish unattended operation; containment requires a different host capability.

## Scope

The demonstration slice of this epic consisted of a non-Coordinator role and worker runtime, a sealed turn frame, and a replayable evidence checker. Existing nouns are declared in `src/core/domains/agent.yaml`, `manager.yaml`, `goal.yaml` and `config.yaml`.

`story:run-a-tool-box` and `story:connectors-as-service-boxes` still own the executable-box capabilities beyond that demonstration. Immediate control and evidence maintenance is now decomposed separately under `epic:trustworthy-swarm-control`: runtime pause/stop, the Rust checker port, and visible claimed issuers.

## Out of Scope

- Operating-system containment: the current metaharness Claude arm refuses the substrate/cgroup controls; the tool decision seam is not a sandbox boundary.
- Agents spawning agents and evidence beyond two agents; neither is established by the retained run.
- New aggregate spend policy or authorization rules; these require their own acceptance and review.

## Remaining verification

A fresh run using issuer-aware envelopes must earn an unattended result rather than inherit one from the old log. The existing verification report remains the evidence for the released demonstration; do not overwrite it to claim a later run.

## Done When

The three demonstration stories named above have implementation evidence and the verification report records every clause, including failures and limitations. This describes the delivered demonstration slice, not completion of every executable-box capability or the broad containment goal. No lifecycle move is made by this planning cleanup.

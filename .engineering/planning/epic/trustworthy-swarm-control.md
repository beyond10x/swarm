---
format: aep.planning-md/1
id: epic:trustworthy-swarm-control
kind: epic
status: draft
title: Control and verify a running swarm
relations:
- serves: vision:swarm-builds-itself
revision: 2
---
## Outcome

A person can halt runtime-owned swarm work, verify recorded work with the project's Rust toolchain, and distinguish the claimed issuer of commands in the canvas.

## Context

This is a bounded follow-up to the published attribution wave at `5331fe8`, not a declaration that the original autonomous, sandboxed swarm goal is complete. The previous session left the Rust checker port and canvas issuer display open in `.engineering/waves/2026-09-13b-an-event-says-who-acted.md:157`. The project review also found that assignment dispatch ignores the swarm lifecycle (`src/runtime/swarm-server/src/trigger.rs:215`) and child execution has no cancellation path (`coordinator.rs:1145`).

## Scope

Three stories own the complete outcome: `story:pause-and-stop-control-runtime-work`, `story:verify-two-agent-evidence-in-rust`, and `story:show-command-issuers-in-the-canvas`. The next proposed wave selects the first two. Canvas work waits because its live issuer field changes `swarm-server/src/swarm.rs`, which the control story also changes.

Existing domain nouns are declared in `src/core/domains/manager.yaml` (Swarm), `agent.yaml` (Agent and Assignment), and `goal.yaml` (Goal). This epic introduces no kernel entity or persistence format. Processes, cancellation, and event-envelope presentation remain host responsibilities.

## Out of Scope

Authentication, operating-system containment, new agents or Tool/Service execution, changing the seven demonstration clauses, a fresh paid demonstration, and a new release. The old run remains limited evidence: depth one, breadth two, with refusal at the tool decision seam and no proven unattended condition. Existing backlog items remain open independently of this epic.

## Acceptance

A reviewer can reproduce all three child-story acceptance results from one integration checkout using the commands recorded in its verification report.

## Verification requirements

The report cites each child's regression evidence and the full repository gate with separate exit statuses. Its explanation distinguishes claimed identity and process cancellation from authentication and containment. These are requirements on the one reproducible verification result, not separate untracked deliverables.

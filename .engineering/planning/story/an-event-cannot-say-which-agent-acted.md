---
format: aep.planning-md/1
id: story:an-event-cannot-say-which-agent-acted
kind: story
status: draft
title: An event cannot say which agent acted
summary: actor is an actor TYPE, so AssignmentTaken by the builder and by anyone else are the same record.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: examples/two-agents/check-two-agents.py
- confidence: cited
  path: src/core/domains/agent.yaml
- confidence: cited
  path: src/runtime/ess-runtime/src/apply.rs
- confidence: cited
  path: src/runtime/ess-runtime/src/store.rs
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
revision: 6
---
## What

An event's `actor` column cannot say **which agent** acted. `apply.rs:174-192` (`permitted`) matches
the actor against the specification's *actor types*, not agent instances, so every command issued as
`swarm.agent.SwarmAgent` records that string whoever sent it. `agent.yaml:434` makes
`TakeAssignment`'s `agent_id` **caller-supplied input**, and `http.rs:38` makes the request body's
`actor` optional, defaulting to `system` (`store.rs:192`, `actor.unwrap_or("system")`).

So `swarm.agent.AssignmentTaken` with `agent_id: builder` is **indistinguishable in the record**
from the builder taking it, whoever actually issued it — the coordinator, an operator's curl, anyone
who can reach `POST /swarms/{slug}/commands/{command}`.

Measured 2026-09-13 by the adversary of unit B of the 2026-09-13a wave
(`review-result:adversary-2026-09-13a-unit-b-pass-1`, finding 6, `CONFIRMED`, `origin:
pre-existing`), while building the checker for
`story:a-recorded-run-shows-two-agents-working`.

## Why it matters more than it looks

It is the difference between a swarm that **demonstrates** multi-agent operation and a log that
**asserts** it. The whole point of the demonstration story is that a claim in the README is worth
nothing and a claim in an event log is worth something; this is the seam where that stops being true.
A single-agent swarm can write a log that a reader, and a checker, cannot tell apart from a
two-agent one.

The same gap makes the demonstration's "unattended" condition unfalsifiable from the record: every
operator command and every machine command both record `system`.

## Acceptance

1. An event records **which agent instance** caused it, distinguishably from another agent of the
   same actor type.
2. A command issued by something that is not the named agent is either refused, or recorded in a way
   a reader can tell apart from the agent's own action. Which of the two is the story's first
   decision.
3. An operator command reaching the HTTP surface is distinguishable in the record from a command the
   runtime issued to itself — so "no operator touched this" becomes a checkable claim.
4. `examples/two-agents/check-two-agents.py`'s clause 2 and its unattended condition are tightened
   onto whatever 1–3 provide, and their weakened forms are removed.

## Out of scope

Authentication. This is about a record being able to *say* who acted, not about proving the claim
against a credential. The two are different, and the second is worth nothing without the first.

## Notes

`UNMAPPED:` whether the acting identity belongs on the command envelope (beside `actor`), on each
event's payload, or both. The specification's actor-type check is `permitted`'s and would need to
stay — this adds identity beside it rather than replacing it.

## Scope

- **Files:** `src/runtime/ess-runtime/src/apply.rs` (`permitted`, the actor-type match) — cited
- **Files:** `src/runtime/ess-runtime/src/store.rs:192` (`actor.unwrap_or("system")`) — cited
- **Files:** `src/runtime/swarm-server/src/http.rs:38` (the optional `actor` field) — cited
- **Files:** `src/core/domains/agent.yaml` (`TakeAssignment`'s caller-supplied `agent_id`) — cited
- **Also likely:** `examples/two-agents/check-two-agents.py` — inferred, acceptance clause 4
- **Confidence:** high — every path was read by the adversary and the indistinguishability was
  measured, not argued
- **Would collide with:** anything touching `apply.rs`, `store.rs` or `http.rs`

---
format: aep.planning-md/1
id: story:two-workers-under-real-model-turns
kind: story
status: draft
title: Two workers at once under paid model turns, which is unmeasured
summary: The two-worker run used Launch::Program test doubles; no vendor was contacted and nothing was spent.
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
- depends_on: story:run-two-workers-at-once
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: examples/two-workers
revision: 2
---
## What

`story:run-two-workers-at-once` demonstrates two turns of one swarm in flight together, and it does
so against `Launch::Program` test doubles: sessions that satisfy the same stdin/stdout contract as
`examples/coordinator-manual.sh`. **No vendor was contacted and nothing was spent in that run.**

This story is the gap that leaves, written down rather than folded into the result it qualifies.
The distinction is not pedantry: what a real `metaharness run claude` does under a frame is
measured only in `examples/two-agents/evidence/2026-09-13-two-agents-proof/`, for $0.224, and that
run took **one turn at a time**. Concurrency plus a paid harness is a combination nothing in this
repository has yet run.

What is specifically unmeasured:

- whether two `metaharness` sessions under sealed frames interleave cleanly, on one machine, with
  two real transcripts and two real spend rows;
- what `SWARM_MAX_TOTAL_SPEND_USD` costs to demonstrate honestly — the cap has to actually fire
  against dollars a vendor charged, or the demonstration is the double again;
- whether the per-turn price a concurrent swarm reaches matches what the per-agent caps assume,
  which is the number the default `IN_FLIGHT * SPEND_USD` derivation rests on.

## Acceptance

A recorded run in which two members of one swarm take paid `metaharness` turns at the same moment,
retained as evidence in the repository the way the 2026-09-13 run is, with both transcripts, both
spend rows, the dollars actually charged, and a swarm-wide cap that fires against them. Until then
`README.md` and `AGENTS.md` must keep saying that the two-worker demonstration ran against a
program launch.

---
format: aep.planning-md/1
id: story:run-two-workers-at-once
kind: story
status: implemented
title: Two members of one swarm take their turns at once, under a ceiling
summary: Breadth, plus the swarm-wide bound on the exposure breadth creates.
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
- depends_on: story:spend-is-bounded-per-goal-only
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: examples/two-workers/evidence/2026-09-18-two-workers-at-once/README.md
- confidence: cited
  path: src/runtime/swarm-server/src/budget.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_ceiling_under_attack.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_second_agent_under_attack_pass_2.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/two_workers_run_at_once.rs
revision: 5
---
## What

A swarm ran one turn at a time, and every cap it had was per-goal or per-agent
(`story:spend-is-bounded-per-goal-only` closed the per-agent half). Two things change together,
and the second is the reason the first is safe to ship: breadth multiplies exposure, so a swarm
whose members are each inside their own bounds is a swarm nothing is bounding.

- `SWARM_MAX_IN_FLIGHT` (default 4) ceilings how many turns of one swarm may be in flight over
  every agent in it. `Server::claim_within` takes the claim and the count in one pass, so two
  arrows of the same period cannot both read "room for one more".
- `SWARM_MAX_TOTAL_SPEND_USD` (default 20.00 = breadth times the per-agent cap) folds the whole
  swarm's record. `Bound::Swarm` is the third subject a cap is read off, and `capped.jsonl` rows
  carry `"bound":"swarm"`.

**Being full is not a refusal.** A turn the ceiling holds back costs nothing, publishes nothing,
writes no `capped.jsonl` row, and runs at the next period. Only a cap reached refuses. The two are
different events and the runtime does not conflate them.

## What the recorded run demonstrates

`examples/two-workers/evidence/2026-09-18-two-workers-at-once/`, read three independent ways,
because two turns that both happened is not two turns that happened together:

1. the runtime's own count — `Server::turns_in_flight_for("two-workers")` is **2** at the instant
   `work_the_assignments` returns, both turns claimed and neither finished;
2. the sessions' clocks — two intervals starting **28 µs** apart and overlapping for **206 ms**;
   a runtime that ran them one after the other produces intervals that do not intersect;
3. the members' own verdicts — each session waits for a second to register before it answers, so
   both report `two turns were in flight at once`, both assignments reach `Done`, and
   `AssignmentTaken` fires once per member.

Four mutation proofs are named in that README; each turns a case red.

## What it does NOT demonstrate

Every session in that run is a `Launch::Program` — the stdin/stdout contract
`examples/coordinator-manual.sh` satisfies. **No vendor was contacted and nothing was spent.** So
what is demonstrated is the runtime's admission path under concurrency and the ceilings over it,
and **not** two paid model turns at once. That gap is carried by
`story:two-workers-under-real-model-turns` and is not closed here.

## Acceptance

Two members of one swarm have their turns in flight at the same moment under the runtime's own
admission path, measured three independent ways; `SWARM_MAX_IN_FLIGHT` holds a swarm at its
ceiling without refusing, publishing or writing a capped row; `SWARM_MAX_TOTAL_SPEND_USD` refuses a
swarm whose members are each still inside their own bounds and writes a `"bound":"swarm"` row; and
one swarm's ceiling is not another swarm's business.

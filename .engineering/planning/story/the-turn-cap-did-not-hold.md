---
format: aep.planning-md/1
id: story:the-turn-cap-did-not-hold
kind: story
status: draft
title: The turn cap did not hold, twice
summary: 78 turns against a declared 20, and $11.35; the fold can only key on goal.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/budget.rs
- confidence: inferred
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
revision: 3
---
## What

`SWARM_MAX_TURNS` defaults to 20 (`src/runtime/swarm-server/src/budget.rs:29`). The swarm `dsfsdf`
recorded **78 turns and $11.35** against one goal and is still `Evaluating`. `budget.rs:5-6` already
records an earlier instance of the same thing — 78 turns, $11.10, on a goal whose text was `2342342`
— which is why the module exists at all. It happened again after the module existed.

So the first question is not how to bound N agents. It is **why the bound did not hold once**:
whether the cap fired late, was lifted by an environment variable, or is never consulted on the path
that ran. `Caps::exceeded` is called from exactly one place, `capped()` at `trigger.rs:290-295`,
folding `swarm.spend_on(goal_id)` — which is `spend_where(row.goal == goal_id)` (`swarm.rs:450-452`).
Turns are measured on `iterations.max(attempts)` for a recorded reason (`budget.rs:176-198`).

**The second half is what makes this urgent.** The fold can only key on `goal`. With N agents working
one goal, either every agent files spend under that goal — N workers sharing one $5 budget, each
attempt inflating the count that trips the 20-turn cap, so the swarm exhausts in 20/N rounds — or
they file nothing and **their spend is unbounded**. There is no third option in the current fold, and
there is no global ceiling anywhere: `turns_in_flight()` (`state.rs:201-203`) only reports the number
to the UI; nothing reads it to refuse.

## Acceptance

A goal with `SWARM_MAX_TURNS=3` stops at three turns, proved by a test rather than by reading the
code. The reason the bound failed for `dsfsdf` is written down, with the measurement that shows it.
Spend and turns are attributable per agent as well as per goal, and a ceiling exists that does not
depend on which goal an agent is working.

## Out of scope

Making the caps configurable per swarm. The environment is the right place for them until a swarm's
config is read at all — see `story:coordinator-from-config`.

## Notes

`dsfsdf` is the evidence and should not be deleted while this is open: 5.7M, 44 spend rows all under
goal `77fc1fcc-a89c-4fb9-b041-89ecfc75922f`, 45 turn files, a 4.1M unchecked WAL.

Per-turn ceilings inside metaharness (`--max-turns 30`, `--max-budget-usd 1.00`,
`coordinator.rs:376-379`) are a different bound and did not stop this either.

## Scope

- **Files:** `src/runtime/swarm-server/src/budget.rs` (`Caps`, the defaults, `exceeded`) — cited
- **Files:** `src/runtime/swarm-server/src/trigger.rs:290` (`capped`, the only caller) — cited
- **Files:** `src/runtime/swarm-server/src/swarm.rs:411-452` (`record_spend`, `spend_where`, `spend_on`) — cited
- **Also likely:** `src/runtime/swarm-server/src/state.rs` (`turns_in_flight`, a ceiling that refuses) — inferred
- **Confidence:** high on the measurement, which is on disk; the cause is **not established** and finding it is the first half of this story
- **Would collide with:** any unit touching `trigger.rs`, `swarm.rs` or `state.rs`

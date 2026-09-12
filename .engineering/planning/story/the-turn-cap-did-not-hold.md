---
format: aep.planning-md/1
id: story:the-turn-cap-did-not-hold
kind: story
status: implemented
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
revision: 8
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

## What the investigation found

**Corrected 2026-09-12, after the investigation. The premise of this story was wrong.**

It happened **once, not twice**, and the cap did not fail — it did not exist.

| measurement | value |
|---|---|
| `data/swarms/dsfsdf/turns/spend.jsonl`, all 44 rows | **$11.345391** |
| the same file's first 43 rows | **$11.098908** |
| `993731e`, which added `budget.rs` and both call sites | `2026-09-12T07:15:35Z` |
| `dsfsdf`'s last turn | `2026-09-12T01:29:33Z` — **5h46m earlier** |

The $11.35 and the $11.10 recorded in `budget.rs:5-6` are **one file read one turn apart**, not two
incidents. `git log -S'fn capped'` and `-S'if capped(server, swarm, id, iterations)'` each return
that single commit, so the module and both of its call sites arrived together, after the run. The
overrun was not the bound failing late and was not the bound being lifted: there was no bound in the
binary that ran. The spend file starting at iteration 35 has the same cause — the earlier binary
recorded nothing at all.

**The bound holds when it exists.** `trigger::bounds` drives the real `capped` with
`SWARM_MAX_TURNS=3` and stops at three, both for a goal that answers and for a goal that never
writes a verdict. Both cases passed on their first compiling run with **no change to the capping
logic**.

What was genuinely missing is attribution, and that is what the unit fixed: `record_spend` wrote no
agent field, so a fold over more than one agent on one goal could not say whose spend it was.

Two things are still open and neither is this unit's file:

- `coordinator.rs:609` still calls the unattributed door, so an **answered** turn's row carries
  `"agent": null`. The patch exists, unapplied, and belongs with `story:coordinator-from-config`.
- The global in-flight ceiling in `state.rs` — the acceptance's fourth clause — belongs to the
  multi-agent wave. `turns_in_flight()` is confirmed read-only to the UI; nothing refuses on it.

One correction to this story's own scope section: `capped` has **two** callers, not one — `fire` at
`trigger.rs:239` and `ask_the_coordinator` at `trigger.rs:137`. `Caps::exceeded` has one.

## A second defect, found by driving the real guards

`SWARM_MAX_TURNS=3` ran **two** turns, not three, and no reading found it — only driving the real
guards did.

The bound is checked in two places and they measure different numbers. `fire` (`trigger.rs:246`)
checks `so_far`, the turns already taken. `ask_the_coordinator` (`trigger.rs:137`) checks the goal's
`iterations` field, which `fire` has **already advanced** for the turn about to be taken. One bound,
two readings, off by one, stopping the loop a turn early.

```
panicked at src/runtime/swarm-server/src/trigger.rs:439:9:
assertion `left == right` failed: a declared cap of three turns stops at three
  left: 2
 right: 3
```

Found 2026-09-12 by unit A of the 2026-09-12c wave, in the correction round that answered the
adversary's finding that the tests re-implemented the loop instead of driving it. The first version
of those cases called `capped` directly and could not see this; both guards are now executed by the
suite and both are mutation-checked — delete either and a case goes red.

Fixed at `trigger.rs:137` with `iterations.saturating_sub(1)` and the reason written above it.

**The lesson is the one the adversary's finding named.** A case that re-implements the path it tests
proves something about the copy. Two guards agreeing with a test that called neither of them is how
a cap of three stopped at two for as long as both existed.

---
format: aep.planning-md/1
id: story:spend-is-bounded-per-goal-only
kind: story
status: draft
title: An agent is bounded per goal, not at all
summary: $6.00 of a $5.00 cap over two goals is refused nowhere, and no global ceiling exists.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/budget.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/src/trigger.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
revision: 3
---
## What

One agent spending across several goals is bounded by nothing. `capped` folds
`spend_on(goal_id)` = `spend_where(row.goal == goal_id)` (`swarm.rs:450-452`), so the fold can only
key on a goal. Measured: an agent that spent **$6.00 of a $5.00 cap over 6 turns**, spread across two
goals, is refused nowhere.

There is no global ceiling either. `turns_in_flight()` (`state.rs:201-203`) only reports its number
to the UI; nothing reads it to refuse.

This is the fourth acceptance clause of `story:the-turn-cap-did-not-hold`, carried here because that
story was moved to `implemented` while this clause was still open — a coordinator error, recorded
rather than hidden. Everything else in that story is done and gated.

## It is pinned, not forgotten

`tests/the_turn_cap_under_attack.rs` holds
`an_agents_spend_is_not_bounded_across_the_goals_it_works`, which asserts today's defect on purpose:
`turns == 6`, `spent == 6.00`, refused nowhere. Its failure message carries this story's id, the file
the ceiling belongs in, and the two edits that undo the pin. It is mutation-checked — halve the cap
and it fails — so it cannot pass because nothing spent.

**It goes red the moment the ceiling lands**, which is the signal to flip it back.

## Why it matters more now than it did this morning

The coordinator's default is `Metaharness` as of 2026-09-12, so a server started with no environment
spends real money on every Pursuing goal every thirty seconds. The per-goal caps hold — that is
measured, and the off-by-one that made a cap of three stop at two is fixed — but they hold per goal.
A swarm with several goals, or a swarm with several agents once
`story:spawn-a-second-agent` lands, is bounded by the number of goals it has rather than by anything
an operator chose.

## Acceptance

An agent's spend and turns are bounded whichever goals it spreads them over, and a ceiling exists
that does not depend on which goal an agent is working. The pinned case above is flipped back to
`an_agents_spend_is_bounded_across_the_goals_it_works` and is green.

## Scope

- **Files:** `src/runtime/swarm-server/src/state.rs` (`turns_in_flight`, a ceiling that refuses) — cited
- **Files:** `src/runtime/swarm-server/src/budget.rs` (`Caps`, what a cap is keyed on) — cited
- **Files:** `src/runtime/swarm-server/src/trigger.rs:297` (`capped`, the fold) — cited
- **Files:** `src/runtime/swarm-server/src/swarm.rs:450` (`spend_where`, which can only key on goal) — cited
- **Also likely:** `src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs` — inferred, it holds the pin
- **Confidence:** high — every path was read, and the $6.00-over-two-goals measurement is on record in `review-result:adversary-2026-09-12c-unit-a-pass-1`
- **Would collide with:** any unit touching `state.rs`, `trigger.rs`, `budget.rs` or `swarm.rs`

---
format: aep.planning-md/1
id: story:spend-is-bounded-per-goal-only
kind: story
status: implemented
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
- confidence: cited
  path: src/runtime/swarm-server/tests/the_ceiling_under_attack.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
- confidence: cited
  path: src/web/src/runtime.ts
revision: 9
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

Derived 2026-09-12, **corrected 2026-09-13 from unit A's confirmation table** after the wave of
2026-09-12d implemented it. Corrections are shown, not deleted.

- **`src/runtime/swarm-server/src/swarm.rs`** — cited, confirmed. `spend_where` could only key on a
  goal; `spend_by_agent` is the fold that keys on the agent.
- **`src/runtime/swarm-server/src/trigger.rs`** — cited, confirmed, and the largest change (+298).
  `bounded()` returns `budget::Capped`; `capped()` is the thin verdict wrapper; `report_capped`
  publishes the firing bound's own figures.
- **`src/runtime/swarm-server/src/budget.rs`** — cited. The implementor's first round reported it as
  **not needing a change**; that was measured false by the adversary, and it carries the largest
  documentation change in the wave plus the new `Bound` and `Capped` types.
- **`src/runtime/swarm-server/src/state.rs`** — cited. Same: reported as not needing a change, then
  corrected in four places, two of which the adversary's finding had not listed (the `caps` field
  and `CappedGoal`). `turns_in_flight()` was **not** touched — it is a concurrency counter, not a
  spend fold, and a parallelism limit is a different bound.
- **`src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs`** — was `inferred`, **confirmed**.
  It held the pin, which is now inverted and green and keeps its per-goal fold asserting that fold
  still refuses nowhere.
- **`src/runtime/swarm-server/tests/the_ceiling_under_attack.rs`** — new, the adversary's file, 290
  lines, both cases rewritten on the coordinator's instruction and mutation-checked.
- **`src/web/src/runtime.ts`** — **not in the original scope at all.** The `Caps` interface there is
  a consumer contract in another package and said a cap is what one goal may use up. A story that
  changes what a cap means lands in two packages, and the scoper did not see it.
- **Confidence:** high — every path was read, and the $6.00-over-two-goals measurement is on record
  in `review-result:adversary-2026-09-12c-unit-a-pass-1`.
- **Would collide with:** any unit touching `state.rs`, `trigger.rs`, `budget.rs` or `swarm.rs`, and
  now also anything touching `src/web/src/runtime.ts`.

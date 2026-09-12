---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12c-unit-a-pass-1
kind: review-result
status: active
title: Adversary, wave c unit A, pass 1
relations:
- reviews: story:the-turn-cap-did-not-hold
revision: 1
---
```
unit: unit A, story:the-turn-cap-did-not-hold — worktree wave-20260912c-a at f0a521f (base 80b9d24)
verdict: NEEDS-CHANGE
cases: executed 33→36, red 3
origin: introduced 4 / pre-existing 2 / undecided 0
wrote-outside-worktree: none
needs-coordinator: whether case 3 (acceptance clause 4, no ceiling across goals) stays red or is re-characterised — out of this unit's scope per the brief
```

Recorded by the coordinator from the adversary's returned report. One change was forced on the
findings block: finding 1's message contains `"agent": null`, whose colon-space the store's YAML
reader rejects, so that message is single-quoted. No word is altered.

## 1. What was touched

```
?? src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
```

One test file. No implementation file touched, none mutated even briefly.

## 2. The three cases, red

**a. `an_answered_turn_names_the_agent_that_spent_it`** — drives the real `coordinator::take_a_turn`
with a program coordinator and reads the row the runtime wrote.

```
assertion `left == right` failed: every recorded turn names the agent that spent it, including one that answered: {"agent":null,"at":"2026-09-12T11:34:17.630839637Z","finished":true,"goal":"77fc1fcc-...","iterations":1,"note":"not yet","reached":false,...}
  left: None
 right: Some("coordinator")
```

**b. `a_negative_spend_cap_is_not_a_lifted_spend_cap`**

```
a mistyped spend cap is refused rather than silently read as no cap: got None
```

The first assertion of the same case passed: `SWARM_MAX_TURNS=-1` keeps `Some(20)`. Same input, two
caps, opposite outcomes — one keeps the default, the other removes the bound.

**c. `an_agents_spend_is_bounded_across_the_goals_it_works`**

```
an agent that has spent $6.00 of a $5.00 cap over 6 turns is refused somewhere, whichever goals it spread them over
```

## 3. The suite

`cargo test -p swarm-server --no-fail-fast`: 33 passed, 3 failed, 36 total; the three are the
adversary's own binary. `fmt --check` no output; `clippy --all-targets` clean with the file present.

## 4. Findings

| # | file:line | verdict / origin | what was measured | what reaches it |
|---|---|---|---|---|
| 1 | `coordinator.rs:610` | NEEDS-CHANGE / introduced | the answered-turn row is `"agent": null` | **every successful coordinator turn.** `one_turn` records spend itself only on the *unfinished* branch; the answered branch records here. The unit's own case asserts the opposite over three rows it wrote through `record_spend_by`, so it cannot see this |
| 2 | `budget.rs:134` | CONFIRMED / pre-existing | `SWARM_MAX_SPEND_USD=-1` → cap lifted; `SWARM_MAX_TURNS=-1` → `Some(20)` | the environment, the only way either cap is set. `Caps::configured`'s own doc says a mistake is "worth refusing loudly rather than silently treating as no cap"; `-1` is neither `0` nor `off` and is silently no cap |
| 3 | `trigger.rs:297` | INFEASIBLE / pre-existing | 6 turns and $6.00 by one agent over two goals trips neither goal's cap | a swarm with more than one goal. The story's fourth acceptance clause, unmet; the ceiling is `state.rs`, which this unit may not touch. **The story must not close on this unit** |
| 4 | `budget.rs:13` | CONFIRMED / introduced | "every other swarm's record is one row" is false — `main` has 4, `e2e-1789212621` has 2 | a reader of the module doc. The conclusion survives: no other swarm exceeds 4 turns or $0.44 |
| 5 | `trigger.rs:361` | CONFIRMED / introduced | `bounds` calls `capped` directly and re-implements the loop; neither case's call graph includes `ask_the_coordinator` or `fire` | delete either guard and both cases stay green. The tests prove `capped` caps, not that the loop consults it |
| 6 | `trigger.rs:352` | CONFIRMED / introduced | the acceptance names `SWARM_MAX_TURNS=3`; the case constructs `Caps` directly | `Caps::configured` — the only reader of that variable — had no test caller. The `&Server`→`Caps` move is what took the environment out of the tested path, which is why finding 2 was invisible |

## 5. Attacked and not broken

- The `&Server` → `Caps` change: `Server::caps()` is a `Copy` snapshot taken once at `Server::start`, so the call is byte-identical in effect. No behaviour moved.
- `iterations.max(attempts)`: both call sites still pass the goal's own count; attribution did not narrow the fold.
- `Caps::exceeded` boundaries at 1, 3, 20, lifted, and an unpriced turn — all as documented.
- Double counting: an answered turn writes one row, an unfinished one writes one, no path writes both.
- The historical arithmetic and the `993731e` timing reproduced independently and are correct. No earlier bound existed under another name — `SWARM_COORDINATOR_MAX_TURNS` arrived at 07:11:32Z, also after the run.
- No other swarm on disk hides a missed overrun: row counts 1, 44, 2, 1, 4, 1; all totals under $0.45 except `dsfsdf`.

```findings
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 610
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'an answered turn still records through the unattributed door, so every successful turn writes "agent": null and the unit''s own assertion that "every recorded turn names the agent that spent it" is false of the dominant path.'
- file: src/runtime/swarm-server/src/budget.rs
  line: 134
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: SWARM_MAX_SPEND_USD=-1 silently lifts the spend cap although the doc says only 0 or off lift it, while the same typo in SWARM_MAX_TURNS keeps the default.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 297
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: the story's fourth acceptance clause is unmet — one agent spending $6.00 over two goals trips no cap, and the ceiling that would refuse lives in state.rs, which this unit may not touch.
- file: src/runtime/swarm-server/src/budget.rs
  line: 13
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the new module doc's "every other swarm's record is one row" is false — main has 4 rows and e2e-1789212621 has 2 — though the no-other-overrun conclusion it supports still holds.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 361
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the bounds cases call capped directly and never execute trigger.rs:137 or trigger.rs:246, so deleting either cap guard leaves them green.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 352
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance names SWARM_MAX_TURNS=3 but the case constructs Caps directly, leaving Caps::configured — the only reader of that variable — with no test caller in the suite.
```

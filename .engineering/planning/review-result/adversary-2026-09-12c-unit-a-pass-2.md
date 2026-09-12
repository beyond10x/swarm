---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12c-unit-a-pass-2
kind: review-result
status: active
title: Adversary, wave c unit A, pass 2
relations:
- reviews: story:the-turn-cap-did-not-hold
- reviews: story:coordinator-from-config
revision: 1
---
```
unit: unit A of the 2026-09-12c wave (branch wave/2026-09-12c/turn-cap, head 3793fdd; base 80b9d24)
verdict: NEEDS-CHANGE
cases: executed 43→46, red 3
origin: introduced 1 / pre-existing 2 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none
```

Recorded by the coordinator from the adversary's returned report, unedited.

## 1. What was touched

```
?? src/runtime/swarm-server/tests/the_turn_cap_under_attack_pass_2.rs
```

One test file. No implementation file touched; `state.rs` and `http.rs` untouched.

## 2. The cases

| case | asserts | now |
|---|---|---|
| `a_spend_cap_of_nan_is_not_a_cap` | `SWARM_MAX_SPEND_USD` set to `nan`/`NaN`/`inf` still refuses $1,000,000 on one goal | RED |
| `a_goal_stopped_by_the_silent_guard_is_still_reported_capped` | a goal the ask-side guard stopped appears in `capped_goals()` | RED |
| `resolve_reads_the_config_that_was_activated_last` | after a second `ActivateConfig`, exactly one config is Active and `resolve` answers the new one | RED |
| `a_cap_of_n_stops_at_exactly_n` | `SWARM_MAX_TURNS` = 1, 2, 5 stops at exactly that many attempts, through `trigger::run` | GREEN |

```
SWARM_MAX_SPEND_USD=nan is neither 0, off nor none, so it must not lift the spend cap: a million
dollars on one goal was refused by nothing, and max_spend_usd is Some(NaN)

a goal the loop stopped asking about is reported capped, whether the guard that stopped it was the
one in `fire` or the one at trigger.rs:137. This goal took 3 of a cap of 3 and then the loop went
quiet: capped_goals() holds [], so every reader the server has sees a goal in Pursuing and no
reason, which is the picture the $11.35 run presented

assertion `left == right` failed: config.yaml:10 says one config is Active per swarm at a time, and
ActivateConfig's own summary says the previous row is superseded in the same transaction. Activating
a replacement left these Active: ["/usr/local/bin/the-new-one", "/usr/local/bin/the-old-one"].
```

**The adversary's disclosure about its own third case:** its first form asserted only `resolve`'s
answer. Alone it was red; in the suite it was **green**. That flip is itself the finding — `.find()`
over a UUID-keyed instance map is a coin flip — so the case was rewritten to lead with the
deterministic invariant and drive eight swarms behind it.

## 3. The suite

`fmt --check` 0, `clippy` 0, `cargo test -p swarm-server` **exit 101**: 19, 0, 2, 2, 7, 3, 4, 2, 3
green and `the_turn_cap_under_attack_pass_2` 1 passed / 3 FAILED. Cargo short-circuits after the
failing target, so `open_does_not_queue_behind_an_unrelated_slug` was not reached; the count is 47
once these are fixed.

## 4. Findings

| # | file:line | verdict / origin | what was measured | what reaches it |
|---|---|---|---|---|
| 1 | `coordinator.rs:371` | NEEDS-CHANGE / introduced | `from_config` resolves with `.find(state == "Active")`; activating a replacement leaves **both** Active and `resolve` answered the replaced one | `DraftConfig` + `ActivateConfig` twice — the documented way to change a launch line. `config.yaml:10-13` records that the supersede cannot be done by a binding. Nothing read the config before this unit, so this unit is what reaches the gap. **The pick is by generated UUID order, so which coordinator a swarm launches is nondeterministic per swarm** |
| 2 | `trigger.rs:143` | CONFIRMED / pre-existing | the ask-side guard `continue`s in silence: no `report_capped`, no `What::Capped`, no warn. Cap of 3 held and `capped_goals()` was `[]` | the incident shape. A coordinator that never writes a verdict leaves the goal `Pursuing`, which `GoalsAwaitingATick` does not select, so `fire`'s reporting guard never runs again and this is the **only** guard that ever trips. The unit's own case builds that state and does not ask who was told |
| 3 | `budget.rs:163` | CONFIRMED / pre-existing | `read::<f64>` classifies with `== T::default()` then `< T::default()`; NaN satisfies neither, so `nan`/`NaN`/`inf` become the ceiling and `spent >= NaN` is false for every spend. No warning | `SWARM_MAX_SPEND_USD`, the only way this cap is set; any string reaches `read`. No workflow that generates `nan` was found — the finding is that the classification the module documents as closed is not closed, in the one direction that uncaps money |

Named fixes, not applied: (1) refuse, or pick by `activated_at`, when more than one row is Active —
and either implement the supersede or stop documenting it as done; (2) call `report_capped` /
`What::Capped` from the ask-side guard as `fire` does; (3) add `!value.is_finite()` to `refuse`.

## 5. Attacked and could not break

- The off-by-one at other cap values: 1, 2 and 5 each stop at exactly N attempts through the real `trigger::run`. `saturating_sub(1)` holds at the boundary. Left behind as a green case.
- `0`/`off`/`none`/`-1`/`-0.01`/`nonsense`/empty/whitespace on both caps — all as documented.
- `record_spend`'s closure: the only two callers both pass an actor; the signature makes an unattributed row unexpressible.
- `resolve` precedence: `binary` beats `harness`; `harness: Claude` with blank binary is Metaharness; blank `launch` falls through; `Codex` is refused by name and does not fall back.
- The pinned case asserts `turns == 6` and `spent == 6.00` **before** the pinned assertion, so it cannot pass because nothing spent. Mutation-sensitive as claimed.

## 6. Paths written outside the worktree

None.

```findings
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 371
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: from_config resolves with a find over state Active while ActivateConfig never supersedes the previous row, so a swarm whose operator replaced its config launches whichever of the two Active configs has the lower generated UUID.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 143
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the ask-side cap guard stops the loop without report_capped, What::Capped or a warning, and for a coordinator that never writes a verdict it is the only guard that ever trips, so a capped goal is indistinguishable to every reader from the overrun this module exists to prevent.
- file: src/runtime/swarm-server/src/budget.rs
  line: 163
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: read classifies against T::default() with equality and less-than, neither of which NaN satisfies, so SWARM_MAX_SPEND_USD=nan or inf is accepted as the ceiling and silently lifts the spend cap the module documents as liftable only by 0, off or none.
```

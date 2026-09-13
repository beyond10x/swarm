---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12d-unit-a-pass-1
kind: review-result
status: active
title: Adversary, wave d unit A, pass 1
relations:
- reviews: story:spend-is-bounded-per-goal-only
revision: 1
---
Adversary, wave 2026-09-12d unit A, pass 1. Against `story:spend-is-bounded-per-goal-only`, working
tree `/home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912d-a`, branch
`wave/2026-09-12d/unit-a`, uncommitted, base `9afa0b8`.

## Header, as returned

```
unit: story:spend-is-bounded-per-goal-only
verdict: NEEDS-CHANGE
cases: executed 69→71, red 2
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: 6 paths, all under the assigned scratch directory
needs-coordinator: yes — whether the caps' unit of account is now the swarm (docs change) or the goal (behaviour changes) is a story decision, not mine
```

## Cases added

Two, in `src/runtime/swarm-server/tests/the_ceiling_under_attack.rs`, the adversary's only written
path. Both use `trigger::COORDINATOR` and write rows through `Swarm::record_spend` — the agent the
runtime passes at both call sites. Neither constructs an agent name nothing uses. No existing case
edited, skipped or weakened.

| case | asserts | now |
|---|---|---|
| `a_cap_is_what_one_goal_may_use_up` | a goal with 0 turns and $0.00 is not refused, because `Caps` is documented in three unchanged files as what **one goal** may use up | red |
| `a_capped_goal_reports_what_that_goal_spent` | the `CappedGoal` `report_capped` publishes describes one goal: `spent_usd` agrees with the `Reached` in the same struct | red |

`cargo test -p swarm-server --test the_ceiling_under_attack`, exit **101**, before any suite run.

## The gate, each exit status read on its own

| command | result | exit |
|---|---|---|
| `cargo fmt -p swarm-server --check` | no diff | 0 |
| `cargo clippy -p swarm-server --all-targets` | `Finished dev profile` | 0 |
| `cargo test -p swarm-server` | stops at `the_ceiling_under_attack`: `FAILED. 0 passed; 2 failed` | 101 |
| `cargo test -p swarm-server --no-fail-fast` | 69 passed, 2 failed across 15 targets | 101 |

**71 executed**, of which the adversary's 2. Deselecting its file leaves **69**.

## Findings

**F1 — `trigger.rs:322` / `budget.rs:57,60,63,88` — a cap is no longer "what one goal may use up",
and every document that says so is unchanged.** `capped` now applies the same `Caps` to
`spend_by_agent(agent)`, and both call sites pass the constant `COORDINATOR`, which is the only
agent the runtime ever attributes a row to (`trigger.rs:214`, `coordinator.rs:803`). So today the
second bound is **swarm-wide**: `SWARM_MAX_SPEND_USD=5` is $5 for the whole swarm across every goal.
The unit declined to change `budget.rs`, arguing `Caps` "already expresses a bound over (turns,
spend) with no goal in it". Measured false in five places in `budget.rs` (`:39` table, `:57`, `:60`,
`:63`, `:88`), `state.rs:633`, `state.rs:874`, and `src/web/src/runtime.ts:166` — a consumer
contract in another package.
*Measured*: `tests/the_ceiling_under_attack.rs:107`, exit 101 — a goal with 0 turns and $0.00 is refused.
*What reaches it*: `fire` at `trigger.rs:253` calls this function with this agent for every AWAITING
row, default `Caps`, default coordinator. A second goal in a swarm whose coordinator has spent $5.00
on the first never takes one turn.
NEEDS-CHANGE / introduced.

**F2 — `trigger.rs:281-289` — `CappedGoal` publishes two folds that contradict each other.** `turns`
and `spent_usd` come from the caller and `spend_on(goal)`; `reached`/`why` come from whichever branch
of `capped` fired. When the agent branch fires, `/status` publishes for a goal that has never run:
`{"turns": 0, "spent_usd": null, "reached": {"cap":"spend","spent":6.0,"max":5.0}, "why": "the spend
cap was reached: $6.00 of $5.00"}`. A reader sees a goal that spent nothing, stopped for spending
$6.00, and is told to raise `SWARM_MAX_SPEND_USD` for a goal that used none of it.
*Measured*: `tests/the_ceiling_under_attack.rs:150`, exit 101.
*What reaches it*: `fire:253` → `report_capped:256` → `Server::report_capped` (`state.rs:640`) →
`capped_goals()` (`state.rs:657`) → `Status.capped` (`state.rs:878`) → `CappedGoal` in
`src/web/src/runtime.ts:173`, and `What::Capped` on the watch stream.
NEEDS-CHANGE / introduced.

**F3 — `trigger.rs:146` and `:253` — the `agent` argument at both call sites is untested.**
`the_turn_cap_under_attack.rs:207` calls `capped(caps, &swarm, "worker-a", goal, 1)`. Every
loop-driving test builds a **single-goal** swarm, where `spend_by_agent == spend_on` and the per-goal
branch returns first, so the agent branch's verdict is never the one that fires. Replace
`COORDINATOR` with any other string at `:146` and `:253` and nothing in the suite goes red.
*Measured*: read, not run — the mutation probe needs a scratch copy of the workspace and a full
build, and the disk is at 99%. Stated as read.
CONFIRMED / introduced, warning.

**F4 — `trigger.rs:333` — the archived-records defence is on the per-goal bound only.** The per-goal
branch keeps `iterations.max(attempts)`; the agent branch reads bare `attempts`. The new doc at
`trigger.rs:316` claims "the same caps apply to it".
*What reaches it*: **nothing found.** There is no archive, rotate or prune path for `spend.jsonl`
anywhere in the tree; `grep -rn "archiv" src` returns one hit, that doc comment.
INFEASIBLE / introduced, note.

**F5 — the reported case count does not reproduce.** The unit reported "package suite 68 → 70".
`--no-fail-fast` with the adversary's file deselected executes **69**. The unit added one case
(`swarm.rs:1590`) and renamed one; 68 + 1 = 69.
CONFIRMED / undecided, note.

## Attacked and could not break

- **Both wave-c adversary cases are byte-identical to base** — diffed against `git show 9afa0b8:`.
- **The pinned case was not relaxed while being rewired** — `turns == 6`, `spent == 6.00` and the
  per-goal `!refused` fold survive verbatim as `!refused_per_goal`; three assertions added, none
  removed.
- **The ceiling is reachable in the real loop** — `record_spend` writes `"agent": actor`, and `actor`
  is `COORDINATOR` on both the answered (`coordinator.rs:803`) and unfinished (`trigger.rs:214`)
  paths, so the fold's key matches what is written.
- **The wave-c off-by-one is not reintroduced** — the agent branch can only add refusals; the
  per-goal `iterations.max(attempts)` still runs first, so `SWARM_MAX_TURNS=3` still stops at three.
- **Ordering does not mask a refusal** — the per-goal branch returning first can only change the
  *reason* reported, never suppress a refusal the agent branch would have made. Reason-only drift is F2.
- **Unattributed rows cannot evade the ceiling** — `"agent": null` rows are outside every agent's
  fold but inside `spend_on`, which is checked first.
- **A second agent's spend bounds nothing** — `capped` is only ever called with `COORDINATOR`.
  Nothing in the tree spawns a second agent or records a row for one. **Nothing found reaches it**,
  so not promoted.

**Note by the coordinator.** The adversary returned this block with unquoted `message` scalars, two
of which contain a colon-space and are therefore not valid YAML; the store refused it. The messages
are reproduced word for word as folded scalars. Nothing else was changed.

```findings
- file: src/runtime/swarm-server/src/trigger.rs
  line: 322
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    the second bound makes SWARM_MAX_TURNS and SWARM_MAX_SPEND_USD swarm-wide because both call
    sites pass COORDINATOR, while budget.rs:39,57,60,63,88, state.rs:633,874 and
    src/web/src/runtime.ts:166 all still say a cap is what one goal may use up, so a goal with 0
    turns and $0.00 is refused.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 281
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    report_capped composes CappedGoal from two folds that no longer agree, publishing a goal as
    turns 0 and spent_usd null alongside a reason reading "the spend cap was reached: $6.00 of
    $5.00" to /status, the watch stream and the web CappedGoal interface.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 146
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the agent argument at both call sites has no coverage — every loop-driving test uses a
    single-goal swarm where the per-goal branch returns first, and the only case exercising the
    agent branch passes the literal "worker-a", which the runtime never passes.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 333
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    the agent branch reads bare attempts rather than iterations.max(attempts), so the documented
    archived-records defence covers only the per-goal bound, but no archive, rotate or prune path
    for turns/spend.jsonl exists anywhere in the tree.
- file: src/runtime/swarm-server/tests/the_turn_cap_under_attack.rs
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: >-
    the unit reported a package suite of 68 to 70 cases, but cargo test -p swarm-server
    --no-fail-fast with my file deselected executes 69.
```

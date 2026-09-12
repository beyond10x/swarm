---
format: aep.planning-md/1
id: review-result:adversary-unit-a-pass-2
kind: review-result
status: active
title: Adversary, unit A, pass 2
relations:
- reviews: story:redelivery-for-at-least-once
revision: 1
---
```
unit: A
verdict: red
cases: executed 50→52, red 2
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: four log files under the assigned scratch directory
needs-coordinator: no
```

Adversary pass 2 against `wave/2026-09-12/redelivery` at `35dabcb`. One new test file,
`src/runtime/swarm-server/tests/the_cascade_of_a_caused_command.rs`. No implementation file touched.

Both cases drive the implementation from documents this unit wrote in this diff, using a copy of
`src/core` in a temporary directory with one binding appended — the extension point `main.rs`
documents. The shipped kernel is not modified.

| command | exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --all-targets` | 0 |
| `cargo test -p ess-runtime -p swarm-server` | 101 |
| the same with the two new cases skipped | 0, 50 cases |

Attacked and could not break: `MAX_ATTEMPTS` arithmetic and the not-yet-due boundary; the drain
ordering, where each pass iterates once and no delivery is attempted twice; `What::Undelivered`,
announced from both abandonment arms and never for a re-queued delivery; the two characterisation
cases, which assert today's behaviour accurately and go red the day a request key becomes
idempotent; and three of the implementor's self-audited claims, spot-checked and true.

```findings
- file: src/runtime/swarm-server/src/swarm.rs
  line: 820
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: redeliver holds self.owed across commit_caused, whose failure arm re-locks it through owe(), so a redelivered command's failing cascade deadlocks the drain while it still holds self.world; no shipped binding reaches it, the documented components.yaml extension point does.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 7
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the rewritten module doc says all three of issue, tick and redeliver run the pump, and Swarm::tick calls no pump at all, so a command a period caused causes no cascade.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 906
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: once the deadlock is fixed, a cascade delivery kept by owe() is overwritten by the trailing `*owed = keep`, losing it with no give_up and no Undelivered.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 726
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: commit_caused's Ok arm uses `?` on store.commit, so a commit failure neither owes that delivery nor examines the remaining bindings of a fan-out, while redeliver's own arm keeps such a delivery — the shared path is not symmetric for its two callers, and I could not construct a commit failure to measure it.
```

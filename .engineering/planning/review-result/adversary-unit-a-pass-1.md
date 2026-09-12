---
format: aep.planning-md/1
id: review-result:adversary-unit-a-pass-1
kind: review-result
status: active
title: Adversary, unit A, pass 1
relations:
- reviews: story:redelivery-for-at-least-once
revision: 1
---
```
unit: A
verdict: red
cases: executed 46→48, red 2
origin: introduced 5, pre-existing 2, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

Adversary pass 1 against `wave/2026-09-12/redelivery` at `1e7e813`, base `a6861dd`.

One path touched: `src/runtime/swarm-server/tests/redelivery_under_attack.rs`, new and untracked. No
implementation file changed.

Two cases added, both red:

```
test a_retry_under_the_same_request_key_does_not_owe_the_same_delivery_twice ... FAILED
test one_request_key_spent_twice_on_a_creating_command_creates_one_instance ... FAILED

---- a_retry_under_the_same_request_key... stdout ----
assertion `left == right` failed: a retry under the same idempotency key answers what the first request answered
  left: Ok("activated")
 right: Ok("wrong-state")

---- one_request_key_spent_twice_on_a_creating_command... stdout ----
assertion `left == right` failed: one request key is one request: Ok("drafted") then Ok("drafted") left 2 configs
  left: 2
 right: 1
```

`cargo test -p ess-runtime -p swarm-server` exit 101, 46 passed 2 failed.
`cargo fmt --all --check` exit 0. `cargo clippy --all-targets` exit 0.

The duplicate-owed-delivery theory is dead: `deliveries().len() == 1` after the retry passes, because
`ActivateConfig` answers `wrong-state` with no events and the pump never re-runs.

Attacked and could not break: `MAX_ATTEMPTS` counts the first attempt as claimed; `Routed.input`
crosses no host state into `route.rs`; `owe()` filters correctly to the three bindings and leaves
`at_most_once`/`drop` alone; lock order is `world` then `owed` in both paths; the restart claim is
accurate and nothing believes a lost delivery was made; the drain reaches the same cached handle the
trigger walks.

```findings
- file: src/runtime/swarm-server/src/http.rs
  line: 39
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a client retrying a command under the documented idempotency key gets a wrong-state refusal instead of the first request's answer, so the key is an append guard only and not the retry contract the field's doc promises.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 303
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Delivery::request claims a shared key makes a duplicate delivery unable to double-write, but store.rs scopes the key to the instance stream and a creating command such as RecordAssignment mints a new stream per attempt, so nothing refuses anything.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 8
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the module doc still says issue is the only place a command is applied, the log is written and the pump runs, but redeliver does the first two and never pumps the redelivered command's events.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 779
  category: mutant
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the commit-failure arm of redeliver drops a delivery at MAX_ATTEMPTS with no What::Undelivered and no error log, which is the silence the Undelivered variant was added to end, but no store commit failure could be constructed from a test.
- file: src/runtime/swarm-server/src/state.rs
  line: 75
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: Server::open is a read-then-insert across an await and can replace a Swarm handle that now holds the only copy of an owed delivery, but the only caller drops its handle without issuing so nothing reaches it.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 66
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: RETRY_DELAY equals the trigger period so a retry lands 30 to 60 seconds after the failure, and because redeliver runs after each earlier swarm's coordinator turn the upper bound grows with swarm count.
- file: src/runtime/swarm-server/tests/serves_a_swarm.rs
  line: 263
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the only test proving a successful redelivery manufactures its transient failure by opening two Swarm handles over one log, a state store.rs explicitly excludes; it is deterministic rather than flaky, but the success path rests entirely on a configuration nothing reaches.
```

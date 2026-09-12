---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-b-pass-2
kind: review-result
status: active
title: Adversary, bug wave unit B, pass 2
relations:
- reviews: story:open-is-a-read-then-insert
revision: 1
---
```
unit: story:open-is-a-read-then-insert (unit B, 2026-09-12b), worktree wave-20260912b-b at head 8224c88 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 24→25, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: whether a process-wide open lock is acceptable for `POST /swarms`, given the story's acceptance did not ask for one and 8cabdd7 already satisfied it
```

Recorded by the coordinator from the adversary's returned report, unedited.

## 1. What was touched

```
?? src/runtime/swarm-server/tests/open_does_not_queue_behind_an_unrelated_slug.rs
```

One path, a test file. No implementation file touched, no existing case deleted, weakened or
rewritten.

## 2. The case added

`an_open_does_not_wait_for_an_unrelated_slugs_open` — builds `heavy` (4000 commands) on disk through
a throwaway `Server` that is then dropped, so the log exists and the server under test has never seen
it; spawns `open("heavy")`, waits 75 ms, then times `open("light")`, a slug with no directory, no log
and no handle in common. Asserts `light` finishes before `heavy` does. Carries a first assertion that
refuses to conclude anything if `heavy` finished before `light` was asked for.

```
$ cargo test -p swarm-server --test open_does_not_queue_behind_an_unrelated_slug
running 1 test
test an_open_does_not_wait_for_an_unrelated_slugs_open ... FAILED

thread '...' panicked at tests/open_does_not_queue_behind_an_unrelated_slug.rs:117:5:
`light` opened at 253.425252ms, after `heavy` finished at 200.25846ms: the open of a slug with no
log, no directory and no handle in common with `heavy` waited for `heavy`'s replay. One
process-wide `opening` mutex serialises every slug's construction, not just the contended one, so
two clients creating two different swarms at once (`http.rs:102`) are served one after the other.

test result: FAILED. 0 passed; 1 failed; finished in 13.46s
```

The first draft (1200 commands, 150 ms head start) tripped the inconclusive guard — `heavy` finished
at 75.7 ms — and the knob was raised before the red above was recorded. The guard is why that draft
is not reported as a finding.

## 3. The suite

```
$ cargo fmt -p swarm-server -- --check        # exit 0
$ cargo clippy -p swarm-server --all-targets  # no warnings
$ cargo test -p swarm-server --no-fail-fast
     unittests src/lib.rs                              ok.      10 passed; 0 failed
     unittests src/main.rs                             ok.       0 passed; 0 failed
     tests/open_does_not_queue_behind_an_unrelated_slug.rs  FAILED.  0 passed; 1 failed
     tests/open_is_one_swarm_under_contention.rs       ok.       2 passed; 0 failed
     tests/redelivery_under_attack.rs                  ok.       2 passed; 0 failed
     tests/serves_a_swarm.rs                           ok.       7 passed; 0 failed
     tests/the_cascade_of_a_caused_command.rs          ok.       3 passed; 0 failed
exit status 101
```

## 4. Findings

| # | file:line | verdict / origin | what was measured | what reaches it |
|---|---|---|---|---|
| 1 | `state.rs:113` (`let _constructing = self.opening.lock().await`) | NEEDS-CHANGE / introduced | `light` opened at 253 ms, `heavy` finished at 203 ms. Construction of every slug is serialised on one mutex held across `Swarm::open`, which is `Store::open` plus a full replay. 8cabdd7 already met the story's acceptance without a lock | `http.rs:102` (`POST /swarms`) is the only caller. Two clients creating two different swarms at once are now served one after the other — reached, but the constant is small for a brand-new slug. The large instance needs a swarm directory on disk absent from the map, which after `Server::start` requires the swallowed `if let Ok(entries)` at `state.rs:58` to have failed, or a directory restored while the process runs |
| 2 | `state.rs:120` (`let loser = if swarms.contains_key(slug)`) | CONFIRMED / introduced | the branch is unreachable. After `Server::start` the only writer to `swarms` is `open`, which holds `opening` and re-reads the map under it, so `contains_key` is always false and `loser` always `None`. A bare insert would leave the suite green — the acceptance clause about dropping the losing handle is now satisfied vacuously | nothing found. Dead code guarding a state the new lock makes impossible |
| 3 | `state.rs:25` | INFEASIBLE / introduced | the field doc says at most one `Swarm::open` runs at a time "in this process". `opening` is a field of `Server`, not a static; two `Server` values over one root have two mutexes | `main.rs:47` constructs exactly one `Server`. A doc line is the whole fix |
| 4 | `state.rs:100` | INFEASIBLE / introduced | "when the store fails and no handle is in the map, the error is returned" implies a contrasting case. `Swarm::open(...)?` returns `Err` unconditionally and never consults the map | nothing found; cutting the qualifier costs one doc line |

## 5. Attacked, could not break

- The retry-loop removal in `serves_a_swarm.rs:388-392`: the case still proves its claim, and is not flaky in this suite.
- Nothing was weakened rather than lifted; the two parked cases are verbatim and neither assertion was loosened.
- Deadlock between `opening` and the `swarms` RwLock: no lock is acquired in the other order, nothing else writes `swarms`, no read guard is held across an await.
- A genuine store failure still returns `Err`, and the `?` has no fallback path around it.
- `Server::start` and the trigger loop never contend on `opening`.
- `Server::open`'s `expect` cannot panic.

## 6. Paths written outside the worktree

None.

```findings
- file: src/runtime/swarm-server/src/state.rs
  line: 113
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: one process-wide `opening` mutex held across `Swarm::open` serialises the construction of every slug, so an open of an unrelated slug waits for another slug's full log replay, which the story's acceptance never asked for and 8cabdd7 already satisfied without a lock.
- file: src/runtime/swarm-server/src/state.rs
  line: 120
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the `contains_key`/`loser` branch is unreachable under `opening`, so the acceptance clause about dropping the losing handle rather than inserting it is now vacuous and replacing the branch with a bare insert would leave the suite green.
- file: src/runtime/swarm-server/src/state.rs
  line: 25
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the field doc claims at most one `Swarm::open` runs at a time "in this process", but `opening` is per-`Server`, and only `main.rs:47` constructing exactly one server makes the claim true.
- file: src/runtime/swarm-server/src/state.rs
  line: 100
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the doc's qualifier "and no handle is in the map" implies a map fallback on store failure that `Swarm::open(...)?` does not perform, in a state that cannot occur.
```

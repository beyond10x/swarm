---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-b-pass-1
kind: review-result
status: active
title: Adversary, bug wave unit B, pass 1
relations:
- reviews: story:open-is-a-read-then-insert
revision: 1
---
```
unit: story:open-is-a-read-then-insert (unit B), worktree wave-20260912b-b at 8cabdd7, base 0c24ac0
verdict: NEEDS-CHANGE
cases: executed 22→25, red 3
origin: introduced 2 / pre-existing 2 / undecided 0
wrote-outside-worktree: none
needs-coordinator: findings 2 and 3 are fixed in src/runtime/ess-runtime/src/store.rs, which unit B does not own — they need their own story, not a route back
```

Recorded by the coordinator from the adversary's returned report, unedited.

## 1. What was touched

```
$ git --no-pager diff --stat
(empty — the adversary's file is untracked)
$ git status --short
?? src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs
```

One path, a new test file. No implementation file touched, no existing case altered.

## 2. The cases, and their red output when written

**`no_concurrent_open_of_one_slug_is_refused` (:46)** — asserts the sentence this unit added at
`state.rs:75`: "Every call for one slug returns the same handle, however many run at once." 40
rounds x 8 simultaneous opens of a fresh slug, no retry. First run, at 12 rounds:

```
running 2 tests
test every_concurrent_opener_of_one_slug_holds_a_handle ... ok
test no_concurrent_open_of_one_slug_is_refused ...
thread 'no_concurrent_open_of_one_slug_is_refused' panicked at .../open_is_one_swarm_under_contention.rs:73:5:
3 of 96 concurrent opens were refused, but `Server::open`'s doc promises every call for one slug the same handle however many run at once: [
    "contested-0: event store is unavailable: database is locked",
    "contested-0: event store is unavailable: database is locked",
    "contested-0: event store is unavailable: database is locked",
]
test result: FAILED. 1 passed; 1 failed
```

At 12 rounds it was red 6 runs in 8; raised to 40 rounds it has been red every run since, 5-6
refusals per 320.

**`an_open_never_creates_a_directory_outside_the_swarms_directory` (:94)** and
**`two_slugs_naming_one_directory_are_one_swarm` (:114)**, run alone:

```
thread '...' panicked at .../open_is_one_swarm_under_contention.rs:104:5:
`open("../escaped")` created /home/timo/.cache/claude-tmp/swarm-adversary.Qbiumf5pnowm/escaped, which is not under .../swarms, and returned Ok(../escaped)

thread '...' panicked at .../open_is_one_swarm_under_contention.rs:151:5:
`alias` and `./alias` are one directory (.../swarms/alias) but two handles: the second holds a world without the swarm the first just created (0 instances against 1), and an `owed` delivery queued on either is invisible to the other
```

The first returns **`Ok`** — the traversing swarm opens, works, and is registered.

## 3. The gate, after the cases existed

| command | result |
|---|---|
| `cargo fmt -p swarm-server -- --check` | exit 0 |
| `cargo clippy -p swarm-server --all-targets` | `Finished dev profile`, exit 0 |
| `cargo test -p swarm-server` | `test result: FAILED. 0 passed; 3 failed` on the adversary's target, exit 101 |
| the same with that target deselected | 10 + 0 + 2 + 7 + 3 = **22 passed, 0 failed**, exit 0 |

The unit's own suite is green; 22→25 with the adversary's, 3 red.

## 4. Findings

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | `state.rs:75` | doc promises every concurrent call for one slug the same handle; 5-6 of 320 get `Err(Store)` instead | `POST /swarms` (`http.rs:99-104`) — two clients creating one slug at once: one `201`, one `500 {"kind":"store"}` for a healthy swarm | NEEDS-CHANGE | introduced |
| 2 | `ess-runtime/src/store.rs:95` | `open("../escaped")` returns `Ok` and creates a working swarm outside `swarms/` | `POST /swarms` body `slug` is an unvalidated `String`; `create_dir_all` runs before `TenantId::new` | CONFIRMED | pre-existing |
| 3 | `state.rs:88` | `entry(slug.to_owned())` dedupes the string, not the directory: `alias` and `./alias` are two live handles over one `eventlog.sqlite3`; 0 instances against 1 | same route, same missing validation — the unit's own stated harm reached with no concurrency at all | CONFIRMED | pre-existing |
| 4 | `state.rs:88` | the loser `Arc<Swarm>` is dropped inside `or_insert`, with the `swarms` write guard held, so a SQLite connection close runs under the global map lock | every losing open; impact not measured | CONFIRMED (note) | introduced |
| 5 | `tests/serves_a_swarm.rs:391` | the retry loop catches every `Refused`, including finding 1's; masking the race needs 7 of 8 opens refused against a measured ~1.7% | not shown to be reached | INFEASIBLE | introduced |

Named fixes, not applied. 1: on `Swarm::open` returning `Err`, re-check the map under the write
guard and return the handle if one is there, or reword the doc. 2/3: validate the slug in
`Store::open` before `create_dir_all`, which is `ess-runtime`. 4: clone out of the entry, drop the
guard, then drop the loser.

## 5. Attacked and could not break

- "`self.swarms.write()` occurs exactly once in the crate" — true. `state.rs:87` is the only hit across `src/` and `tests/`.
- "no other shared state on `Server` has the same shape" — true. `next_tick_at`, `ticks`, `in_flight`, `capped` are `std::sync::Mutex`, every mutation one statement, no guard across an await.
- "does dropping the loser lose anything?" — no. `Swarm::open` does an idempotent `create_dir_all`, opens the store, folds the world read-only, makes a broadcast channel with no subscribers, sets `owed` empty.
- The unit's new case is red at every step for the right reason.
- `Server::start` reopening the same slugs — no interaction; the map is built before the server exists.

## 6. Paths written outside the worktree

None.

```findings
- file: src/runtime/swarm-server/src/state.rs
  line: 75
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the doc this change added promises every concurrent open of one slug the same handle, but a concurrent open whose Swarm::open lost the SQLite race returns Err without ever looking at the map, so POST /swarms answers 500 for a swarm that is open and healthy.
- file: src/runtime/ess-runtime/src/store.rs
  line: 95
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: Store::open runs create_dir_all on root/swarms/<slug> before any validation of the slug, so open("../escaped") returns Ok and leaves a working swarm outside the swarms directory, reachable from the unvalidated slug in POST /swarms.
- file: src/runtime/swarm-server/src/state.rs
  line: 88
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the fix dedupes on the slug string rather than the directory, so "alias" and "./alias" produce two live Swarm handles over one eventlog with divergent in-memory worlds and owed queues - the unit's own stated harm, reached with no concurrency at all.
- file: src/runtime/swarm-server/src/state.rs
  line: 88
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the losing handle is dropped inside or_insert with the swarms write guard still held, so a SQLite connection close runs under the global map lock and blocks every open, get, slugs and status for its duration; the impact was not measured.
- file: src/runtime/swarm-server/tests/serves_a_swarm.rs
  line: 391
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the new case's retry loop swallows every Refused including the contract breach in finding 1, but masking the race itself would need 7 of 8 opens refused and the measured refusal rate is about 1.7 percent, so no reachable masking scenario was shown.
```

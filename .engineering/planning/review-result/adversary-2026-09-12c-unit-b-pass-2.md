---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12c-unit-b-pass-2
kind: review-result
status: active
title: Adversary, wave c unit B, pass 2
relations:
- reviews: story:a-swarm-cannot-be-removed
revision: 1
---
```
unit: unit B of wave 2026-09-12c — DELETE /swarms/{slug} and the deferred unlink, at 0d2d555 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 46→49, red 3
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: none
needs-coordinator: none
```

Recorded by the coordinator from the adversary's returned report, unedited.

## 1. What was touched

```
?? src/runtime/swarm-server/tests/adversary_pending_removal.rs
```

One test file. No implementation file touched.

## 2. The cases, all red

```
panicked at tests/adversary_pending_removal.rs:142:5:
the second removal answered `Ok(Now)` and unlinked `.../swarms/parked` while a handle over that
log is still alive and still answering `created` to whoever holds it — the deferred unlink
`pending` exists for is skipped because `remove` branches on the `swarms` map, which a parked
slug is not in

panicked at tests/adversary_pending_removal.rs:215:5:
the retried DELETE answered 204 and the directory is gone while the handle that was the reason
for the first answer's `202` is still alive

panicked at tests/adversary_pending_removal.rs:184:18:
a slug holding a live `swarm.manager.Swarm` record answered `Ok(Now)` instead of
SwarmStateConflict, because the record was reachable only through the handle parked in `pending`
and `remove` asks only the `swarms` map; the directory is now erased
```

## 3. The suite

`cargo test -p swarm-server --no-fail-fast` — 10, 0, 10, **FAILED 0/3**, 6, 2, 2, 7, 3, 4, 2; exit
101. 49 executed, 3 red, 46 before. `fmt --check` 0, `clippy` 0, `npm test` 59/59, `vue-tsc` 0. No
ENOSPC; 9.0G free throughout.

## 4. Findings

**F1 — `remove` branches on the `swarms` map, so a second removal of a parked slug unlinks under the
live holder.** `state.rs:321`: `pending` is consulted only to avoid `NotHere`. Everything after keys
off `held = self.swarms.read().get(slug)`, and a parked slug is not in `swarms`. So `held` is `None`,
the park branch at `:339` is never reached, and `:354 unlink(&directory)` runs while `pending` still
holds an `Arc` over that log — the exact state the `pending` field's own doc calls a HONESTY problem.
Reached by `DELETE` answering `202` and the client retrying, which is what `202 Accepted` means and
what `http.rs:148-150` documents. Holders are ordinary: every command handler holds one for its whole
request, and the trigger holds every handle for a whole tick including a coordinator turn.
**Fix named:** `let held = swarms.get(slug).or_else(|| pending.get(slug))`, so the strong-count check,
the `live_state` refusal and the re-park all run for a parked slug too.

**F2 — the `live_state` refusal is skipped entirely for a parked slug.** `state.rs:327`, same root.
The doc on `remove` says "ANY live record refuses it"; a record written through the parked handle
refuses nothing and the slug is erased. The holder that caused the `202` is by construction still
issuing commands, and `CreateSwarm` is accepted through it.

**F3 — `204` is answered for a disk state the same call made false.** `http.rs:155` maps
`Removed::Now` to `204`, documented as "the place is gone". After F1's path the place is gone for
readers and still being written by the holder — precisely the claim `202` was invented to avoid
making. The wire-visible face of F1, listed separately because it is the `202`/`204` distinction and
it fails in the direction the unit argued it would not.

**F4 (judgement, residue) — `live_state` and `listed()` use two different rules for a record with no
readable `state`.** `state.rs:415` uses `filter_map`, so a record whose `state` is absent or
non-string is dropped and the slug counts as removable; `listed()` at `:497` uses `is_some_and` on
the same field, so the same record is not terminal and the slug is shown as live. One field, two
answers, twelve lines apart. No producer of that record shape could be constructed, so no case was
written.

## 5. Attacked and could not break

- `state::check` — `only == OsStr::new(slug)` is byte-exact against the normalised single component; `busy/`, `busy/.`, `./busy`, `..`, `.`, embedded separators and NUL all refused.
- **The four doors** — every one of the 12 `{slug}` routes reaches the filesystem only via `get` or `open`, plus `remove` and the boot scan; all four call `check`. **No fifth path.**
- A parked slug leaking forever — `sweep` is called from `open`, `remove`, `listed` and `all`, and the trigger calls `all` every tick.
- A removed-but-pending swarm reappearing in `listed()` — it sweeps, then reads `swarms`, which a parked slug has left.
- Two `sweep`s racing, and `sweep` racing `open` — both serialise on the per-slug gate, one gate at a time, none held on entry. No cycle, no window.
- `listed()`'s fan-out — one task per open swarm, bounded by directories on disk, not by request traffic.
- Process death between eviction and unlink — leaves a directory the next boot scan adopts as a normal terminal swarm. Not an orphan, and removable.

```findings
- file: src/runtime/swarm-server/src/state.rs
  line: 321
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: remove reads pending only to avoid NotHere and then branches on the swarms map, so a second removal of a parked slug takes the held-is-none path and unlinks the files while the pending map still holds a live Arc over that log.
- file: src/runtime/swarm-server/src/state.rs
  line: 327
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the live_state refusal runs only when a handle is in the swarms map, so a parked slug holding a non-terminal record written through its parked handle is erased instead of answering SwarmStateConflict.
- file: src/runtime/swarm-server/src/http.rs
  line: 155
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a retried DELETE answers 204 the place is gone for a swarm whose handle is still live and still being written through, which is exactly the claim the 202 path was added to avoid making.
- file: src/runtime/swarm-server/src/state.rs
  line: 415
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: live_state drops a record whose state is missing or non-string and treats the slug as removable, while listed at line 497 treats the same record as live, two rules for one field with no producer of that record shape found.
```

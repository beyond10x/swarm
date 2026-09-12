---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12c-unit-b-pass-1
kind: review-result
status: active
title: Adversary, wave c unit B, pass 1
relations:
- reviews: story:a-swarm-cannot-be-removed
revision: 1
---
```
unit: story:a-swarm-cannot-be-removed — worktree wave-20260912c-b at ccad939 (base 80b9d24), plus one untracked test file
verdict: NEEDS-CHANGE (4 red cases; one is an unauthenticated arbitrary-directory erase)
cases: executed 36→42, red 4
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (scratch/adversary, empty, unused)
needs-coordinator: the UI finding needs a product call on how an operator reaches a Deleted swarm at all
```

Recorded by the coordinator from the adversary's returned report, unedited.

**Containment, established by the coordinator before routing:** the running server answers `405` to
`DELETE /swarms/{slug}` and `main` declares no `delete(` route. The defect exists only on the
unmerged branch `wave/2026-09-12c/removal`. Nothing is exposed.

## 1. What was touched

```
?? src/runtime/swarm-server/tests/adversary_removal.rs
```

One test file. No implementation file touched.

## 2. The cases

| line | asserts | now |
|---|---|---|
| 122 | `DELETE /swarms/..%2Fescaped` does not erase a directory outside `swarms/` | **red** |
| 152 | a `Running` swarm reached as `.%2Fbusy` is still refused `409` | **red** |
| 191 | a slug whose log holds a `Running` record and a `Deleted` one is refused | **red** |
| 273 | a command on a handle taken before the removal is not told it succeeded | **red** |
| 317 | listing ten swarms under command traffic stays under 500 ms | green; the number is the finding |

```
panicked at tests/adversary_removal.rs:137:5:
DELETE /swarms/..%2Fescaped answered 204 () and erased
/home/timo/.cache/claude-tmp/swarm-adversary.DUtdpcl4sDlK/escaped, which is not under the
swarms directory at all

panicked at tests/adversary_removal.rs:168:5:
an alias of a Running swarm's slug answered 204 () and erased its directory without ever
consulting the state that refuses it

panicked at tests/adversary_removal.rs:301:5:
a command issued on a handle taken before the removal answered `created` and the record it
created is in no log any reader of this slug can reach

panicked at tests/adversary_removal.rs:224:5:
`mixed-1` holds a Running record and a Deleted one; DELETE answered 204 () and erased the
log the Running record lives in

test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

**A correction the adversary declared about its own work:** case 122 was green on its first run for a
fixture reason — a fresh `TempDir` has no `swarms/` directory, so the escaping path's intermediate
component does not stat. Adding `server.open("ordinary")` so `swarms/` exists — true of any
deployment that has ever held a swarm — turned it red for the reason claimed.

## 3. The gate

`cargo test -p swarm-server --no-fail-fast` — 10, 6, **FAILED 2 passed/4 failed**, 2, 2, 7, 3, 4, 2.
`fmt --check` clean, `clippy` clean, `npm test` 57/57, `vue-tsc` exit 0. 11G free throughout, debug
only.

Listing measurement, `--nocapture`: `slugs() 9.437µs` · `listed() 219.56641ms` for ten swarms under
a command loop — about 23,000×.

## 4. Findings

| # | file:line | what was measured | what reaches it |
|---|---|---|---|
| 1 | `state.rs:193` | `DELETE /swarms/..%2Fescaped` → `204`; `remove_dir_all` erased `<root>/escaped` and its contents | the route table, **unauthenticated, one HTTP request**. axum matches `{slug}` on the encoded segment and the extractor percent-decodes. Only precondition: `<root>/swarms` exists |
| 2 | `state.rs:191` | `DELETE /swarms/.%2Fbusy` on a `Running` swarm → `204`, directory erased, handle left live over a log that is gone | the refusal is keyed on the slug **string** in the handle map, the unlink on the slug **path**; `./busy` and `busy` are one directory and two keys, so the conflict branch is skipped entirely |
| 3 | `state.rs:196` | a slug holding a `Running` record and a `Deleted` one → `204`, log erased | `Swarm::summary` keeps the **last** `swarm.manager.Swarm` it walks, so removability depends on minted-id sort order. Reached by issuing `CreateSwarm` twice with different `tmux_session` values; nothing enforces one record per slug |
| 4 | `state.rs:268` | a command on a handle taken before the removal answered `created`; the next `open` finds no record | `Server::get` takes no `opening` gate, so a command concurrent with the delete is the live pair. The doc at `state.rs:170-174` claims "nothing hands that handle to a new caller"; `get` does |
| 5 | `src/web/src/stores/actions.ts:57` | reasoned from two halves, not a failing case: `listed()` removes terminal swarms from `GET /swarms`, the store's only record source, so a `Deleted` swarm's page renders "No such swarm — the runtime does not hold one by that name", which is false | navigating to or reloading a `Deleted` swarm's page. **The one text that tells an operator how to finish the job is unreachable in the running app while its test asserts it.** `visible` is now dead code |
| 6 | `state.rs:299` | `listed()` takes every swarm's world mutex serially, the same mutex `issue` holds across its commit and `tick` across its cascade | `GET /swarms` on every page load. One busy swarm now delays the whole list |

Named fixes, not applied: (1)(2) reject any slug that is not a single normal path component before it
is joined — `story:slug-is-not-validated` is the general home, but a destructive verb cannot wait for
it; (3) refuse if **any** `swarm.manager.Swarm` instance in the world is non-terminal rather than the
one `summary()` happens to keep; (4) take the `opening` gate in `get`, or refuse while another `Arc`
is outstanding; (5) keep terminal swarms in `GET /swarms` and filter in `visible` as before, or give
the UI a second list; (6) filter on something cheaper than `summary()`, or cache the state.

## 5. Attacked and could not break

- `OFFERED` against `manager.yaml:94-110` — all five action/`from` sets match the declared transitions.
- `Stopped` is not terminal, `DELETE` refuses it `409`, which is what the acceptance asks.
- `terminal_states()` reads the IR rather than hard-coding; a second terminal state would be honoured.
- The three-file SQLite removal: the re-open assertion does distinguish a left-behind `-wal`, and no stale `-wal` could be constructed through the public surface.
- The `Arc`-before-unlink ordering: still unmeasurable on Linux. The limitation the unit admitted is real.
- `404` for an unknown slug, the `409` body, `with_suffix` versus `set_extension` — all correct.
- Web gate 57/57 and `vue-tsc` clean, before and after.

```findings
- file: src/runtime/swarm-server/src/state.rs
  line: 193
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: DELETE /swarms/..%2Fescaped joins an unvalidated slug onto the data root and remove_dir_all erases an arbitrary directory outside swarms/, answering 204, from one unauthenticated request.
- file: src/runtime/swarm-server/src/state.rs
  line: 191
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the SwarmStateConflict refusal is keyed on the slug string in the handle map while the unlink is keyed on the slug path, so DELETE /swarms/.%2Fbusy erases a Running swarm's directory without ever consulting its state.
- file: src/runtime/swarm-server/src/state.rs
  line: 196
  category: mutant
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the state check reads Swarm::summary, which keeps only the last swarm.manager.Swarm instance in the world, so a slug holding both a Running and a Deleted record is removable whenever the Deleted one sorts last by minted id.
- file: src/runtime/swarm-server/src/state.rs
  line: 268
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Server::get hands out the handle with no opening gate, so a command issued on a handle taken before a removal answers created against an unlinked log and the client is told a write succeeded that no reader of that slug can ever see.
- file: src/web/src/stores/actions.ts
  line: 57
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: filtering terminal swarms out of GET /swarms empties the store's only record source, so a Deleted swarm's page renders "No such swarm" and the disabledNote text that points the operator at the delete route is unreachable in the running app while its test asserts it.
- file: src/runtime/swarm-server/src/state.rs
  line: 299
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: listed() takes every swarm's world mutex serially where slugs() read map keys, measured at 219.57ms against 9.44us for ten swarms under command traffic, so one busy swarm now delays the whole list on every page load.
```

# Unit B — story:open-is-a-read-then-insert

## Identity

```
story:  story:open-is-a-read-then-insert
branch: wave/2026-09-12b/open-race   (already checked out for you)
base:   0c24ac0504020c89d91e0fb2c65db7eb2af54f00
```

Review your own change with `git diff 0c24ac0...HEAD`.

## The triple

```
worktree:  /home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912b-b
build dir: <that worktree>/target
scratch:   /home/timo/.cache/swarm-wave-2026-09-12b/unit-b/scratch
```

## The defect

Read the story with `aep plan artifact show story:open-is-a-read-then-insert`.

`Server::open` in `src/runtime/swarm-server/src/state.rs:75-85` reads the map under a read lock,
drops the guard, awaits `Swarm::open`, then inserts under a write lock with no re-check. Two
concurrent opens of one slug both miss, both construct a handle, and the second insert replaces the
first. The replaced handle may hold the only copy of an owed delivery.

## The file assignment

| | |
|---|---|
| yours | `src/runtime/swarm-server/src/state.rs`, `src/runtime/swarm-server/tests/serves_a_swarm.rs` |
| not yours | `src/runtime/swarm-server/src/http.rs`, `swarm.rs`, `tests/redelivery_under_attack.rs` — unit A owns them this wave. `src/web/**` — unit C |

If your fix needs a change in `swarm.rs`, write the patch to your scratch directory and name it in
the report. Do not edit it.

## What "done" looks like

1. Two concurrent `Server::open` calls for one slug return the same `Arc<Swarm>`.
2. The losing handle is dropped, not inserted.
3. A test asserts it, and that test is red before your fix. Put the red run's own output in the
   report. A test that cannot be made to fail before the fix does not prove the fix.
4. The fix is the smallest one that holds: re-check under the write guard, or
   `entry(slug).or_insert(...)`. Do not restructure `Swarm::open`.
5. `Server::open`'s doc says what is now guaranteed.

## The gate

```
cargo fmt -p swarm-server
cargo clippy -p swarm-server --all-targets
cargo test -p swarm-server
```

Quote each command's summary line verbatim in the report.

## Repository invariants

* the build directory is `target/` **inside your worktree**. Never set `CARGO_TARGET_DIR`, never
  point at another tree's build directory
* `cargo fmt -p <crate>`, never `cargo fmt --all`
* never write under `.engineering/`, and run no `aep plan artifact` write verb. Reads (`show`,
  `list`, `--help`) are fine
* the gate is **package-scoped** for you. The whole gate runs once, later, on the integration
  branch, and is not yours to run
* no `git worktree`, no `git commit`, no `git add`, no `git stash`, no branch command. Leave your
  changes in the working tree; the coordinator commits them
* nothing under `/tmp`. Scratch goes in the scratch directory named above
* this repository has no `AGENTS.md` and no `CLAUDE.md`
* test first: write the failing case, run it, and put that red run's own output in your report

## The report header

Six lines, before any prose:

```
unit: <story-id>
verdict: green | red | blocked
cases: executed <before>→<after>, red <n>
origin: <the brief or finding this round answers>
wrote-outside-worktree: <paths> | none
needs-coordinator: <what you could not do and who owns it> | none
```

## Scope confirmation

Your story carries a `## Scope` section with lines marked `cited` and `inferred`. Check every
`inferred` line against the tree before you build on it, and return a table: path | the mark it
carried | what you found. Two inferred lines in the last wave were wrong.

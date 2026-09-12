# Unit A — story:request-key-is-not-idempotency

## Identity

```
story:  story:request-key-is-not-idempotency
branch: wave/2026-09-12b/request-key   (already checked out for you)
base:   0c24ac0504020c89d91e0fb2c65db7eb2af54f00
```

Review your own change with `git diff 0c24ac0...HEAD`.

## The triple

```
worktree:  /home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912b-a
build dir: <that worktree>/target
scratch:   /home/timo/.cache/swarm-wave-2026-09-12b/unit-a/scratch
```

## The decision, already taken

Read the story with `aep plan artifact show story:request-key-is-not-idempotency`. It offers two
acceptance branches. **The operator chose the doc branch: restate the contract, do not change
behaviour.**

So: the `request` field guards the APPEND, scoped by `store.rs` to one instance's stream, and it
does not make re-application idempotent. Say that where a reader meets it, and say what a
disconnected client actually gets.

Do NOT implement a request-to-answer record, do NOT touch `ess-runtime`'s schema, and do NOT change
what `Swarm::issue` does.

## The file assignment

| | |
|---|---|
| yours | `src/runtime/swarm-server/src/http.rs`, `src/runtime/swarm-server/src/swarm.rs`, `src/runtime/swarm-server/tests/redelivery_under_attack.rs`, `src/web/src/runtime.ts` |
| not yours | `src/runtime/swarm-server/src/state.rs` — unit B owns it this wave. `src/web/src/components/**` — unit C. If you need a change there, write the patch to your scratch directory and name it in the report |

`src/runtime/ess-runtime/src/store.rs` is yours to READ. Change it only if a doc line there is
factually wrong, and say so in the report.

## What "done" looks like

1. `src/runtime/swarm-server/src/http.rs:39` no longer promises "retrying with the same one is a
   retry, not a second request". It says what the key does and what it does not.
2. `src/web/src/runtime.ts:271` carries the same corrected promise — it repeats the wrong one today.
3. `src/runtime/swarm-server/src/swarm.rs:324` (`Delivery::request`) agrees with both.
4. **A test pins the behaviour the doc now describes**, so the doc cannot drift back silently:
   two `issue()` calls with one request key and one `ActivateConfig` input answer `Ok("activated")`
   then `Ok("wrong-state")`, and a creating command issued twice under one key leaves two
   instances. `tests/redelivery_under_attack.rs` already holds two characterisation cases written
   for this story — read them first; extend rather than duplicate.
5. The message a test asserts names `story:request-key-is-not-idempotency` and says what to change
   when the behaviour changes.

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

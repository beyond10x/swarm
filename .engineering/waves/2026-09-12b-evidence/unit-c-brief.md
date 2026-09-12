# Unit C — story:uitable-emits-contract

## Identity

```
story:  story:uitable-emits-contract
branch: wave/2026-09-12b/uitable-emits   (already checked out for you)
base:   0c24ac0504020c89d91e0fb2c65db7eb2af54f00
```

Review your own change with `git diff 0c24ac0...HEAD`.

## The triple

```
worktree:  /home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912b-c
build dir: <that worktree>/target and <that worktree>/node_modules
scratch:   /home/timo/.cache/swarm-wave-2026-09-12b/unit-c/scratch
```

You are the web unit. You are still assigned a build directory: if anything you run invokes
`cargo`, it builds into the `target/` inside your own worktree and nowhere else.

## The decision, already taken

Read the story with `aep plan artifact show story:uitable-emits-contract`. It offers two acceptance
branches. **The operator chose: table panels go inert.**

So: correct the contract table, let `UiTable` join `uiEmitters`, and a `Box{kind: Ui,
ref_id: "UiTable"}` on the canvas renders read-only with the existing "Read-only: nothing can keep
what this component emits." note. Do NOT re-cut the emitter rule to distinguish an actionable emit
from one a panel could not act on.

## The facts, read on main at b66ef29

- `src/web/src/components/ui/UiTable.vue:15` — `const emit = defineEmits<{ 'row-click': [row: Row] }>()`, fired at `:41` and `:63`.
- `src/web/src/components/ui/index.ts:15-16` — the `UiTable` row's slots/emits column reads only `cell-<key>, empty`. No `emits`.
- `src/web/src/components/ui/index.ts:158-169` — `uiEmitters`, which `UiBox.vue:88` computes `inert` from.
- `src/web/src/components/ui/index.test.ts:47` — `tableOf`, the parser; used at `:105`, `:120`, `:183`.
- `src/web/src/views/ComponentsView.vue:46` — already documents `emits: 'row-click(row)'`, so two in-repo descriptions of `UiTable` contradict each other today.

## The file assignment

| | |
|---|---|
| yours | everything under `src/web/src/components/ui/` and `src/web/src/components/canvas/`, plus `src/web/src/lib/uibox.ts` and `src/web/src/views/SwarmCanvas.vue` if the change reaches them |
| not yours | `src/web/src/runtime.ts` — unit A owns it this wave. All of `src/runtime/**` — units A and B |

## What "done" looks like

1. The contract table declares `UiTable`'s `row-click`, and `uiEmitters` contains `UiTable`.
2. `index.test.ts` stays green, including the cases at `:120` and `:183`.
3. **A new test compares each component's `defineEmits` against the table's emits column**, so this
   class of drift fails rather than being found by an adversary. That is the test the story calls
   the useful part.
4. A canvas test asserts a `UiTable` box renders inert. The behaviour change is on purpose and a
   test says so.
5. Nothing else joins or leaves `uiEmitters` or `uiRefused` as a side effect. If your new test finds
   a second component whose emits disagree with the table, do NOT fix it — report it, with the
   file:line, as a finding for the coordinator.

## The gate

```
npm test          # in src/web
npx vue-tsc --noEmit
npx vite build
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

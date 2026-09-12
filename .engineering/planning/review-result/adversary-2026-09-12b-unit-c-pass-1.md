---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-c-pass-1
kind: review-result
status: active
title: Adversary, bug wave unit C, pass 1
relations:
- reviews: story:uitable-emits-contract
revision: 1
---
```
unit: story:uitable-emits-contract — worktree wave-20260912b-c, head ad7be5f (base 0c24ac0), plus one untracked test file of the adversary's
verdict: NEEDS-CHANGE
cases: executed 38→41, red 2
origin: introduced 2 / pre-existing 0 / undecided 2 (the two judgement rows)
wrote-outside-worktree: 2 paths (part 6)
needs-coordinator: the named fixes are not the adversary's to apply
```

Recorded by the coordinator from the adversary's returned report, unedited.

## 1. Diff — proof of what was touched

```
$ git --no-pager diff --stat
(empty)
$ git status --porcelain
?? src/web/src/components/ui/index.emits-guard.test.ts
```

One path, untracked, a test file. No implementation file touched.

## 2. The cases added

`index.emits-guard.test.ts` copies the component directory, rewrites `UiTag`'s emit into another
Vue-supported spelling, walks the contract back to match (row's emits column cleared, `UiTag`
dropped from `uiEmitters`) and runs the shipped, unmodified `index.test.ts` over it. That is the
`UiTable` defect resurrected. It asserts the child suite goes red.

| case | now |
|---|---|
| CONTROL: tuple spelling `defineEmits<{ remove: [] }>()` | green — the harness can observe a red child |
| runtime array `defineEmits(['remove'])` | **red** |
| call-signature `defineEmits<{ (e: 'remove'): void }>()` | **red** |

```
$ node --test --experimental-strip-types src/components/ui/index.emits-guard.test.ts
ok 1 - CONTROL: the drift guard catches a component walked out of the contract (tuple spelling)
not ok 2 - the drift guard catches a component that emits with the runtime array form
    # tests 8
    # pass 8
    # fail 0
not ok 3 - the drift guard catches a component that emits with the call-signature form
    # tests 8
    # pass 8
    # fail 0
# tests 3
# pass 1
# fail 2
```

The nested `# pass 8 / # fail 0` is the finding: all eight contract checks, including the new one,
pass over a library where a component emits and the table says it does not.

## 3. The suite, after the cases existed

```
$ npm test            # src/web
1..41
# tests 41
# pass 39
# fail 2
EXIT=1

$ npx vue-tsc --noEmit     exit 0
$ npx vite build           ✓ built in 2.02s    exit 0
```

38 → 41 executed, 2 red. `tsconfig.json` excludes `src/**/*.test.ts`.

## 4. Findings

**F1 — the drift guard fails open on two of the three `defineEmits` spellings.**
`index.test.ts:229`. `emitsOf` matches only `defineEmits<{ name: [...] }>`; the array form and the
call-signature form yield `[]`, which the guard reads as "emits nothing" — the state `UiTable` was
in. Measured at `index.emits-guard.test.ts:89` and `:99`, child exit 0 with 8/8 green. What reaches
it: nothing in the tree today; all 12 emitting components use the tuple form. The guard exists for
the component added next. Fix: fail closed, or parse with `@vue/compiler-sfc`'s `compileScript`,
already present via `@vitejs/plugin-vue`.

**F2 — the new canvas case asserts source text, not the behaviour it names.**
`uibox.inert.test.ts:168-170` checks that `UiBox.vue` contains `uiEmitters.has(spec.value.component)`
and the note string. Measured on a scratch copy: with `<div :inert="inert || undefined">` replaced
by `<div>`, the whole suite is 38/38 green; with the note's `v-if` forced false, likewise. Both
halves of "renders inert, and says so" survive deletion untested. Fix: assert against the compiled
template.

**F3 (judgement, no case)** — `uibox.inert.test.ts:132` states "the rows still render and still
read". `inert` removes the subtree from the accessibility tree, find-in-page and text selection, so
a table panel's data is unreadable to assistive tech and uncopyable, and "Read-only" implies
otherwise.

**F4 (judgement, no case)** — `UiTable.vue:70` evaluates `!rows.length`. A `Box{kind: Ui, ref_id:
'UiTable', props: {}}` passes no `rows` unless the box names a view, so the render dereferences
`undefined`. Read, not run. Pre-existing to this diff.

## 5. Attacked, could not break

- The emits column of all 21 rows against every `defineEmits` in the directory: agrees.
- `ComponentsView.vue`'s per-component `emits:` meta: agrees for all 21, `UiTable` included.
- Nothing else joined or left `uiEmitters`/`uiRefused`: 12 emitting, 11 inert + `UiModal` refused.
- Deleting a table row, or desyncing `uiPropTypes`: caught by the existing prop-type check.
- Reflowing the `UiTable` row's whitespace so the emits column loses its 2-space separator: caught.

## 6. Paths written outside the worktree

- `~/.cache/swarm-wave-2026-09-12b/unit-c/scratch/inert-mutant/`
- `~/.cache/swarm-wave-2026-09-12b/unit-c/scratch/note-mutant/`

```findings
- file: src/web/src/components/ui/index.test.ts
  line: 229
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the new drift guard's emitsOf reads only the tuple-type defineEmits spelling, so a component emitting via the runtime array or call-signature form is recorded as emitting nothing and the whole contract suite stays green over the same defect this unit exists to prevent recurring.
- file: src/web/src/components/canvas/uibox.inert.test.ts
  line: 168
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the case asserting a UiTable box "renders inert, and says so" matches strings in UiBox.vue rather than the rendered attribute, and the suite stays 38/38 green with the :inert binding deleted or the read-only note disabled, so the chosen acceptance branch is unguarded.
- file: src/web/src/components/canvas/uibox.inert.test.ts
  line: 132
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: the comment's claim that inert rows "still render and still read" is false for assistive tech, find-in-page and text selection, which inert removes; a wording and documentation question inside the chosen branch, not testable here.
- file: src/web/src/components/ui/UiTable.vue
  line: 70
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: a UiTable box with no props and no view passes no rows, and the template dereferences rows.length during render; read rather than run, because this suite has no component runner.
```

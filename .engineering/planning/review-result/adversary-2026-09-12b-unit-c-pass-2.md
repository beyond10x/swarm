---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-c-pass-2
kind: review-result
status: active
title: Adversary, bug wave unit C, pass 2
relations:
- reviews: story:uitable-emits-contract
revision: 1
---
```
unit: story:uitable-emits-contract, unit C pass 2 — worktree wave-20260912b-c at 123210c plus three untracked test files
verdict: NEEDS-CHANGE
cases: executed 41→50, red 7
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: whether the accessibility cost of `inert` on `UiTable` (F4) is a new story or an accepted cost of the branch the operator chose
```

Recorded by the coordinator from the adversary's returned report. One change was forced on the
findings block below: F3's message begins with a backtick, which YAML reserves, so that message is
single-quoted. No word is altered.

## 1. `git --no-pager diff --stat`

```
(empty)
```

Three new untracked files; nothing tracked modified.

```
?? src/web/src/components/canvas/test-deps.test.ts
?? src/web/src/components/canvas/uibox.inert.mutants.test.ts
?? src/web/src/components/ui/index.emits-open.test.ts
```

`UiBox.vue` was mutated only in memory, inside a test.

## 2. The cases, and their red output

### 2a. `uibox.inert.mutants.test.ts`

Lifts the new AST predicate out of `uibox.inert.test.ts` and runs it over mutated `UiBox.vue`
strings held in memory. CONTROL asserts the deletion mutant the correction was written for is still
rejected — green. Three mutants that neuter `inert` while leaving the word in place — red.

```
ok 1 - CONTROL: the guard rejects UiBox with the `inert` binding deleted
not ok 2 - the guard rejects an `inert` binding that can never be true
  error: 'a binding whose value is `undefined` for every input passes the guard: the guard asks whether the word `inert` occurs in the expression, not whether the expression can ever inert anything'
not ok 3 - the guard rejects a note shown for the components that are not inert
  error: 'the note is shown for the complement of the inert set and the guard accepts it: a live panel claims to be display-only and an inert one explains nothing'
not ok 4 - the guard rejects an `inert` computed that is always false
  error: '`inert` is false for every box and the guard accepts it, because it matches the call and not the value the call decides'
1..4
# pass 1
# fail 3
```

Mutants, verbatim: `:inert="inert || undefined"` → `:inert="inert && undefined"`;
`v-if="inert && !refusal && !feed"` → `v-if="!inert && ..."`;
`uiEmitters.has(spec.value.component)` → `uiEmitters.has(spec.value.component) && false`.

### 2b. `index.emits-open.test.ts`

Runs the shipped `index.test.ts` unmodified over a mkdtemp copy of the `ui/` directory with `UiTag`
rewritten. CONTROL (tuple spelling, contract walked back) — green. Three spellings the corrected
guard does not see — red.

```
not ok 1 - the drift guard reads every event, not the ones before the first `}>` inside the declaration
  error: 'UiTag now emits `remove` AND `clear`, the table still says only `remove`, and the guard is satisfied: its regex stopped at the `}>` closing the payload type, so it compared a subset of the events against the full row and found them equal'
ok 2 - CONTROL: the corrected guard still catches the drift it was written for
not ok 3 - the drift guard catches a component that emits from its template with `$emit`
  error: 'a component that emits `remove` with the template `$emit` Vue documents contains no `defineEmits` token at all, so the fail-closed assertion never fires'
not ok 4 - the drift guard catches a component that declares its emits with `defineOptions`
  error: 'the emits are DECLARED, in a compiler macro sitting beside `defineEmits` in the same file, and the guard reads the component as one that emits nothing — the fourth spelling its own comment promises to fail loudly on'
1..4
# pass 1
# fail 3
```

In every red case the child printed `# fail 0` — all 8 shipped checks green while `UiTag` emitted an
event the table did not name.

### 2c. `test-deps.test.ts`

```
not ok 1 - every package the test suite imports is declared in package.json
  + [
  +   'src/components/canvas/uibox.inert.mutants.test.ts imports @vue/compiler-sfc',
  +   'src/components/canvas/uibox.inert.test.ts imports @vue/compiler-sfc'
  + ]
  - []
```

## 3. The suite

With the adversary's three files deselected: `# tests 41 / # pass 41 / # fail 0`, exit 0.

```
$ npm test
1..50
# tests 50
# pass 43
# fail 7
EXIT=1

$ npx vue-tsc --noEmit -p tsconfig.json     EXIT=0
```

`npx vite build` not run: no test file is reachable from the app entry.

## 4. Findings

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | `index.test.ts:262` | NEEDS-CHANGE | introduced | the fail-closed door is keyed on the token `defineEmits`. A component emitting via template `$emit('remove')`, or declaring `defineOptions({ emits: ['remove'] })`, contains no such token, so the guard reports "emits nothing" and `UiBox` renders it live | any component added with Vue's documented template `$emit` or `defineOptions({emits})`. None today; the guard is future-facing and its own comment claims a fourth spelling fails loudly, which is false |
| F2 | `index.test.ts:245` | NEEDS-CHANGE | introduced | the non-greedy regex stops at the first `}` followed by `>`, so `defineEmits<{ remove: [reason: Partial<{ why: string }>], clear: [] }>()` hides `clear`; `names` is non-empty so fail-closed never fires | constructed; no component uses such a payload today. The direction is the harmful one — a silent under-read |
| F3 | `package.json:13` | NEEDS-CHANGE | introduced | `uibox.inert.test.ts:178` imports `@vue/compiler-sfc`, declared nowhere; it resolves only because npm hoists it out of `vue` | `npm test` is the package gate. Under `--omit=dev`, a strict store, or a `vue` release that drops the dependency, the gate dies with ERR_MODULE_NOT_FOUND instead of failing a check |
| F4 | `UiBox.vue:162` | INFEASIBLE | introduced | the new note is accurate, which is the problem: `UiTable` in `uiEmitters` makes a canvas data table unreadable by a screen reader, unfindable by Ctrl-F and uncopyable | the operator chose the inert branch. A cheaper mechanism exists for a later story: no listener is bound by `<component :is>` anyway, so `pointer-events: none` plus `aria-disabled` would stop input without removing the subtree |

The two things the implementor flagged, judged: `npm test` needing `node_modules` is fine, and
importing an undeclared package is F3. The `mkdtemp` under TMPDIR is not a defect — unique name,
removal in a `finally`, `tmpdir()` honours `TMPDIR`.

## 5. Attacked, could not break

- The glob: `files` is built from `exported`, pinned to the `uiComponents` keys, so a `.vue` the guard never sees is one no `Box` can name.
- The three `emitsOf` spellings on realistic declarations, including `'update:modelValue': [v: string]` and nested payloads closed by `]`.
- The note-element finder picks the `<p class="note">` and not an ancestor.
- `:inert="inert || undefined"` renders correctly under `inheritAttrs: false`.
- `uiEmitters`/`uiRefused` membership: `UiTable` joined, nothing else moved, `UiModal` still refused.
- The new note wording is factually correct about what `inert` removes.

## 6. Paths written outside the worktree

- `~/.cache/swarm-wave-2026-09-12b/unit-c/scratch/adv2-emits-open.log`

```findings
- file: src/web/src/components/ui/index.test.ts
  line: 262
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the fail-closed assertion keys on the token `defineEmits`, so a component emitting via template `$emit` or declaring `defineOptions({emits})` is read as emitting nothing and is rendered live while its events are discarded, contradicting the guard's own comment that a fourth spelling fails loudly.
- file: src/web/src/components/ui/index.test.ts
  line: 245
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the non-greedy brace-angle terminator in the type-literal regex ends the captured declaration at an inline object closing a generic payload type, so events declared after it are silently dropped and a subset is compared equal to the full table row.
- file: src/web/src/components/canvas/uibox.inert.test.ts
  line: 195
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the AST guard asserts only that the word `inert` occurs in the binding, the note's v-if and the computed, so `inert && undefined`, an inverted note condition and `uiEmitters.has(...) && false` all pass while every emitter panel is interactive or the note is shown for the complement of the inert set.
- file: src/web/package.json
  line: 13
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '`uibox.inert.test.ts` imports `@vue/compiler-sfc`, which the manifest declares in neither dependencies nor devDependencies and which resolves only through npm hoisting it out of `vue`, so the `npm test` gate breaks under a strict store or `--omit=dev`.'
- file: src/web/src/components/canvas/UiBox.vue
  line: 162
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: adding UiTable to uiEmitters makes a data table on the canvas invisible to assistive technology, unfindable and uncopyable, which the new note correctly admits; the operator chose the inert branch so it cannot be fixed here, but a listener is never bound anyway and pointer-events plus aria-disabled would cost no accessibility.
```

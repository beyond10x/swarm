---
format: aep.planning-md/1
id: story:uitable-emits-contract
kind: story
status: implemented
title: UiTable emits what the contract does not declare
summary: The component library's contract table disagrees with UiTable, and correcting it changes canvas behaviour.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/web/package.json
- confidence: inferred
  path: src/web/src/components/canvas/UiBox.vue
- confidence: cited
  path: src/web/src/components/canvas/uibox.inert.guard.ts
- confidence: inferred
  path: src/web/src/components/canvas/uibox.inert.test.ts
- confidence: cited
  path: src/web/src/components/ui/UiTable.vue
- confidence: cited
  path: src/web/src/components/ui/index.test.ts
- confidence: cited
  path: src/web/src/components/ui/index.ts
- confidence: inferred
  path: src/web/src/lib/uibox.ts
- confidence: inferred
  path: src/web/src/views/SwarmCanvas.vue
revision: 9
---
## What

`src/web/src/components/ui/index.ts` is a contract: a comment table declaring every component's
props, slots and emits, and the 21 named re-exports beside it. `UiTable` emits `row-click` and the
table does not say so.

Found 2026-09-12 by unit B of that day's wave, which needed the emits column as data: it renders a
component that emits inside `:inert`, because a canvas panel cannot keep what a component emits.
`UiTable` is therefore treated as non-emitting, which happens to be true today only because
`<component :is>` binds no listeners at all.

## Why it was not fixed in that wave

Correcting the table is not a documentation change. The drift test parses the table, and a
`UiTable` row gaining `emits row-click` would put it in the emitter set and make every table panel
inert — a behaviour change nobody asked for, in the middle of a story whose acceptance is that a
table box shows a view's rows.

## Acceptance

The contract table and `UiTable.vue` agree about what it emits, and whatever that agreement implies
for the canvas is decided on purpose rather than inherited: either a table panel is inert like every
other emitter, or the emitter rule distinguishes an emit that a panel could act on from one it
cannot.

## Notes

The drift test that found this is the useful part and should stay: it parses the table and fails
when the lookup and the re-exports disagree. It is the reason this was discovered rather than
assumed.

## Scope

Rewritten 2026-09-12 from the implementor's confirmation after the unit merged.

**What the unit actually touched**, read from `git diff 0c24ac0...wave/2026-09-12b/uitable-emits`:

| path | the mark it carried | what it turned out to be |
|---|---|---|
| `src/web/src/components/ui/index.ts` | cited | touched. The `UiTable` row's emits column, and `UiTable` joining `uiEmitters` |
| `src/web/src/components/ui/index.test.ts` | cited | touched. The drift guard: three `defineEmits` spellings, then five emitting markers, brace-balanced reads, fail-closed |
| `src/web/src/components/ui/UiTable.vue` | cited | **not touched.** It was already correct; the contract was the wrong half |
| `src/web/src/components/canvas/UiBox.vue` | **inferred** | **right.** The note's wording changed; the `inert` binding did not |
| `src/web/src/components/canvas/uibox.inert.test.ts` | **inferred** | **right.** Rewritten twice — source-text assertion, then AST tokens, then evaluating the expressions |
| `src/web/src/lib/uibox.ts` | **inferred** | **wrong.** The inert decision lives in `UiBox.vue`; this file carries the panel/refused decision and needed nothing |
| `src/web/src/views/SwarmCanvas.vue` | **inferred** | **wrong.** It carries no part of the emitter or inert rule |
| `src/web/src/components/canvas/uibox.inert.guard.ts` | not scoped | **new file.** The evaluating predicate, imported by both the suite and the adversary's mutant file so the harness cannot certify a copy |
| `src/web/package.json`, `package-lock.json` | not scoped | touched. `@vue/compiler-sfc` declared; it had been resolving only through npm hoisting it out of `vue` |
| three `*.test.ts` from the adversary | not scoped | **new files**, kept: `index.emits-guard.test.ts`, `index.emits-open.test.ts`, `test-deps.test.ts` |

Two of the four inferred paths were wrong, and both were conditional on the acceptance branch the
operator did not take. The scoper said so at the time.

- **Confidence:** settled. Read from the merged diff.
- **Collides with:** anything under `src/web/src/components/ui/` or `src/web/src/components/canvas/`,
  and now `src/web/package.json`. `lib/uibox.ts` and `views/SwarmCanvas.vue` are **not** this
  story's surface.

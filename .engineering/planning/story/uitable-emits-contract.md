---
format: aep.planning-md/1
id: story:uitable-emits-contract
kind: story
status: active
title: UiTable emits what the contract does not declare
summary: The component library's contract table disagrees with UiTable, and correcting it changes canvas behaviour.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/web/src/components/canvas/UiBox.vue
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
revision: 6
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

Derived 2026-09-12 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/web/src/components/ui/index.ts` — cited, the story names it as the contract whose table is wrong
- **Files:** `src/web/src/components/ui/index.ts:15` (the `UiTable` table row), `src/web/src/components/ui/index.ts:158` (`uiEmitters`) — cited
- **Files:** `src/web/src/components/ui/UiTable.vue:15` — cited, the `defineEmits` is there
- **Files:** `src/web/src/components/ui/index.test.ts` — cited, the story requires the drift test stay
- **Symbols:** `uiEmitters`, `uiRefused`, `defineEmits<{ 'row-click': [row: Row] }>` — cited
- **Also likely:** `src/web/src/components/canvas/UiBox.vue` and `src/web/src/components/canvas/uibox.inert.test.ts` — inferred, both read `uiEmitters` (`UiBox.vue:88` computes `inert` from it)
- **Also likely:** `src/web/src/lib/uibox.ts` and `src/web/src/views/SwarmCanvas.vue` — inferred, they carry the panel/refused decision
- **Documents:** none — cited, the contract table is a comment inside `index.ts`
- **Confidence:** high — the story names the file, the component and the behaviour, and the tree confirms the emit and the emitter set at those lines
- **Would collide with:** any unit touching `src/web/src/components/ui/index.ts`, its drift test, or the canvas inert rule in `src/web/src/components/canvas/`

Not established: which acceptance branch is taken. "Table panel goes inert" touches only the three
cited files; "the emitter rule distinguishes an actionable emit" rewrites `UiBox.vue` and its inert
test, so the four inferred paths are conditional on a decision the story leaves open.

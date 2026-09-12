---
format: aep.planning-md/1
id: story:inert-costs-a-table-its-readers
kind: story
status: draft
title: An inert panel is unreadable, not merely uninteractive
summary: inert removes a canvas table from assistive tech, find-in-page and selection.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/web/src/components/canvas/UiBox.vue
- confidence: cited
  path: src/web/src/components/canvas/uibox.inert.test.ts
- confidence: inferred
  path: src/web/src/components/ui/index.ts
- confidence: inferred
  path: src/web/src/styles
revision: 3
---
## What

`story:uitable-emits-contract` put `UiTable` into `uiEmitters`, on the operator's decision of
2026-09-12, so a `Box{kind: Ui, ref_id: "UiTable"}` on the canvas renders inside `:inert`. The note
the unit wrote is accurate and that is the finding: `inert` removes the subtree from the
accessibility tree, from find-in-page and from text selection, so a data table drawn on a canvas is
unreadable by a screen reader, unfindable by Ctrl-F and uncopyable.

Reported 2026-09-12 by the adversary of unit C of the 2026-09-12b bug wave
(`review-result:adversary-2026-09-12b-unit-c-pass-2`, finding F4, `verdict: INFEASIBLE`,
`origin: introduced`). It is a cost of the branch the operator chose, not a defect in the unit that
carried the decision out, which is why it is a story rather than a correction.

**A cheaper mechanism was named and not evaluated.** `<component :is>` binds no listeners at all, so
an emitter panel's events already go nowhere. `pointer-events: none` plus `aria-disabled` would stop
input without removing the subtree from the accessibility tree. Whether that is enough for every
component in `uiEmitters` — eleven today, several of which are focusable — is the question, and the
answer has to cover keyboard focus, not only the pointer.

## Acceptance

A `UiTable` panel's rows can be read by a screen reader, found by find-in-page and selected, and no
panel in `uiEmitters` accepts an input whose effect would be discarded. A test asserts both halves,
and the rule that decides which mechanism a component gets is written down rather than inherited.

## Out of scope

Whether `UiTable` is an emitter. It is, it was measured, and `story:uitable-emits-contract` settled
it.

## Scope

- **Files:** `src/web/src/components/canvas/UiBox.vue` (the `inert` binding and the note) — cited
- **Files:** `src/web/src/components/canvas/uibox.inert.test.ts` — cited, it asserts the current mechanism
- **Also likely:** `src/web/src/components/ui/index.ts` (`uiEmitters`) — inferred, only if the rule that picks a mechanism changes the set
- **Also likely:** `src/web/src/styles/` — inferred, conditional on the `pointer-events` mechanism
- **Confidence:** medium — the mechanism is decided, the replacement is not, and nothing has measured what the eleven emitter components do under it
- **Would collide with:** `story:escapes-scan-fails-open` and `story:uitable-without-rows`, both of which land in the same two files

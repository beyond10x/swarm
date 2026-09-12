---
format: aep.planning-md/1
id: story:uitable-without-rows
kind: story
status: draft
title: A UiTable box with no rows and no view dereferences undefined
summary: The template reads rows.length, and a box that names no view passes no rows.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/web/package.json
- confidence: cited
  path: src/web/src/components/canvas/UiBox.vue
- confidence: inferred
  path: src/web/src/components/canvas/uibox.inert.test.ts
- confidence: cited
  path: src/web/src/components/ui/UiTable.vue
revision: 3
---
## What

`src/web/src/components/ui/UiTable.vue:70` evaluates `!rows.length` during render. A
`Box{kind: Ui, ref_id: "UiTable", props: {}}` passes no `rows` unless the box names a view
(`src/web/src/components/canvas/UiBox.vue:130-138`), so the render dereferences `undefined`.

Found 2026-09-12 by the adversary of unit C of the 2026-09-12b bug wave
(`review-result:adversary-2026-09-12b-unit-c-pass-1`, finding 4, `verdict: INFEASIBLE`,
`origin: undecided`; the coordinator routed it as pre-existing, because `UiTable.vue:70` predates
that unit's diff and the unit changed no render path).

**Read, not run.** The web suite has no component runner, so nobody has observed the render fail.
That is the first thing this story has to settle: whether a component runner is worth adding, or
whether the guard is added on the reading alone.

The same box shape is constructed by `uibox.inert.test.ts:156`, which calls it a panel and asserts
on it without rendering it.

## Acceptance

A `UiTable` box that carries no rows and names no view renders something a reader can act on rather
than throwing, and a test exercises that render rather than reading the template.

## Out of scope

The emitter and inert rules, settled by `story:uitable-emits-contract`.

## Scope

- **Files:** `src/web/src/components/ui/UiTable.vue:70` — cited, quoted by the adversary
- **Files:** `src/web/src/components/canvas/UiBox.vue:130` (where `rows` is bound, or not) — cited
- **Also likely:** `src/web/src/components/canvas/uibox.inert.test.ts` — inferred, it constructs the failing box shape today
- **Also likely:** `src/web/package.json` — inferred, a component runner would be a new dev dependency
- **Confidence:** medium — the line and the binding were read; the failure was not observed, because nothing can run a component here
- **Would collide with:** any unit touching `UiBox.vue` or the `components/ui/` library

---
format: aep.planning-md/1
id: review-result:adversary-unit-b-pass-2
kind: review-result
status: active
title: Adversary, unit B, pass 2
relations:
- reviews: story:render-a-ui-box
revision: 1
---
```
unit: B
verdict: red
cases: executed 24→27, red 2
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: seven p2-*.log files under the assigned scratch directory
needs-coordinator: no
```

Adversary pass 2 against `wave/2026-09-12/ui-box` at `a626f38`. One new test file,
`src/web/src/components/canvas/uibox.inert.test.ts`. No implementation file touched.

| gate | exit | result |
|---|---|---|
| `npm test` | 1 | `# tests 27  # pass 25  # fail 2` |
| `npx vue-tsc --noEmit` | 0 | clean |
| `npx vite build` | 0 | built |
| `ess specify validate --path src/core` | 0 | valid |
| `check-sets-are-emitted.py` | 0 | every written field emitted |

With the new file deselected the suite is 24 passed, exit 0.

Attacked and not broken: the comment-table parser, hand-walked over all 21 rows including
continuation lines, unions, optional markers, slot-only tails and the two-line `UiStateBadge` prose —
no row read wrongly, and both halves are right rather than merely agreeing. `columnsFrom` with
non-uniform rows is unreachable, because `views.rs:134` inserts every declared field into every row,
so pass 1 was right to let it stand. The refusal branch handles wrong case and surrounding whitespace
correctly. `UiTable`'s `row-click` offers nothing today, as the implementor claimed.

```findings
- file: src/web/src/components/canvas/UiBox.vue
  line: 139
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: UiBox claims ":inert is the whole mechanism" for every component in uiEmitters, but UiModal teleports to document.body and installs a document-level keydown handler, so a Box{kind:Ui, ref_id:"UiModal", props:{open:"true"}} renders an unclosable full-screen dialog that scroll-locks the page and swallows Tab and Escape application-wide.
- file: src/web/src/components/canvas/UiBox.vue
  line: 101
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a box naming a view the server does not have binds rows:[] and renders "No rows", indistinguishable from a view that exists and is empty, which is the same authoring mistake the unit built the unknown-component refusal branch to make visible for ref_id.
```

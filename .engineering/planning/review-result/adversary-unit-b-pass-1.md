---
format: aep.planning-md/1
id: review-result:adversary-unit-b-pass-1
kind: review-result
status: active
title: Adversary, unit B, pass 1
relations:
- reviews: story:render-a-ui-box
revision: 1
---
```
unit: B
verdict: red
cases: executed 12→18, red 6
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 9 paths under the assigned scratch directory
needs-coordinator: no
```

Adversary pass 1 against `wave/2026-09-12/ui-box` at `d5f267e`, base `a6861dd`. Two new test files,
no implementation file touched.

| gate | exit | result |
|---|---|---|
| `npm test` | 1 | `1..18  # pass 12  # fail 6` |
| `npx vue-tsc --noEmit` | 0 | clean |
| `npx vite build` | 0 | built |
| `ess specify validate --path src/core` | 0 | valid, 9 files |
| `check-sets-are-emitted.py` | 0 | every written field emitted and attributable |

Five of the six red cases assert that one prop arrives as the type `components/ui/index.ts` declares
for it, for a value a person would actually write. The sixth shows the documented refusal branch
cannot be selected.

Origin is `introduced` for all four findings: `git show a6861dd:src/web/src/lib/uibox.ts` and
`…:UiBox.vue` both exit 128, so neither file exists at the base.

Attacked and could not break: the drift test genuinely goes red under three separate mutations;
`tsconfig.json`'s exclude changes nothing, since `include` is already `src/**` and neither test file
is bundled; `SwarmView.vue`'s `:slug` reaches the one mount and nothing depended on its absence; the
view-refresh path holds through the live stream; `columnsFrom` reads only the first row's keys but no
non-uniform row set could be produced.

```findings
- file: src/web/src/lib/uibox.ts
  line: 92
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: decode() reinterprets any prop value that parses as JSON, so a UiBadge whose text is "404" is passed the number 404 and one whose text is "null" loses a required prop, against both components/ui/index.ts's declared prop types and the story's "with the props the box carries".
- file: src/web/src/components/canvas/UiBox.vue
  line: 96
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the documented refusal branch is unreachable because uiBoxSpec returns the same undefined for "not a Ui box" and "unknown component", and SwarmCanvas.vue:118 therefore never selects the ui node type for an unknown name.
- file: src/web/src/components/canvas/UiBox.vue
  line: 94
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: <component :is> binds props with no listeners while uiComponentNames admits every interactive component, so a Ui box naming UiTextInput renders an editable field whose edits are discarded, contradicting the UNMAPPED "no command changes props after DraftBox".
- file: src/core/domains/blackbox.yaml
  line: 231
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the Box.props comment says only non-scalar values are JSON-encoded, which tells the author scalars are carried as themselves, while the decoder reinterprets every value and the quoting convention appears nowhere an author reads.
```

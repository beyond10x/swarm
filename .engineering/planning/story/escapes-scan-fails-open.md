---
format: aep.planning-md/1
id: story:escapes-scan-fails-open
kind: story
status: draft
title: The escape scan decides refusal by four literal spellings
summary: A component that leaves its subtree by an unmatched spelling is inerted rather than refused.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/web/package.json
- confidence: cited
  path: src/web/src/components/canvas/uibox.inert.test.ts
- confidence: cited
  path: src/web/src/components/ui/index.test.ts
- confidence: cited
  path: src/web/src/components/ui/index.ts
revision: 3
---
## What

`src/web/src/components/ui/index.test.ts:157` holds `ESCAPES`, a regex matching
`<Teleport`, `document.`, `window.addEventListener` and `window.matchMedia`. Twin scans sit at
`src/web/src/components/canvas/uibox.inert.test.ts:53` and `:65`. They decide which components are
`uiRefused` — refused from a canvas panel entirely — as against merely inerted.

A component that leaves its own subtree by another spelling reads as safe: `globalThis.document`,
an aliased `const doc = document`, `el.ownerDocument`, or a composable that does it one call away.
Such a component is inerted rather than refused, which is the `UiModal` defect
(`review-result:adversary-unit-b-pass-2`, 2026-09-12) in a new spelling: a full-screen dialog that
scroll-locks the page and swallows Tab and Escape application-wide, from a `Box` an author drew.

Reported 2026-09-12 by unit C of the 2026-09-12b bug wave, in its correction round, under
`needs-coordinator`. The unit fixed the two guards the adversary broke and enumerated the rest of
the class; this is the one member it could not close.

**Why it was not fixed there.** The sibling guard `emitsOf` was made to fail closed: a component
whose source calls `defineEmits` and yields no event name is drift by definition. `ESCAPES` has no
equivalent, because there is no positive marker that says "this component escapes its subtree".
Closing it needs a different mechanism, and adding a fifth pattern is the hand-maintained list the
correction exists to stop trusting.

## Acceptance

A component that reaches outside its own subtree is refused from a canvas panel whatever spelling it
reaches by, and the check that decides this cannot pass by failing to recognise something. A test
demonstrates a component escaping by a spelling `ESCAPES` does not match, and that test is red
before the change.

## Notes

Two mechanisms were named and neither was evaluated: compile the template and look for a resolved
`Teleport`, or a lint rule banning a bare `document`/`window` reference in `components/ui/`. The
second catches more and costs a lint configuration; the first catches only teleports. Choosing
between them is the first work in this story.

## Scope

- **Files:** `src/web/src/components/ui/index.test.ts:157` (`ESCAPES`) — cited, quoted by the unit
- **Files:** `src/web/src/components/canvas/uibox.inert.test.ts:53`, `:65` (the twin scans) — cited
- **Files:** `src/web/src/components/ui/index.ts` (`uiRefused`, which these decide) — cited
- **Also likely:** `src/web/package.json` and an eslint configuration — inferred, conditional on which of the two mechanisms is chosen
- **Confidence:** high for the three cited files, which the unit read and reported line by line; the mechanism is undecided
- **Would collide with:** any unit touching the `components/ui/` contract or the canvas inert rule

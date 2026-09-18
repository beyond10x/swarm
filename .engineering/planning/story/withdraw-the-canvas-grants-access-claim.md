---
format: aep.planning-md/1
id: story:withdraw-the-canvas-grants-access-claim
kind: story
status: implemented
title: A drawn edge grants nothing, and the site said it did
summary: The canvas-as-access-control claim was published for the site's whole life and was never true of the code.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
- informed_by: story:run-a-tool-box
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: docs/index.md
- confidence: cited
  path: src/core/README.md
- confidence: cited
  path: website/scripts/spec-facts.mjs
- confidence: cited
  path: website/src/pages/index.js
revision: 5
---
## What

The published prose handed the canvas an access-control role: draw a connection to give a
coordinator something, delete it to take it back. **No code has ever behaved that way.**

`swarm.blackbox.Box` and `swarm.blackbox.Connection` are declared, created, folded and rendered,
and nothing under `src/runtime/` reads either to decide anything:

- `grep -rn 'LiveConnections' src/runtime/` selects nothing — the view exists only as a declaration
  at `src/core/domains/blackbox.yaml:1197`;
- no message travels a `Connection`, and a `Box` of kind `Tool` or `Service` "is declared, drawn,
  connected and executed by nothing" — `story:run-a-tool-box` says so outright;
- what a coordinator may actually reach is settled per call by the sealed `metaharness.frame/1`:
  `frame::ADMITTED` and `frame::scope`, whose only arguments are the agent's own work directory,
  the shared directory and an operation list intersected with `ADMITTED`. Neither consults the
  canvas, and a coordinator drawing on the canvas cannot move either.

Five files published it: `README.md`, `AGENTS.md`, `website/src/pages/index.js`,
`src/core/README.md` and `docs/index.md`. The last two were outside the first fix's reach and are
corrected here.

## What is kept

The canvas is a real declared model with a real purpose — it records what a swarm intends to reach
and how its pieces are meant to fit. Wiring as the grant is the design's *intent*. It is written as
intent, never as behaviour, until a runner exists (`story:run-a-tool-box`).

This does not touch the finding above it in `AGENTS.md`: the frame is still refusal at a decision
seam and still not containment, and correcting one must not blur the other.

## How a retired claim is held down

A claim is not a number. No derivation produces "the canvas does not grant access", so there is
nothing to compare against. What a check *can* do is refuse the sentence that says otherwise, which
is what `WITHDRAWN` plus `checkWithdrawn()` in `website/scripts/spec-facts.mjs` is: an absence
assertion, one entry per retired claim, naming the file, the line and the sentence when it fires.

Its limit, stated rather than left to be discovered: the patterns match the wordings that were
actually published and the obvious re-edits of them, not every paraphrase. They are anchored on the
assertive construction, so prose *about* the withdrawal does not trip them — which is what makes it
safe to explain the retraction in the same files. All three patterns were confirmed to fire on the
text `src/core/README.md` and `docs/index.md` carried before this change.

`checkWithdrawn()` runs before anything is derived: it needs no `ess`, no compile and no committed
file, so it holds in a bare checkout and on the build path as well as under `--check`.

## Acceptance

No published file states that drawing an edge grants access or that deleting one revokes it; a
fourth honesty rule in `AGENTS.md` records the finding and what replaced it; and
`npm run spec:check` fails, naming file, line and sentence, if the claim reappears in any of the
five files that carried it.

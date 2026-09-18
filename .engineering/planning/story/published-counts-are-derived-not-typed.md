---
format: aep.planning-md/1
id: story:published-counts-are-derived-not-typed
kind: story
status: implemented
title: A published count with no assertion, and an assertion that cannot fail
summary: 16 errors and 2 bindings were published against a compiler emitting 17 and 4; the prose check could not see either.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: docs/index.md
- confidence: cited
  path: src/core/README.md
- confidence: cited
  path: website/scripts/spec-facts.mjs
- confidence: cited
  path: website/src/data/spec-facts.json
- confidence: cited
  path: website/src/pages/index.js
revision: 5
---
## What

Two published counts were wrong, and the check that existed to catch them could not.

- `src/core/README.md` said **16 errors** and **2 bindings**; the compiler emits **17** and **4**.
  Both had been wrong since the line was written, in the file the root README sends readers to
  first and one of the four `b10x.docs.yaml` ships to `/docs/swarm/`.
- The demonstration measurement said *1 of 41 decided calls refused* in four places. The retained
  evidence says **42**.
- `docs/index.md` was never opened by the script at all.

## Why the check could not fail

`assertProse(rel, label, value)` tested whether the number appeared *anywhere* in the file. On a
line reading

    6 domains · 10 entities · 24 types · 53 commands · 56 events · 27 views · 17 errors ·
    12 actors · 4 components · 4 bindings · 4 workloads

asserting `bindings = 4` passes on the "4" in "4 components" even when the prose says 2. A check
that cannot fail is worse than no check, because it is believed. And `errors` and `bindings` had no
assertion at all, while `components` and `workloads` were not even derived — a count the script
cannot produce is a count the prose gets to keep wrong.

## What replaces it

`assertCount(rel, label, value, noun, within?)`: the number has to sit against its own noun, and
*every* place a file states a number against that noun has to agree — one right mention does not
excuse a wrong one elsewhere in the same file. `within` narrows the search to a passage, for the
places a file legitimately states an old number as history (`src/core/README.md` discusses an
earlier 11-domain draft, which is a fact about the past and not a claim this script may overwrite).

A fourth class of derived fact joins the three: `demonstration()` reads the two-agent run's
measurements off `spend.jsonl` and cross-checks them against the retained `checker-output.txt`,
which `swarm-check` wrote from the gitignored log. Two independent retained sources have to agree
or it fails — one alone could be edited by hand and nothing would notice, which is the defect this
whole script exists against.

## Acceptance

`npm run spec:check` derives every element class the compiler emits, opens `README.md`,
`AGENTS.md`, `CHANGELOG.md`, `src/core/README.md` and `docs/index.md`, and fails when any count
those files state against a derived noun disagrees with the derivation — including when a count is
stated nowhere at all. `assertProse` is gone. The site imports the demonstration's figures instead
of printing a typed one.

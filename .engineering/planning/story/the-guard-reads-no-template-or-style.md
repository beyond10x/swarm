---
format: aep.planning-md/1
id: story:the-guard-reads-no-template-or-style
kind: story
status: draft
title: The escape guard reads no template expression and no style block
summary: The script side fails closed; the template and style sides do not, by three measured routes.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/web/src/components/canvas/uibox.inert.test.ts
- confidence: cited
  path: src/web/src/components/ui/escapes.guard.adversary2.test.ts
- confidence: cited
  path: src/web/src/components/ui/escapes.guard.ts
- confidence: inferred
  path: src/web/src/components/ui/index.test.ts
revision: 5
---
## What

`src/web/src/components/ui/escapes.guard.ts` decides which components are refused from a canvas
panel. As of 2026-09-12 its **script** side fails closed: every construct by which a module names
another module is routed through one function, `reachModule`, which has no silent branch, and where
the guard cannot resolve something it produces a reason.

Its **template** and **style** sides do not. The template AST is walked for exactly two tag names,
`/^teleport$/i` and `/^component$/i` (`escapes.guard.ts:259`); no template expression is ever parsed
(`:257`); and `descriptor.styles` is read by nothing (`:242`). Three routes out of a component's own
subtree follow, all measured 2026-09-12 by the adversary of unit B of the 2026-09-12d wave
(`review-result:adversary-2026-09-12d-unit-b-pass-2`, findings 1–3, all `CONFIRMED`, all
`origin: pre-existing` — the four-literal `ESCAPES` regex missed them too):

1. **`<div is="vue:Teleport" to="body">`.** `resolveComponentType` in `@vue/compiler-core`
   (`compiler-core.cjs.js:5580`) rewrites `tag` to the `is` value after `vue:` **before** the
   `isCoreComponent` lookup, so the component ships a real `Teleport`. The adversary proved this
   in-case with `compileTemplate`: the render function is `_createBlock(_Teleport, { to: "body" }, …)`.
   To the guard's walk it is a native `div`.
2. **A template expression.** `$root` and `$parent` are Vue public instance properties, usable in any
   template without being declared, imported or named in a script, so the free-identifier walk is
   never offered them. A component with no `<script>` at all escapes through
   `@click="$root.$el.ownerDocument.body.style.overflow = 'hidden'"`.
3. **An unscoped `<style>` block.** `<style>body { overflow: hidden }</style>` is `UiModal`'s scroll
   lock in CSS, outside any subtree `inert` covers.

## What reaches each

- Routes 1 and 2: any author of a component in `components/ui/`. **No shipping component does either
  today.**
- Route 3: **nothing found.** All 20 style blocks in `components/ui/` are `scoped`, checked
  2026-09-12. Nothing in the repository requires `scoped`, there is no lint rule for it, and
  `<style module>` leaks element selectors globally even with the module flag.

## It is pinned, not forgotten

`src/web/src/components/ui/escapes.guard.adversary2.test.ts` holds the adversary's three cases,
rewritten to assert **today's** state — that the guard returns no reason for each of these three
spellings — with a failure message naming this story. Each case asserts first that the old
`FOUR_LITERALS` regex also misses the probe, so it cannot pass by measuring nothing, and case 1
compiles the template and asserts the generated code imports `Teleport as _Teleport`, so it cannot
pass because the escape was never real.

**They go red the moment any of the three routes is closed**, which is the signal to invert them.

## Acceptance

A component that reaches outside its own subtree through its template or its styles is refused, by
the same rule the script side already follows: where the guard cannot resolve something, it says so.
The three pinned cases are inverted and green. The enumeration in the guard's header is true — it
lists every route the guard does not catch, and nothing it does.

## Out of scope

The routes the header already declares and this story does not close: a DOM element handle reaching
the document through its own properties, property access beyond `PROTOTYPE_DOORS`, and a value handed
in as a prop or slot.

## Notes

Named fixes, from the adversary, none applied: treat an `is` attribute whose static value starts with
`vue:` as the tag when matching `teleport`, and produce a reason for any `is` the walk cannot resolve
to a supplied name; run `babelParse` over every `SimpleExpressionNode` in the template AST through the
same free-identifier walk with a Vue template globals allowlist, making `$root` and `$parent` reasons;
produce a reason for any `descriptor.styles` entry that is not `scoped`.

`story:escapes-scan-fails-open` is the parent defect and closed the script side. This is the rest of
the class, filed rather than folded in, because the wave's attack budget was two passes and both were
spent.

## Scope

- **Files:** `src/web/src/components/ui/escapes.guard.ts` (`:242`, `:257`, `:259`) — cited
- **Files:** `src/web/src/components/ui/escapes.guard.adversary2.test.ts` (the three pins) — cited
- **Also likely:** `src/web/src/components/ui/index.test.ts`, `src/web/src/components/canvas/uibox.inert.test.ts` — inferred, the two callers that read the reasons
- **Confidence:** high — every path was read and all three routes were measured red before being pinned
- **Would collide with:** any unit touching the `components/ui/` contract or the canvas inert rule

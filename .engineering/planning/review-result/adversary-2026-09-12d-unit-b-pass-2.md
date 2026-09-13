---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12d-unit-b-pass-2
kind: review-result
status: active
title: Adversary, wave d unit B, pass 2
relations:
- reviews: story:escapes-scan-fails-open
revision: 1
---
Adversary, wave 2026-09-12d unit B, pass 2. Against `story:escapes-scan-fails-open`, working tree
`/home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912d-b`, uncommitted, base `9afa0b8`.

## Header, as returned

```
unit: story:escapes-scan-fails-open, unit B pass 2
verdict: CONFIRMED (3 red cases)
cases: executed 68→71, red 3
origin: introduced 0 / pre-existing 3 / undecided 0
wrote-outside-worktree: 6 paths, all under the assigned scratch directory
needs-coordinator: no
```

## Cases added

Three, in `src/web/src/components/ui/escapes.guard.adversary2.test.ts`, the adversary's only written
path. Each asserts first that the pre-change `ESCAPES` regex misses the probe — so it measures a new
spelling — and then that `escapingComponents` returns a reason for it.

| case | the spelling |
|---|---|
| a component that teleports by the `is="vue:"` spelling is refused | `<div is="vue:Teleport" to="body">`. The case compiles the template with `compileTemplate` first and asserts the generated code imports `Teleport as _Teleport`, so the escape is proven real before the guard is asked |
| a component that escapes in a template EXPRESSION is refused | `<button @click="$root.$el.ownerDocument.body.style.overflow = 'hidden'">` — no script at all |
| a component that escapes through an unscoped `<style>` block is refused | `<style>body { overflow: hidden; }</style>` — `UiModal`'s scroll lock in CSS |

Each failed at its guard-reason assertion, not at the compile or `FOUR_LITERALS` precondition.

## The suite, each exit status read separately

```
npm test              # tests 71  # pass 68  # fail 3     exit 1
npx vue-tsc --noEmit                                      exit 0
npx vite build                                            exit 0
```

68 passing is exactly the implementor's declared 68: nothing pre-existing broke, and the three reds
are the adversary's own.

## Findings

**F1 — a Teleport spelling the guard does not recognise.** `escapes.guard.ts:259`.
`directReach` walks the template AST for exactly two tag names, `/^teleport$/i` and `/^component$/i`.
`<div is="vue:Teleport" to="body">` is a native `div` to that walk, so `escapingComponents` returns
`[]`.
*What reaches it*: `resolveComponentType` in the installed `@vue/compiler-core`
(`compiler-core.cjs.js:5580`) rewrites `tag` to the `is` value after `vue:` **before** the
`isCoreComponent` lookup, so the component ships the real `Teleport`. Compiled inside the case: the
render function is `_createBlock(_Teleport, { to: "body" }, …)`. Any author of a `components/ui/`
component can write it; **no shipping component does today.**
CONFIRMED / pre-existing — the base `ESCAPES` regex misses it too, asserted in the case.

**F2 — template expressions are never read as code.** `escapes.guard.ts:257`.
The template walk inspects tags only; not one expression is parsed. A component with no `<script>`
whatsoever, escaping entirely through `@click="$root.$el.ownerDocument…"`, returns `[]`.
*What reaches it*: `$root` and `$parent` are Vue public instance properties, usable in any template
without being declared, imported or named in a script, so the free-identifier walk is never offered
them. **No shipping component does it.**
*Why the declared edge does not cover it*: the header at `escapes.guard.ts:43-61` enumerates three
uncaught routes — DOM handles, property access past `PROTOTYPE_DOORS`, a value handed in as a
prop/slot. "The whole template expression language" is none of the three, and is larger than all
three.
CONFIRMED / pre-existing.

**F3 — `<style>` blocks are read by nothing.** `escapes.guard.ts:242`.
The block loop covers `descriptor.script`, `descriptor.scriptSetup`, `descriptor.template`;
`descriptor.styles` is never touched. A component shipping `<style>body { overflow: hidden }</style>`
— `UiModal`'s scroll lock in CSS, outside any subtree `inert` covers — returns `[]`.
*What reaches it*: **nothing found today** — all 20 style blocks in `components/ui/` are `scoped`,
checked. Nothing in the repository requires `scoped`, there is no lint rule for it, and
`<style module>` leaks element selectors globally even with the module flag.
CONFIRMED / pre-existing, warning.

**F4 — `PROTOTYPE_DOORS` is a two-name denylist inside an allowlist.** `escapes.guard.ts:126`.
The file's thesis is that a denylist cannot fail closed. The adversary attacked it and **could not
break it**: no third property name turns an `INTRINSICS` member back into `Function` without spelling
`constructor` or `__proto__`. A shape observation, not a defect. The header already states the
closure condition.
INFEASIBLE / introduced, note.

## Attacked and could not break

- **The five pass-1 cases pass for the right reason**, not because the guard refuses everything. Each
  names its own cause: `import('…')` → "imports at runtime `@/composables/useScrollLock`";
  `import * as vue` → "imports all of `vue`, which binds Teleport, createApp, render";
  `Object.constructor` → "reads `.constructor`".
- **The "does not simply refuse everything" control is real**: over the whole directory only
  `UiModal.vue`, `escapes.guard.ts` and `index.ts` (transitively) get reasons; every other component
  returns `[]`.
- **`reachModule`'s script-side coverage held** against `export * from`, `export {x} from`,
  `export * as ns from`, `import x = require()`, `import()` with a template literal (correctly a
  reason — no literal), `import.meta`, `new Worker`, `require()`,
  `defineAsyncComponent(() => import(…))`, type-only imports, a `<script src=>` whose target is and
  is not supplied, and an unparseable script. **No silent branch in the script path.**
- **The caller path**: a reason does reach `uiRefused` — `index.test.ts:195` deep-equals the declared
  set against the computed one, so a new escaping component fails the suite rather than being dropped.

```findings
- file: src/web/src/components/ui/escapes.guard.ts
  line: 259
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "A component that mounts outside its subtree by the native-element spelling `<div is=\"vue:Teleport\" to=\"body\">` compiles to a real Teleport (proved in-case by compileTemplate) and the guard returns no reason, so the acceptance clause \"whatever spelling it reaches by\" does not hold for the very construct the template walk exists to catch."
- file: src/web/src/components/ui/escapes.guard.ts
  line: 257
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "The template AST is walked for tag names only and no template expression is ever parsed, so a component with no script at all escapes through `@click=\"$root.$el.ownerDocument.body.style.overflow='hidden'\"` with no reason; this is a fourth uncaught route and the header's enumeration at :43 claims there are exactly three."
- file: src/web/src/components/ui/escapes.guard.ts
  line: 242
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "descriptor.styles is read by nothing, so an unscoped `<style>body { overflow: hidden }</style>` performs UiModal's scroll lock in CSS with no reason produced; nothing reaches it today because all 20 shipping style blocks are scoped, but nothing requires them to be and the header's enumerated edge does not mention styles."
- file: src/web/src/components/ui/escapes.guard.ts
  line: 126
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "PROTOTYPE_DOORS is a two-name hand-maintained denylist inside a file whose thesis is that a denylist cannot fail closed; I could not find a third property name that reaches Function from an INTRINSICS member, so this is a shape observation rather than a demonstrated hole."
```

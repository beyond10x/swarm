---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12d-unit-b-pass-1
kind: review-result
status: active
title: Adversary, wave d unit B, pass 1
relations:
- reviews: story:escapes-scan-fails-open
revision: 1
---
Adversary, wave 2026-09-12d unit B, pass 1. Against
`story:escapes-scan-fails-open`, working tree
`/home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260912d-b`, branch
`wave/2026-09-12d/unit-b`, uncommitted, base `9afa0b8`.

## Header, as returned

```
unit: story:escapes-scan-fails-open
verdict: NEEDS-CHANGE
cases: executed 63→68, red 5
origin: introduced 4 / pre-existing 0 / undecided 1
wrote-outside-worktree: none
needs-coordinator: no
```

## Cases added

Five, all in `src/web/src/components/ui/escapes.guard.adversary.test.ts`, the adversary's only
written path. No implementation file touched, no existing case deleted, skipped or weakened.

| # | asserts | now |
|---|---|---|
| 1 | a component reaching a module by `await import('@/composables/useScrollLock')` gets a reason | red |
| 2 | `escapes.guard.ts` itself — a real file in the directory both callers scan — reaches `@vue/compiler-sfc` by `await import(...)` and gets a reason | red |
| 3 | `<script setup src="./UiProbeLogic.ts">` gets a reason (its whole script is unread) | red |
| 4 | `import * as vue from 'vue'; vue.createApp(...); vue.render(...)` gets a reason | red |
| 5 | `Object.constructor('return globalThis')().document` gets a reason | red |

Every failure is `ERR_ASSERTION` / `notDeepStrictEqual`; none is a compile or import error.

## The gate, after the cases exist

```
npm test           # tests 68  # pass 63  # fail 5     exit 1
npx vue-tsc --noEmit                                   exit 0
npx vite build                                         exit 0
```

Type-check and build stay green, so none of the five is a syntax or typing artefact.

## Findings

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | `escapes.guard.ts:190` | `directReach` reads only `ImportDeclaration`. `import('x')` is a `CallExpression` on an `Import` callee: no `source` for the import branch, no identifier for `walkIdentifiers`. `escapingComponents` returns `[]` for a component that awaits any module | Not constructed — `escapes.guard.ts` itself is in the directory both callers scan and reaches `@vue/compiler-sfc` four times by `await import(...)`; the guard reports it clean | CONFIRMED | introduced |
| 2 | `escapes.guard.ts:135` | `scriptOf` joins `descriptor.script?.content` and `descriptor.scriptSetup?.content` and never reads `block.src`. For `<script setup src="./UiProbeLogic.ts">` the script is `''`, `directReach` returns early at the `!script.trim()` guard, reasons `[]` | `src=` on an SFC block is a supported Vue form the callers pass straight through; `index.test.ts` reads every `.vue` in the directory with `readFileSync`, so a component written this way is scanned and passes. Rare in practice | CONFIRMED | introduced |
| 3 | `escapes.guard.ts:195` | The vue branch reads `one.imported`, which only a named specifier has. `ImportNamespaceSpecifier` has none, so `import * as vue from 'vue'` binds `Teleport`, `createApp` and `render` under a local the free-identifier walk then treats as declared | Any author of a component in `components/ui/`; `VUE_ESCAPES` exists precisely to stop these three, and the namespace form is ordinary Vue | CONFIRMED | introduced |
| 4 | `escapes.guard.ts:78` | `Object` is allowlisted. `Object.constructor` is `Function`; `Object.constructor('return globalThis')().document` yields `[]` | **Nothing found** — no shipping component does this and no plausible author writes it. Adversarial construction, built by the adversary | INFEASIBLE | introduced |
| 5 | `escapes.guard.ts:36` (header) | The header names exactly one uncaught route (DOM element handles). Findings 1–3 are three further routes it does not mention, and finding 1 is exercised by the guard's own source | The declaration is what a reader of `index.ts:155-160` and of the story's acceptance relies on | NEEDS-CHANGE | introduced |

Named fixes, not applied: treat a `CallExpression` whose callee is `Import` as a reason unless its
argument is a literal that resolves in `supplied`; treat a non-empty `block.src` on
`script`/`scriptSetup`/`template` as a reason unless the named file is supplied; in the vue branch,
treat an `ImportNamespaceSpecifier` on `vue` as importing all of `VUE_ESCAPES`.

## Attacked and could not break

- **Transitive sibling laundering** — `escapingComponents` closes reach over `./` imports correctly, including cycles (`seen`).
- **Parse-failure path** — genuinely fail-closed; the `try/catch` pushes a reason and nothing upstream swallows it.
- **Relative-import resolution** — `candidates()` fails closed on `.`, `../`, and any unsupplied name.
- **Template `v-if` / `v-for` hiding a `<Teleport>`** — the untransformed parse AST keeps the tag visible to the recursive walk.
- **`<component :is="'Teleport'">`** — `resolveDynamicComponent('Teleport')` does not resolve a built-in.
- **Unicode-escaped identifiers (`document`)** — babel normalises the name, still caught.
- **`Function`, `Reflect`, `Proxy`, `eval`, `require`, `console`, `fetch`** — all absent from `INTRINSICS` and correctly reported.
- **The four added cases measuring nothing** — they do not; the `FOUR_LITERALS` assertion and the "does not simply refuse everything" case pin both directions.
- **Template expressions naming globals** — Vue prefixes unknown template identifiers with `_ctx.`.

## Verdict on the acceptance

The direction is right and the fail-closed cases that survived do fail closed. "The check that decides
this cannot pass by failing to recognise something" is **not yet true**: it fails to recognise
`import()`, `src=` blocks and namespace imports, and the first is demonstrated against a file already
sitting in the scanned directory.

```findings
- file: src/web/src/components/ui/escapes.guard.ts
  line: 190
  category: mutant
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a dynamic `import()` carries no ImportDeclaration and no free identifier, so a component can load any module and be reported as staying in its own subtree — demonstrated against escapes.guard.ts itself, which reaches @vue/compiler-sfc that way and is reported clean.
- file: src/web/src/components/ui/escapes.guard.ts
  line: 135
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: scriptOf never reads a block's `src` attribute, so a component whose entire script is `<script setup src="./x.ts">` walks an empty script and returns no reasons, which is unknown reading as safe.
- file: src/web/src/components/ui/escapes.guard.ts
  line: 195
  category: mutant
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: the vue branch inspects `imported`, which an ImportNamespaceSpecifier lacks, so `import * as vue from 'vue'` restores Teleport, createApp and render under a local the free-identifier walk treats as declared.
- file: src/web/src/components/ui/escapes.guard.ts
  line: 78
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the allowlist is not closed under property access — `Object.constructor` is `Function` and one call from every withheld global — but no shipping or plausible component reaches it, so the state is one I constructed.
- file: src/web/src/components/ui/escapes.guard.ts
  line: 36
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the header declares DOM element handles as the guard's single uncaught route, and three further routes exist, so the stated edge is wrong about what it covers.
```

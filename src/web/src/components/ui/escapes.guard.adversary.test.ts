// Adversary cases against `escapes.guard.ts` (wave 2026-09-12d, unit B, pass 1).
//
// The acceptance says the check that decides refusal "cannot pass by failing to recognise
// something". The guard answers that by allowlisting what a component may NAME. These cases ask
// what a component can reach WITHOUT naming anything — the ways a module gets another module, or
// another global, through syntax that produces no free identifier and no `ImportDeclaration`.
//
// Every probe below leaves its own subtree exactly as `UiModal` does. Each asserts the guard gives
// a reason for it. Written before any run; no existing case is touched.
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'

import { escapingComponents } from './escapes.guard.ts'

const HERE = new URL('.', import.meta.url)

/** The same directory-as-text the two shipping callers hand the guard. */
const directory: Record<string, string> = Object.fromEntries(
  readdirSync(HERE)
    .filter((name) => !name.endsWith('.test.ts'))
    .map((name) => [name, readFileSync(new URL(`./${name}`, HERE), 'utf8')]),
)

async function reasonsFor(probe: string, extra: Record<string, string> = {}): Promise<string[]> {
  const probed = await escapingComponents({ ...directory, ...extra, 'UiProbe.vue': probe })
  return probed['UiProbe.vue'] ?? []
}

test('a component that reaches another module by `import()` is refused', async () => {
  // `directReach` reads `ImportDeclaration` nodes and free identifiers. A dynamic import is
  // neither: `import('x')` is a CallExpression on an `Import` callee, so it carries no identifier
  // for `walkIdentifiers` to see and no `source` for the import branch to read. A component can
  // therefore load any module in the repository, or any package, and be reported as staying in its
  // own box — the same lazily-loaded scroll lock the story names, spelled with an `await`.
  const probe = `<script setup lang="ts">
const { lockScroll } = await import('@/composables/useScrollLock')
lockScroll()
</script>
<template><div /></template>`
  assert.notDeepEqual(
    await reasonsFor(probe),
    [],
    'a dynamic import reaches a module the guard never resolved, and reads as a component that stays inside itself',
  )
})

test('a file already in this directory reaches outside it by `import()` and is reported clean', async () => {
  // Not a constructed fixture: `escapes.guard.ts` is a real file in the directory both callers
  // hand to `escapingComponents`, and it reaches `@vue/compiler-sfc` — not `vue`, not a sibling —
  // four times, by `await import(...)`. The static form of the same line is a reason
  // ("imports `@vue/compiler-sfc`, which is outside this directory and not the framework"). That
  // the guard exempts itself from its own rule is the demonstration that the rule is spelling-
  // sensitive, which is the property the story says it must not be.
  const guard = directory['escapes.guard.ts']
  assert.match(guard, /await import\('@vue\/compiler-sfc'\)/)
  const reach = await escapingComponents(directory)
  assert.notDeepEqual(
    reach['escapes.guard.ts'],
    [],
    'a file reaching a non-framework package is an escape however the import is spelled; a sibling importing this one would inherit the silence',
  )
})

test('a component whose script lives in a `src=` block is refused or read', async () => {
  // `<script setup src="./x.ts">` is a supported SFC form: the block is empty and its content is
  // another file. `scriptOf` joins `descriptor.script?.content` and `descriptor.scriptSetup?.content`
  // and never looks at `block.src`, so the script it walks is the empty string and `directReach`
  // returns early with no reasons. Everything the component actually does is in a file the guard
  // was neither given nor asked for — the state the header calls "unknown", which must not read as
  // safe.
  const probe = `<script setup lang="ts" src="./UiProbeLogic.ts"></script>
<template><div /></template>`
  const logic = `export const doc = globalThis.document\ndoc.body.style.overflow = 'hidden'\n`
  assert.notDeepEqual(
    await reasonsFor(probe, { 'UiProbeLogic.ts': logic }),
    [],
    'a component whose entire script is an unread external block reads as one that declares nothing and reaches nothing',
  )
})

test('a component that mounts an app through a `vue` namespace import is refused', async () => {
  // The vue branch reads `one.imported`, which only a named specifier has. An
  // `ImportNamespaceSpecifier` has none, so `import * as vue from 'vue'` binds every export of the
  // framework — including the three the guard removed by name — under a local the free-identifier
  // walk then treats as declared. `createApp(...).mount(document.body)` is the whole `UiModal`
  // defect, and the guard's own comment says these three exports are the ones it excludes.
  const probe = `<script setup lang="ts">
import * as vue from 'vue'
const host = vue.createApp({ render: () => vue.h('div', 'elsewhere') })
vue.render(vue.h('div'), host as never)
</script>
<template><div /></template>`
  assert.notDeepEqual(
    await reasonsFor(probe),
    [],
    '`Teleport`, `createApp` and `render` are excluded by name only, so a namespace import restores all three',
  )
})

test('an allowlisted intrinsic does not hand back the globals it was allowed instead of', async () => {
  // `Object` is on the intrinsic list as a global that "computes and owns nothing outside this
  // process". `Object.constructor` is `Function`, which is not on the list, and one call from it is
  // every global the list exists to withhold. The guard's stated rule is about what a component may
  // REACH, not about what it may spell, so a reach to the whole global object through a permitted
  // name is a reach the rule does not permit.
  const probe = `<script setup lang="ts">
const everything = Object.constructor('return globalThis')() as Record<string, never>
const doc = everything.document as unknown as { body: { style: { overflow: string } } }
doc.body.style.overflow = 'hidden'
</script>
<template><div /></template>`
  assert.notDeepEqual(
    await reasonsFor(probe),
    [],
    'an intrinsic whose `.constructor` is `Function` is a door to every global the allowlist withholds',
  )
})

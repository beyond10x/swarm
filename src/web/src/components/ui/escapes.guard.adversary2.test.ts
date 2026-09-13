// Adversary cases against `escapes.guard.ts` (wave 2026-09-12d, unit B, pass 2).
//
// Pass 1 asked what a component reaches without naming anything IN ITS SCRIPT, and the guard was
// corrected to route every module-naming construct through `reachModule`. These cases ask the same
// question of the two parts of a Single File Component the guard STILL does not read as code: the
// template's EXPRESSIONS (it reads only two tag names) and the `<style>` block (it reads nothing at
// all). Both are compiled and shipped; neither is a script.
//
// The acceptance is "refused whatever spelling it reaches by". Each probe below leaves its own
// subtree exactly as `UiModal` does, by a spelling `ESCAPES` does not match and — the point of this
// pass — that the new guard does not match either. No existing case is touched.
//
// WHAT THESE THREE CASES ARE NOW. All three routes were CONFIRMED and all three are
// `origin: pre-existing`: the four-literal `ESCAPES` this wave replaced missed every one of them
// too, so they are not a defect of the guard that replaced it and they are not this unit's to
// close. They are filed as `story:the-guard-reads-no-template-or-style` and each case is PINNED to
// today's measured state — the guard returns NO reason for that spelling — rather than deleted,
// skipped or left red. A red case takes the suite's exit status for everything after it, and a
// deleted one takes the measurement with it.
//
// So each case below asserts the escape is real, asserts the four literals miss it, and then
// asserts the reason list is EMPTY. When the story closes, the last assertion of each is inverted
// back to `notDeepEqual(..., [])` and the name loses its `no_reason` — the probe, the precondition
// and the compiled-code check are already right and do not move.
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'

import { compileTemplate } from '@vue/compiler-sfc'

import { escapingComponents } from './escapes.guard.ts'

const HERE = new URL('.', import.meta.url)

/** The same directory-as-text the two shipping callers hand the guard. */
const directory: Record<string, string> = Object.fromEntries(
  readdirSync(HERE)
    .filter((name) => !name.endsWith('.test.ts'))
    .map((name) => [name, readFileSync(new URL(`./${name}`, HERE), 'utf8')]),
)

/** The predicate that shipped before this story, kept so a probe proves it measures a NEW spelling. */
const FOUR_LITERALS = /<Teleport\b|document\.|window\.addEventListener|window\.matchMedia/

async function reasonsFor(probe: string): Promise<string[]> {
  const probed = await escapingComponents({ ...directory, 'UiProbe.vue': probe })
  return probed['UiProbe.vue'] ?? []
}

test('the_guard_gives_no_reason_for_a_teleport_spelled_is_vue_Teleport', async () => {
  // The guard walks the template AST for exactly two tags: `/^teleport$/i` and `/^component$/i`.
  // Vue has a third spelling of the same mount. `<div is="vue:Teleport">` is a NATIVE element whose
  // `is` attribute carries the `vue:` prefix, and `resolveComponentType` in `@vue/compiler-core`
  // rewrites the tag to `Teleport` BEFORE the built-in lookup — so the render function this
  // component ships contains the real `Teleport`, while the guard sees a `div` and says nothing.
  // The first assertion compiles the template and reads the generated code, so this is not a claim
  // about what Vue might do: it is what Vue does with this exact string.
  const template = `<div is="vue:Teleport" to="body"><div class="overlay" /></div>`
  const probe = `<template>${template}</template>`
  const { code } = compileTemplate({ id: 'probe', filename: 'UiProbe.vue', source: template })
  assert.match(code, /Teleport as _Teleport/, 'the probe must actually compile to a Teleport, or it measures nothing')
  assert.ok(!FOUR_LITERALS.test(probe), 'the four literals must miss it, or it measures nothing new')
  assert.deepEqual(
    await reasonsFor(probe),
    [],
    'PINNED, not wanted: the guard walks the template for the tag names `teleport` and `component` ' +
      'only, so a component whose markup mounts on `document.body` through the `is="vue:"` spelling ' +
      'reads as one that stays inside its own subtree and a Box naming it is rendered inert on the ' +
      'canvas rather than refused. Filed as story:the-guard-reads-no-template-or-style (CONFIRMED, ' +
      'origin pre-existing — the four ' +
      'literals missed it too). When that story closes this becomes `notDeepEqual`; everything ' +
      'above this line is already the case it will need.',
  )
})

test('the_guard_gives_no_reason_for_an_escape_written_in_a_template_expression', async () => {
  // `directReach` walks the template AST for TAG names only; not one expression in a template is
  // ever parsed. `$root` and `$parent` are public instance properties, available in every template
  // without being declared, imported or named anywhere in a script — so an escape written in a
  // `@click` handler passes the free-identifier walk by never being offered to it. Reaching the
  // ROOT component instance is outside this component's subtree by definition, and it is none of
  // the three routes the header enumerates as uncaught: not a DOM element handle, not a property
  // access on an allowed value, not a value handed in as a prop or a slot.
  const probe = `<template><button @click="$root.$el.ownerDocument.body.style.overflow = 'hidden'">x</button></template>`
  assert.ok(!FOUR_LITERALS.test(probe), 'the four literals must miss it, or it measures nothing new')
  assert.deepEqual(
    await reasonsFor(probe),
    [],
    'PINNED, not wanted: the guard reads no template expression at all, so anything a component ' +
      'does in one — `$root`, `$parent`, and every other public instance property, none of which is ' +
      'declared, imported or named in a script — is invisible to the free-identifier walk. Filed as ' +
      'story:the-guard-reads-no-template-or-style (CONFIRMED, origin pre-existing). When that ' +
      'story closes this becomes `notDeepEqual`.',
  )
})

test('the_guard_gives_no_reason_for_an_unscoped_style_block', async () => {
  // The guard reads `descriptor.script`, `descriptor.scriptSetup` and `descriptor.template`. It
  // never looks at `descriptor.styles`. An SFC `<style>` without `scoped` emits its selectors
  // GLOBALLY, so `body { overflow: hidden }` is the scroll lock `UiModal` performs in JavaScript,
  // performed in CSS instead — outside the subtree `inert` covers, and outside the subtree any
  // attribute covers. Every shipping component happens to write `scoped` today; nothing in the
  // repository requires it, and the guard's enumerated edge does not mention styles.
  const probe = `<template><div /></template>\n<style>\nbody { overflow: hidden; }\n</style>`
  assert.ok(!FOUR_LITERALS.test(probe), 'the four literals must miss it, or it measures nothing new')
  assert.deepEqual(
    await reasonsFor(probe),
    [],
    'PINNED, not wanted: the guard reads `descriptor.script`, `descriptor.scriptSetup` and ' +
      '`descriptor.template` and never `descriptor.styles`, so a global stylesheet shipped by a ' +
      'component reaches the whole document unseen — `body { overflow: hidden }` is UiModal\'s ' +
      'scroll lock written in CSS. Filed as story:the-guard-reads-no-template-or-style ' +
      '(CONFIRMED, origin pre-existing; every ' +
      'shipping component writes `scoped` today and nothing requires it). When that story closes ' +
      'this becomes `notDeepEqual`.',
  )
})

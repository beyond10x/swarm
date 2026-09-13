// Adversarial: `:inert` is the whole of the read-only claim, and it does not cover a Teleport.
//
// `src/web/src/components/canvas/UiBox.vue` says of itself:
//
//   "A component the contract says EMITS is rendered inert, and says so. Nothing writes
//    `Box.props` back ... so a text input a person could type into would discard the typing on the
//    next refresh."
//
// and its template says:
//
//   "`inert` is the whole mechanism: the component renders exactly as it would, and takes no
//    input, because there is nowhere for what it emits to go."
//
//     <div :inert="inert || undefined">
//       <component :is="component" v-bind="bound" />
//     </div>
//
// `inert` is an attribute on a DOM SUBTREE. It disables interaction with the elements inside that
// div and with nothing else. Two things a component can do escape it entirely:
//
//   1. `<Teleport to="body">` renders the component's content as a child of `document.body`, which
//      is not inside the inert div. The teleported markup is fully interactive.
//   2. A listener registered on `document` or `window`, or a write to `document.body.style`, is
//      not scoped to any subtree, so `inert` cannot reach it.
//
// These cases assert that no component the contract lists as an emitter does either, because
// `UiBox.vue` renders every one of them behind nothing but that attribute.
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'

import { escapingComponents } from '../ui/escapes.guard.ts'

const contract = new URL('../ui/index.ts', import.meta.url)
const source = readFileSync(contract, 'utf8')

/** The contract's emitter list, read out of the file: `.vue` cannot be imported by node. */
const emitters: string[] = (() => {
  const body = /export const uiEmitters[^[]*\[([^\]]*)\]/s.exec(source)
  assert.ok(body, 'index.ts exports a `uiEmitters` set')
  return body[1]
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
})()

/**
 * The brace-balanced body of `export const <name> = {…}`.
 *
 * A non-greedy `\{[\s\S]*?\n\}` — which this file used to carry twice — ends at the first line
 * that starts with `}`, so a declaration containing a nested object on its own lines is read as a
 * strict subset and the case then compares that subset with something and finds it equal. Reading
 * less than is there is the harmful direction: it fails nothing and hides the rest.
 */
function objectLiteral(text: string, name: string): string {
  const at = text.indexOf(`export const ${name}`)
  assert.ok(at >= 0, `index.ts exports ${name}`)
  const open = text.indexOf('{', at)
  assert.ok(open >= 0, `${name} is an object literal`)
  let depth = 0
  for (let i = open; i < text.length; i += 1) {
    if (text[i] === '{') depth += 1
    else if (text[i] === '}') {
      depth -= 1
      if (depth === 0) return text.slice(open, i + 1)
    }
  }
  throw new Error(`${name} is not closed`)
}

// Both cases below asked their question of the CHARACTERS of a component: `/<Teleport\b/` for the
// first, `/\b(?:document|window)\.addEventListener\b|\bdocument\.body\b/` for the second. A
// component escaping in another spelling — `globalThis.document`, an aliased `const doc =
// document`, a composable one call away — answered "no" to both, and "no" here means "safe to
// render inert on the canvas". So the reach is now decided by `../ui/escapes.guard.ts`, which
// allows what a component may name and reports everything else, and the two cases read its reasons
// rather than a regex of their own. One predicate, in one file, for the contract and for this.
const UI = new URL('../ui/', import.meta.url)
const reach = await escapingComponents(
  Object.fromEntries(
    readdirSync(UI)
      .filter((name) => !name.endsWith('.test.ts'))
      .map((name) => [name, readFileSync(new URL(name, UI), 'utf8')]),
  ),
)

/** Why `name` leaves its own subtree; empty when it does not. */
function reasons(name: string): string[] {
  return reach[`${name}.vue`] ?? []
}

test('a component UiBox renders inert keeps its markup inside the inert subtree', () => {
  // `inert` is scoped to the div `UiBox.vue` puts it on. A component that teleports its content to
  // `document.body` renders that content outside the div, where the attribute does not reach it,
  // so its controls stay clickable and focusable and everything it emits is discarded in silence.
  const teleporting = emitters.filter((name) => reasons(name).some((why) => why.includes('Teleport')))
  assert.deepEqual(
    teleporting,
    [],
    'UiBox.vue relies on `:inert` on a wrapper div; a teleported component escapes that wrapper',
  )
})

test('a component UiBox renders inert takes no input `inert` cannot stop', () => {
  // A document- or window-level listener, and a write to `document.body`, are not inside any
  // subtree, so the attribute cannot disable them. A keydown handler on `document` keeps taking
  // Tab and Escape from the whole application while the panel is on the canvas.
  const escaping = emitters.filter((name) => reasons(name).length > 0)
  assert.deepEqual(
    escaping.map((name) => `${name}: ${reasons(name).join('; ')}`),
    [],
    'these components act outside their own subtree, which `:inert` cannot reach',
  )
})

// The state the two cases above describe is not one this file constructs: it is what the shipped
// path produces for a box an agent may write. This case reads the contract's own declared types
// and drives the shipped decision function with them, so the reachability is measured rather than
// asserted in prose. It is expected to PASS; it is here to show the other two matter.
test('a box naming UiModal with open true is a panel the canvas mounts', async () => {
  const { uiBoxSpec } = await import('../../lib/uibox.ts')

  const declared = JSON.parse(
    objectLiteral(source, 'uiPropTypes')
      .replace(/([{,]\s*)([A-Za-z][A-Za-z0-9]*)\s*:/g, '$1"$2":')
      .replace(/'/g, '"')
      .replace(/,(\s*[}\]])/g, '$1'),
  )
  const names: ReadonlySet<string> = new Set(Object.keys(declared))

  const spec = uiBoxSpec(
    { entity: 'swarm.blackbox.Box', fields: { kind: 'Ui', ref_id: 'UiModal', props: { open: 'true' } } },
    names,
    declared,
  )
  assert.deepEqual(spec, {
    kind: 'panel',
    component: 'UiModal',
    props: { open: true },
    view: undefined,
  })
  // What follows was, when this case was written, `assert.ok(emitters.includes('UiModal'))` —
  // "the contract says UiModal emits, so UiBox renders it inert". That was the defect, not a
  // property to keep: `inert` is a subtree attribute and UiModal's markup is teleported out of the
  // subtree. The fix moved UiModal out of the inert list and into a refusal, so the premise is
  // asserted here in the form it now takes, and more of it: the name is refused BY THE CONTRACT,
  // and the shipped decision function routes it to a refusal rather than to a panel.
  assert.ok(!emitters.includes('UiModal'), 'UiModal is not something a panel may render inert')
  const refused: ReadonlySet<string> = new Set(
    /export const uiRefused[^[]*\[([^\]]*)\]/s
      .exec(source)![1]
      .split(',')
      .map((entry) => entry.trim().replace(/'/g, ''))
      .filter(Boolean),
  )
  assert.ok(refused.has('UiModal'), 'the contract refuses UiModal to any panel')
  assert.deepEqual(
    uiBoxSpec(
      { entity: 'swarm.blackbox.Box', fields: { kind: 'Ui', ref_id: 'UiModal', props: { open: 'true' } } },
      names,
      declared,
      refused,
    ),
    { kind: 'unsafe-component', named: 'UiModal' },
    'the canvas draws a refusal for it, not the component',
  )
})

// `UiTable` emits `row-click`, and the contract table did not say so, so a table panel took clicks
// on every row and dropped each one: nothing writes `Box.props` back and no listener is bound by
// `<component :is>`. The decision taken is that a table panel is inert like every other emitter.
//
// What that costs is more than "not clickable", and the note the box shows has to say so: `inert`
// removes the subtree from the ACCESSIBILITY TREE, from find-in-page and from text selection, so a
// table inside it is drawn on screen and is not readable by a screen reader, not findable by
// Ctrl-F, and not copyable. It is display-only, not read-only.
//
// This is a behaviour change on purpose, so it is asserted rather than inherited.
test('a box naming UiTable is a panel the canvas renders inert, and says so', async () => {
  const { uiBoxSpec } = await import('../../lib/uibox.ts')

  const declared = JSON.parse(
    objectLiteral(source, 'uiPropTypes')
      .replace(/([{,]\s*)([A-Za-z][A-Za-z0-9]*)\s*:/g, '$1"$2":')
      .replace(/'/g, '"')
      .replace(/,(\s*[}\]])/g, '$1'),
  )
  const names: ReadonlySet<string> = new Set(Object.keys(declared))
  const refused: ReadonlySet<string> = new Set(
    /export const uiRefused[^[]*\[([^\]]*)\]/s
      .exec(source)![1]
      .split(',')
      .map((entry) => entry.trim().replace(/'/g, ''))
      .filter(Boolean),
  )

  assert.deepEqual(
    uiBoxSpec(
      { entity: 'swarm.blackbox.Box', fields: { kind: 'Ui', ref_id: 'UiTable', props: {} } },
      names,
      declared,
      refused,
    ),
    { kind: 'panel', component: 'UiTable', props: {}, view: undefined },
    'a table is drawn, not refused: it stays inside its own subtree',
  )
  assert.ok(emitters.includes('UiTable'), 'the contract lists UiTable as an emitter, so UiBox inerts it')

  // "Renders inert, and says so", decided by VALUE. The predicate lives in
  // `./uibox.inert.guard.ts` because `uibox.inert.mutants.test.ts` runs the SAME predicate
  // over mutated copies of `UiBox.vue`; a copy in each file would let the mutation harness prove
  // something about a predicate that is not the one shipping here.
  const { assertInertGuard } = await import('./uibox.inert.guard.ts')
  await assertInertGuard(readFileSync(new URL('./UiBox.vue', import.meta.url), 'utf8'))
})

// The guard reads `UiBox.vue`, so "the guard passes" says nothing until the guard is known to be
// able to fail. `uibox.inert.mutants.test.ts` shows that against the real file; this shows it
// against a box small enough to read, and without reference to anything that ships.
test('the inert guard rejects a box that mounts a component with no inert binding', async () => {
  const { assertInertGuard, NOTE } = await import('./uibox.inert.guard.ts')
  const box = (inert: string) => `
<script setup lang="ts">
const inert = computed(() => (spec.value ? uiEmitters.has(spec.value.component) : false))
</script>
<template>
  <div${inert}><component :is="component" /></div>
  <p v-if="inert && !refusal && !feed">${NOTE}</p>
</template>
`
  await assertInertGuard(box(' :inert="inert || undefined"'))
  await assert.rejects(
    () => assertInertGuard(box('')),
    /carries no `inert` binding/,
    'a guard that cannot reject is not a guard',
  )
})

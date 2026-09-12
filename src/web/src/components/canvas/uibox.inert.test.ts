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
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

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

function componentSource(name: string): string {
  return readFileSync(new URL(`../ui/${name}.vue`, import.meta.url), 'utf8')
}

test('a component UiBox renders inert keeps its markup inside the inert subtree', () => {
  // `inert` is scoped to the div `UiBox.vue` puts it on. A component that teleports its content to
  // `document.body` renders that content outside the div, where the attribute does not reach it,
  // so its controls stay clickable and focusable and everything it emits is discarded in silence.
  const teleporting = emitters.filter((name) => /<Teleport\b/.test(componentSource(name)))
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
  const escaping = emitters.filter((name) =>
    /\b(?:document|window)\.addEventListener\b|\bdocument\.body\b/.test(componentSource(name)),
  )
  assert.deepEqual(
    escaping,
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
    /export const uiPropTypes[^{]*(\{[\s\S]*?\n\})/
      .exec(source)![1]
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

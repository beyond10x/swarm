// Adversarial: the refusal `UiBox.vue` documents cannot be reached by its only caller.
//
// `src/web/src/components/canvas/UiBox.vue` says of itself:
//
//   "A box naming a component the library does not have draws as a refusal rather than as nothing:
//    an agent wrote that name, and an empty rectangle would look like a panel that had not loaded."
//
// and carries the branch that does it (`<p v-else class="refused">No component named ...`).
//
// Its only caller is `SwarmCanvas.vue`, which picks the node type with
// `const panel = uiBoxSpec(instance, uiComponentNames); const type = panel ? 'ui' : 'instance'`.
// So `UiBox` is mounted only when `uiBoxSpec` returned a spec, and a spec always names a component
// the library has — the refusal branch is unreachable.
//
// The root cause is here, in the decision function: `uiBoxSpec` collapses "this is not a Ui box"
// and "this is a Ui box whose component the library does not have" onto the same `undefined`, so no
// caller can tell the second from the first. This case asserts they are distinguishable.
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { uiBoxSpec } from './uibox.ts'

const KNOWN = new Set(['UiTable', 'UiKeyValue', 'UiBadge'])

function box(fields: Record<string, unknown>) {
  return { entity: 'swarm.blackbox.Box', fields }
}

test('a Ui box naming a component nobody wrote is told apart from a box that is not a Ui box', () => {
  const unknownComponent = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiMarkdown' }), KNOWN)
  const notAUiBox = uiBoxSpec(box({ kind: 'Service', ref_id: 'UiMarkdown' }), KNOWN)
  assert.notDeepEqual(
    unknownComponent,
    notAUiBox,
    'UiBox.vue documents a refusal for the first and an instance node for the second; ' +
      'a caller given the same value for both cannot draw them differently',
  )
})

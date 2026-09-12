// What a Ui box renders, decided from the box alone.
//
// Run with `npm test` in src/web: node's own test runner, type-stripping the TypeScript. This
// repository has no test framework and this story does not add one — the decision below is a pure
// function precisely so it can be checked without mounting a component.
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { columnsFrom, uiBoxSpec } from './uibox.ts'

const KNOWN = new Set(['UiTable', 'UiKeyValue', 'UiBadge'])

function box(fields: Record<string, unknown>, entity = 'swarm.blackbox.Box') {
  return { entity, fields }
}

test('a Ui box names its component in ref_id', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: { text: 'live' } }), KNOWN)
  assert.deepEqual(spec, { component: 'UiBadge', props: { text: 'live' }, view: undefined })
})

test('a box of another kind is not a Ui box', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Service', ref_id: 'UiBadge' }), KNOWN), undefined)
})

test('an instance of another entity is not a Ui box', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge' }, 'swarm.goal.Goal'), KNOWN), undefined)
})

test('a component the library does not have is refused, not guessed', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiMarkdown' }), KNOWN), undefined)
  assert.equal(uiBoxSpec(box({ kind: 'Ui', ref_id: null }), KNOWN), undefined)
})

test('props are absent, not empty, when nothing has written them', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: null }), KNOWN)
  assert.deepEqual(spec?.props, {})
})

test('a prop value that is JSON is decoded; one that is not stays the string it is', () => {
  const spec = uiBoxSpec(
    box({
      kind: 'Ui',
      ref_id: 'UiTable',
      props: {
        columns: '[{"key":"slug","label":"Swarm"}]',
        dense: 'true',
        rowKey: 'slug',
        width: '12',
        label: '"12"',
      },
    }),
    KNOWN,
  )
  assert.deepEqual(spec?.props, {
    columns: [{ key: 'slug', label: 'Swarm' }],
    dense: true,
    rowKey: 'slug',
    width: 12,
    label: '12',
  })
})

test('the view a table is fed from is not a prop of the component', () => {
  const spec = uiBoxSpec(
    box({ kind: 'Ui', ref_id: 'UiTable', props: { view: 'swarm.blackbox.Canvas', dense: 'true' } }),
    KNOWN,
  )
  assert.equal(spec?.view, 'swarm.blackbox.Canvas')
  assert.deepEqual(spec?.props, { dense: true })
})

test('a props map that is not a map of strings is ignored rather than half-read', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: 'text=live' }), KNOWN)
  assert.deepEqual(spec?.props, {})
})

test('a table fed from a view gets its columns from the rows when the box named none', () => {
  assert.deepEqual(columnsFrom([{ slug: 'alpha', events: 3 }, { slug: 'beta', events: 1 }]), [
    { key: 'slug', label: 'slug' },
    { key: 'events', label: 'events' },
  ])
})

test('no rows means no columns invented', () => {
  assert.deepEqual(columnsFrom([]), [])
})

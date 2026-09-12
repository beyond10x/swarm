// What a Ui box renders, decided from the box alone.
//
// Run with `npm test` in src/web: node's own test runner, type-stripping the TypeScript. This
// repository has no test framework and this story does not add one — the decision below is a pure
// function precisely so it can be checked without mounting a component.
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { columnsFrom, uiBoxSpec, viewProblem } from './uibox.ts'

const KNOWN = new Set(['UiTable', 'UiKeyValue', 'UiBadge', 'UiTextArea'])

/** As `components/ui/index.ts` declares them, for the cases that exercise a declared type. */
const DECLARED = {
  UiTable: { columns: 'json', rows: 'json', rowKey: 'string', dense: 'boolean' },
  UiBadge: { tone: 'string', text: 'string' },
  UiTextArea: { modelValue: 'string', rows: 'number' },
} as const

function box(fields: Record<string, unknown>, entity = 'swarm.blackbox.Box') {
  return { entity, fields }
}

test('a Ui box names its component in ref_id', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: { text: 'live' } }), KNOWN)
  assert.deepEqual(spec, {
    kind: 'panel',
    component: 'UiBadge',
    props: { text: 'live' },
    view: undefined,
  })
})

test('a box of another kind is not a Ui box', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Service', ref_id: 'UiBadge' }), KNOWN), undefined)
})

test('an instance of another entity is not a Ui box', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge' }, 'swarm.goal.Goal'), KNOWN), undefined)
})

test('a component the library does not have is a refusal a caller can draw', () => {
  // Not `undefined`: that is what a box which is not a Ui box returns, and a caller given one
  // value for both cannot draw the refusal `UiBox.vue` documents.
  assert.deepEqual(uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiMarkdown' }), KNOWN), {
    kind: 'unknown-component',
    named: 'UiMarkdown',
  })
})

test('a Ui box that names nothing at all is not a refusal, because nothing was refused', () => {
  assert.equal(uiBoxSpec(box({ kind: 'Ui', ref_id: null }), KNOWN), undefined)
})

test('props are absent, not empty, when nothing has written them', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: null }), KNOWN)
  assert.deepEqual(spec?.props, {})
})

test('a prop value is read as the type the contract declares for it', () => {
  const spec = uiBoxSpec(
    box({
      kind: 'Ui',
      ref_id: 'UiTable',
      props: {
        columns: '[{"key":"slug","label":"Swarm"}]',
        dense: 'true',
        rowKey: '123',
      },
    }),
    KNOWN,
    DECLARED,
  )
  assert.deepEqual(spec?.props, {
    columns: [{ key: 'slug', label: 'Swarm' }],
    dense: true,
    // `rowKey: string` in the contract, so digits are the column name they are.
    rowKey: '123',
  })
})

test('a number the contract declares as one arrives as a number', () => {
  const spec = uiBoxSpec(
    box({ kind: 'Ui', ref_id: 'UiTextArea', props: { rows: '8', modelValue: '8' } }),
    KNOWN,
    DECLARED,
  )
  assert.deepEqual(spec?.props, { rows: 8, modelValue: '8' })
})

test('a value that is not the type declared is left as written rather than mangled', () => {
  const spec = uiBoxSpec(
    box({ kind: 'Ui', ref_id: 'UiTextArea', props: { rows: 'many' } }),
    KNOWN,
    DECLARED,
  )
  assert.deepEqual(spec?.props, { rows: 'many' })
})

test('a prop the contract does not type is text, unless it is bracketed', () => {
  const spec = uiBoxSpec(
    box({
      kind: 'Ui',
      ref_id: 'UiBadge',
      props: { text: '404', hint: 'null', extra: '[1,2]', shape: '{"a":1}' },
    }),
    KNOWN,
    DECLARED,
  )
  assert.deepEqual(spec?.props, {
    text: '404',
    hint: 'null',
    extra: [1, 2],
    shape: { a: 1 },
  })
})

test('the view a table is fed from is not a prop of the component', () => {
  const spec = uiBoxSpec(
    box({ kind: 'Ui', ref_id: 'UiTable', props: { view: 'swarm.blackbox.Canvas', dense: 'true' } }),
    KNOWN,
    DECLARED,
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


// A component that cannot be contained is refused rather than rendered.
//
// `UiModal` teleports to `document.body`, locks the page's scroll and swallows Escape and Tab from
// the whole document, so the `inert` a panel puts on its own wrapper cannot reach any of it. A box
// naming one used to draw a full-screen backdrop over the application with no way out.
const REFUSED = new Set(['UiModal'])

test('a component that acts outside its own subtree is refused, not drawn', () => {
  const spec = uiBoxSpec(
    { entity: 'swarm.blackbox.Box', fields: { kind: 'Ui', ref_id: 'UiModal', props: { open: 'true' } } },
    new Set(['UiModal', 'UiBadge']),
    undefined,
    REFUSED,
  )
  assert.deepEqual(spec, { kind: 'unsafe-component', named: 'UiModal' })
})

test('a component that stays in its own box is drawn as before', () => {
  const spec = uiBoxSpec(box({ kind: 'Ui', ref_id: 'UiBadge', props: { text: 'live' } }), KNOWN, DECLARED, REFUSED)
  assert.equal(spec?.kind, 'panel')
})

// A view named on a box is a name an agent wrote, exactly like `ref_id`, and it is wrong in the
// same ways. An unread view and an empty view look identical once the rows are `[]`, so the panel
// has to be told which it is looking at.

test('a view the specification does not declare is named as the mistake it is', () => {
  const problem = viewProblem('swarm.blackbox.Canvs', ['swarm.blackbox.Canvas'], undefined)
  assert.match(problem ?? '', /swarm\.blackbox\.Canvs/)
})

test('a view the specification declares is no problem', () => {
  assert.equal(viewProblem('swarm.blackbox.Canvas', ['swarm.blackbox.Canvas'], undefined), undefined)
})

test('a view that could not be read says so rather than showing no rows', () => {
  const problem = viewProblem('swarm.blackbox.Canvas', ['swarm.blackbox.Canvas'], 'the runtime refused it')
  assert.match(problem ?? '', /the runtime refused it/)
})

test('before the specification is read nothing is accused of not existing', () => {
  assert.equal(viewProblem('swarm.blackbox.Canvas', undefined, undefined), undefined)
})

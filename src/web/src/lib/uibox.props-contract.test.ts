// Adversarial: the decoder is driven against the two documents this unit wrote about it.
//
// `src/core/domains/blackbox.yaml` (the `Box.props` comment) tells whoever writes a box:
//
//   "a value that is not a scalar (a table's `columns`) is written as JSON text and decoded by
//    the reader"
//
// and `src/web/src/components/ui/index.ts` — the component library CONTRACT — declares
// `UiBadge  tone: ...; text*: string`. Together those two say: a scalar prop is carried as
// itself, and `text` arrives at the component as a string.
//
// The story's acceptance says the component renders "with the props the box carries".
//
// These cases assert exactly that, for prop values a person would actually write on a panel: an
// HTTP status on a badge, a version tag, an id. They are expected to be RED against
// `decode()` in uibox.ts, which reinterprets any value that happens to parse as JSON.
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { uiBoxSpec } from './uibox.ts'

const KNOWN = new Set(['UiTable', 'UiKeyValue', 'UiBadge', 'UiTile', 'UiTextInput'])

function propsOf(fields: Record<string, unknown>): Record<string, unknown> {
  const spec = uiBoxSpec({ entity: 'swarm.blackbox.Box', fields }, KNOWN)
  assert.ok(spec, 'the box is a Ui box naming a known component')
  return spec.props
}

test('a badge whose text is an HTTP status carries that text as a string', () => {
  // index.ts: `UiBadge  tone: 'ok'|'warn'|'fault'|'info'|'muted'; text*: string`
  const props = propsOf({ kind: 'Ui', ref_id: 'UiBadge', props: { tone: 'fault', text: '404' } })
  assert.equal(typeof props.text, 'string', '`text*: string` per components/ui/index.ts')
  assert.equal(props.text, '404')
})

test('a required string prop whose text is `null` is not dropped', () => {
  // A badge reading `null` — the word an ESS reader sees for an unwritten field — is a thing an
  // agent writes. `text` is required; the decoder must not turn it into the absent value.
  const props = propsOf({ kind: 'Ui', ref_id: 'UiBadge', props: { text: 'null' } })
  assert.equal(props.text, 'null')
})

test('a tile titled with a year is a title, not a number', () => {
  // index.ts: `UiTile  title*: string; subtitle: string; ...`
  const props = propsOf({ kind: 'Ui', ref_id: 'UiTile', props: { title: '2026', subtitle: 'wave' } })
  assert.equal(typeof props.title, 'string', '`title*: string` per components/ui/index.ts')
  assert.equal(props.title, '2026')
})

test("a table's rowKey is the column name it is, even when that name is digits", () => {
  // index.ts: `UiTable  ... rowKey: string`
  const props = propsOf({ kind: 'Ui', ref_id: 'UiTable', props: { rowKey: '123' } })
  assert.equal(typeof props.rowKey, 'string', '`rowKey: string` per components/ui/index.ts')
})

test('a text input carries the text it was given, whatever that text says', () => {
  // index.ts: `UiTextInput  modelValue*: string; ...`
  const props = propsOf({ kind: 'Ui', ref_id: 'UiTextInput', props: { modelValue: 'true' } })
  assert.equal(typeof props.modelValue, 'string', '`modelValue*: string` per components/ui/index.ts')
  assert.equal(props.modelValue, 'true')
})

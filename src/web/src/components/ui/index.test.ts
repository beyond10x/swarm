// The name-to-component map cannot drift from the contract's exports.
//
// A `Box{kind: Ui}` names its component as a string, so rendering one needs a lookup from name to
// component — a second list of the same 21 names. This checks the two lists are the same list,
// by reading the file rather than importing it: a `.vue` module cannot be loaded by node, and a
// hand-kept comparison is exactly the drift it is meant to catch.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const source = readFileSync(new URL('./index.ts', import.meta.url), 'utf8')

/** Every component the contract re-exports: `export { default as UiButton } from './UiButton.vue'`. */
const exported = [...source.matchAll(/export \{ default as (\w+) \} from/g)].map((m) => m[1])

/** Every key of the `uiComponents` lookup. */
const mapped = (() => {
  const body = /export const uiComponents[^{]*\{([^}]*)\}/s.exec(source)
  assert.ok(body, 'index.ts declares a `uiComponents` lookup')
  return body[1]
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean)
})()

test('every exported component is in the lookup, and nothing else is', () => {
  assert.deepEqual([...mapped].sort(), [...exported].sort())
})

test('the contract still documents all 21 components', () => {
  assert.equal(exported.length, 21)
})

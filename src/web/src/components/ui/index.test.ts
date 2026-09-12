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

// The contract's comment table is the only place a prop's DECLARED TYPE is written down, and two
// lookups now depend on it: what a prop value means, and which components emit. Both are parsed
// out of the table here and compared with the exported maps, so neither can drift from the
// sentence a person reads.

type PropType = 'string' | 'number' | 'boolean' | 'json'

interface Declared {
  props: Record<string, PropType>
  emits: boolean
}

/** The table, parsed: `UiBadge  tone: 'ok'|…; text*: string   —`. */
function tableOf(source: string): Record<string, Declared> {
  const declared: Record<string, Declared> = {}
  let current: string | undefined
  for (const line of source.split('\n')) {
    const comment = /^\/\/ (.*)$/.exec(line)
    if (!comment) {
      if (current) break // the table ends at the first line that is not a comment
      continue
    }
    const body = comment[1]
    const named = /^(Ui[A-Za-z]+)\s{2,}(.*)$/.exec(body)
    if (named) {
      current = named[1]
      declared[current] = { props: {}, emits: false }
    } else if (!current) {
      continue
    }
    const rest = named ? named[2] : body.trim()
    const chunks = rest.split(/\s{2,}/)
    const tail = chunks.length > 1 ? chunks[chunks.length - 1] : ''
    if (/emits/.test(tail)) declared[current!].emits = true
    for (const entry of chunks.slice(0, chunks.length > 1 ? -1 : undefined).join(' ').split(';')) {
      const pair = /^\s*([A-Za-z][A-Za-z0-9]*)\*?\s*:\s*(.+)$/.exec(entry)
      if (!pair) continue
      declared[current!].props[pair[1]] = typeOf(pair[2])
    }
  }
  return declared
}

function typeOf(written: string): PropType {
  if (/\[\]|\{/.test(written)) return 'json'
  if (/\bboolean\b/.test(written)) return 'boolean'
  if (/\bnumber\b/.test(written)) return 'number'
  return 'string'
}

const table = tableOf(source)

/** The exported maps, read as JSON-ish literals out of the same file. */
function literal(name: string): string {
  const at = source.indexOf(`export const ${name}`)
  assert.ok(at >= 0, `index.ts exports ${name}`)
  const opened = /[[{]/.exec(source.slice(at))
  assert.ok(opened, `${name} is a literal`)
  const open = at + opened.index
  const shut = source[open] === '{' ? '}' : ']'
  let depth = 0
  for (let i = open; i < source.length; i += 1) {
    if (source[i] === source[open]) depth += 1
    if (source[i] === shut) {
      depth -= 1
      if (depth === 0) return source.slice(open, i + 1)
    }
  }
  throw new Error(`${name} is not closed`)
}

test('the declared prop types are the contract table, component for component', () => {
  // The literal is TypeScript: bare keys and single quotes. Quoting them is all it takes to read
  // it as data, and reading it as data is what keeps this check independent of the module loading.
  const asJson = literal('uiPropTypes')
    .replace(/([{,]\s*)([A-Za-z][A-Za-z0-9]*)\s*:/g, '$1"$2":')
    .replace(/'/g, '"')
    .replace(/,(\s*[}\]])/g, '$1')
  const exported = JSON.parse(asJson)
  const wanted = Object.fromEntries(Object.entries(table).map(([name, d]) => [name, d.props]))
  assert.deepEqual(exported, wanted)
})

test('the components that emit are the components the table says emit', () => {
  const listed = literal('uiEmitters')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  const wanted = Object.entries(table)
    .filter(([, d]) => d.emits)
    .map(([name]) => name)
  assert.deepEqual([...listed].sort(), [...wanted].sort())
})

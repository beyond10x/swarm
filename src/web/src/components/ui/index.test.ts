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
  /** The event names the table's last column writes after `emits`, `select(path)` read as `select`. */
  emitted: string[]
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
      declared[current] = { props: {}, emits: false, emitted: [] }
    } else if (!current) {
      continue
    }
    const rest = named ? named[2] : body.trim()
    const chunks = rest.split(/\s{2,}/)
    const tail = chunks.length > 1 ? chunks[chunks.length - 1] : ''
    if (/emits/.test(tail)) {
      declared[current!].emits = true
      declared[current!].emitted.push(...emittedIn(tail))
    }
    for (const entry of chunks.slice(0, chunks.length > 1 ? -1 : undefined).join(' ').split(';')) {
      const pair = /^\s*([A-Za-z][A-Za-z0-9]*)\*?\s*:\s*(.+)$/.exec(entry)
      if (!pair) continue
      declared[current!].props[pair[1]] = typeOf(pair[2])
    }
  }
  return declared
}

/** `default, footer; emits click` -> `['click']`; `emits select(path)` -> `['select']`. */
function emittedIn(tail: string): string[] {
  const after = /emits\s+(.*)$/.exec(tail)
  if (!after) return []
  return after[1]
    .split(',')
    .map((name) => name.trim().replace(/\(.*$/, '').trim())
    .filter(Boolean)
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

// Superseded in force by 'every component the table says emits is either rendered inert or
// refused' below, which is the same assertion once a refused component exists. Kept as it was for
// the case where nothing is refused at all.
test('the components that emit are the components rendered inert, plus any that are refused', () => {
  const listed = literal('uiEmitters')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  const refused = literal('uiRefused')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  const wanted = Object.entries(table)
    .filter(([, d]) => d.emits)
    .map(([name]) => name)
  assert.deepEqual([...listed, ...refused.filter((name) => wanted.includes(name))].sort(), [...wanted].sort())
})

// A component a panel can render is one that draws inside its own box and touches nothing else.
//
// `UiBox.vue` made every emitter safe with `inert`, which is an attribute on a SUBTREE: a
// component that teleports its markup to `document.body`, or that installs a document-level
// listener, or that writes `document.body.style`, is not inside that subtree and the attribute
// cannot reach it. `UiModal` does all three, so a box naming it put a fixed full-screen backdrop
// over the application with the scroll locked and Escape and Tab swallowed, and — no `@close`
// being bound — no way back.
//
// So the predicate is not "emits". It is "acts only on itself", and it is decided by reading the
// components rather than by remembering: every `.vue` beside this file is scanned, and the scan
// must equal what `index.ts` declares. A component that starts teleporting is refused by having
// been written that way.

const HERE = new URL('.', import.meta.url)

/** Every component file the contract re-exports. */
const files = exported.map((name) => ({ name, source: readFileSync(new URL(`./${name}.vue`, HERE), 'utf8') }))

/** Reaching outside your own subtree, in the three forms the library actually uses. */
const ESCAPES = /<Teleport\b|document\.|window\.addEventListener|window\.matchMedia/

const escaping = files.filter((file) => ESCAPES.test(file.source)).map((file) => file.name)

test('the refused components are exactly the ones that act outside their own subtree', () => {
  const declaredRefused = literal('uiRefused')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  assert.deepEqual([...declaredRefused].sort(), [...escaping].sort())
})

test('nothing a panel renders inert reaches outside its own subtree', () => {
  const inert = literal('uiEmitters')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  assert.deepEqual(
    inert.filter((name) => escaping.includes(name)),
    [],
    '`inert` is a subtree attribute; a component that escapes the subtree must be refused, not inerted',
  )
})

test('every component the table says emits is either rendered inert or refused', () => {
  const inert = literal('uiEmitters')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  const refused = literal('uiRefused')
    .replace(/[[\]]/g, '')
    .split(',')
    .map((entry) => entry.trim().replace(/'/g, ''))
    .filter(Boolean)
  const documented = Object.entries(table)
    .filter(([, d]) => d.emits)
    .map(([name]) => name)
  assert.deepEqual(
    [...documented].sort(),
    [...new Set([...inert, ...refused.filter((name) => documented.includes(name))])].sort(),
  )
  assert.deepEqual(inert.filter((name) => refused.includes(name)), [], 'a component is one or the other')
})

// The table's emits column is now DATA — `uiEmitters` is built from it and `UiBox.vue` decides
// `inert` from that — so a component whose `defineEmits` the table does not name is not a
// documentation slip: it is a panel the canvas lets a person interact with while discarding
// everything it emits. `UiTable` was exactly that, found by an adversary rather than by a check.
//
// So the two are compared directly here: every event name in a component's `defineEmits<{…}>` must
// appear in its row's last column, and the column may name no event the component does not emit.
// This reads the `.vue` files, as the refusal scan above does, because node cannot import them.

/** The event names a component declares: `defineEmits<{ 'row-click': [row: Row] }>` -> `row-click`. */
function emitsOf(componentSource: string): string[] {
  const generic = /defineEmits<\{([\s\S]*?)\}>/.exec(componentSource)
  if (!generic) return []
  return [...generic[1].matchAll(/(?:'([^']+)'|([A-Za-z][\w:-]*))\s*:\s*\[/g)]
    .map((m) => m[1] ?? m[2])
    .filter(Boolean)
}

test('the contract table names exactly the events each component emits', () => {
  const drift = files
    .map((file) => ({
      name: file.name,
      declared: [...(table[file.name]?.emitted ?? [])].sort(),
      actual: [...emitsOf(file.source)].sort(),
    }))
    .filter((row) => JSON.stringify(row.declared) !== JSON.stringify(row.actual))
  assert.deepEqual(
    drift,
    [],
    'a component whose emits the table does not name is rendered as if it emitted nothing',
  )
})

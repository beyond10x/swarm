// Adversarial: the drift guard added for `story:uitable-emits-contract` reads ONE spelling of
// `defineEmits`, and Vue has three.
//
// `index.test.ts` now carries 'the contract table names exactly the events each component emits',
// whose stated purpose is that the `UiTable` defect — a component that emits, a table row that does
// not say so, and therefore a panel the canvas renders NOT inert while discarding everything it
// emits — "fails rather than being found by an adversary". It detects emits with
//
//   /defineEmits<\{([\s\S]*?)\}>/   then   /(?:'([^']+)'|([A-Za-z][\w:-]*))\s*:\s*\[/g
//
// which only matches the type-literal-with-tuple form every component happens to use today. The two
// other forms the Vue compiler accepts — the runtime array `defineEmits(['remove'])` and the
// call-signature type `defineEmits<{ (e: 'remove'): void }>()` — yield no names at all, and the
// guard reads "no names" as "emits nothing", which is indistinguishable from a component that does
// not emit. It fails OPEN.
//
// This is measured rather than argued: the shipped `index.test.ts` is run, unmodified, over a copy
// of this directory in which `UiTag` emits `remove` in one of those spellings and the contract has
// been walked back to match what the guard can see (the row's emits column cleared and `UiTag`
// dropped from `uiEmitters`). That is exactly the state `UiTable` was in before this unit, so every
// other check in the file stays green, and the new guard is the only thing that could object.
//
// The CONTROL case does the same walk-back with the spelling the guard does read, and asserts the
// child suite goes red — so a green child below is the guard failing to see the drift, not this
// harness failing to run anything.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const HERE = fileURLToPath(new URL('.', import.meta.url)).replace(/\/$/, '')

/** `UiTag` as the contract describes it today, and as this file walks it back. */
const TUPLE_EMITS = "const emit = defineEmits<{ remove: [] }>()"
const TABLE_ROW_EMITS = 'emits remove'

/**
 * Run the shipped `index.test.ts` over a copy of this directory in which `UiTag` still emits
 * `remove` — spelled `spelling` — but the contract no longer says so.
 *
 * Returns the child's exit status: 0 means every check in `index.test.ts` passed while a component
 * that emits was documented as one that does not.
 */
function guardOn(spelling: string): { status: number; stdout: string } {
  const dir = mkdtempSync(join(tmpdir(), 'ui-emits-guard-'))
  try {
    cpSync(HERE, dir, {
      recursive: true,
      filter: (from) =>
        from.replace(/\/$/, '') === HERE ||
        from.endsWith('.vue') ||
        from.endsWith('/index.ts') ||
        from.endsWith('/index.test.ts'),
    })

    const tag = readFileSync(join(dir, 'UiTag.vue'), 'utf8')
    assert.ok(tag.includes(TUPLE_EMITS), 'UiTag.vue declares its emit in the form this case rewrites')
    writeFileSync(join(dir, 'UiTag.vue'), tag.replace(TUPLE_EMITS, spelling))

    // The contract, walked back to what the guard can see: the row stops naming the emit and the
    // component leaves the inert list. `uiPropTypes` and `uiRefused` are untouched.
    const contract = readFileSync(join(dir, 'index.ts'), 'utf8')
    assert.ok(contract.includes(TABLE_ROW_EMITS), "the table's UiTag row names the emit")
    assert.ok(contract.includes("  'UiTag',\n"), 'uiEmitters lists UiTag')
    writeFileSync(
      join(dir, 'index.ts'),
      contract.replace(TABLE_ROW_EMITS, '—             ').replace("  'UiTag',\n", ''),
    )

    const run = spawnSync(
      process.execPath,
      ['--test', '--experimental-strip-types', join(dir, 'index.test.ts')],
      // `NODE_TEST_CONTEXT` is set for this process; inherited, it makes the child decline to run
      // any file at all and exit 0, which would be a green this case had not earned.
      { encoding: 'utf8', env: { ...process.env, NODE_TEST_CONTEXT: undefined } },
    )
    return { status: run.status ?? -1, stdout: `${run.stdout}${run.stderr}` }
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

test('CONTROL: the drift guard catches a component walked out of the contract (tuple spelling)', () => {
  const run = guardOn("const emit = defineEmits<{ remove: [] }>()")
  assert.notEqual(run.status, 0, `the guard should fail for undocumented emits:\n${run.stdout}`)
})

test('the drift guard catches a component that emits with the runtime array form', () => {
  const run = guardOn("const emit = defineEmits(['remove'])")
  assert.notEqual(
    run.status,
    0,
    'a component emitting `remove` is documented as emitting nothing, so `uiEmitters` omits it and ' +
      `UiBox renders it interactive while discarding every event — the guard did not object:\n${run.stdout}`,
  )
})

test('the drift guard catches a component that emits with the call-signature form', () => {
  const run = guardOn("const emit = defineEmits<{ (e: 'remove'): void }>()")
  assert.notEqual(
    run.status,
    0,
    'same drift, spelled the way Vue 3.2 documents it, and the guard reads it as a component that ' +
      `emits nothing:\n${run.stdout}`,
  )
})

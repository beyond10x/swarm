// Adversarial, pass 2: the drift guard's "fail closed" is keyed on the string `defineEmits`, and
// a component can emit without ever writing it.
//
// The correction rewrote `emitsOf` in `index.test.ts` to read three spellings of `defineEmits`,
// and then added what it calls the important half:
//
//   "more importantly, the reading FAILS CLOSED: a file that calls `defineEmits` at all and out of
//    which no event name can be read is drift by that fact. A fourth spelling, or a component that
//    builds its emit list some way this file has never seen, therefore fails loudly instead of
//    being silently documented as inert-free."
//
// The closing assertion is
//
//   assert.ok(names.size > 0 || !/\bdefineEmits\b/.test(componentSource), ...)
//
// so the door is closed only for a file containing the token `defineEmits`. Vue does not require
// it. `<button @click="$emit('remove')">` in the template is a complete, documented,
// compiler-supported emit, and `defineOptions({ emits: ['remove'] })` is the declared form of the
// same thing. Neither contains `defineEmits`, so `names` is empty, the guard reads "emits
// nothing", the table row may say nothing, `uiEmitters` omits the component, and `UiBox.vue`
// renders it WITHOUT `inert` — a panel a person interacts with while every event it fires is
// discarded. That is the `UiTable` defect exactly, in a spelling the corrected guard still cannot
// see.
//
// Measured, not argued, with the pass-1 harness: the shipped `index.test.ts` is run unmodified
// over a copy of this directory in which `UiTag` emits `remove` in one of these forms and the
// contract has been walked back to match what the guard can see. The CONTROL case walks the same
// drift back in the tuple spelling and asserts the child suite goes red, so a green child below is
// the guard failing to see the drift and not this harness failing to run anything.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const HERE = fileURLToPath(new URL('.', import.meta.url)).replace(/\/$/, '')

/** `UiTag` as it is written today: the script declaration, and the call in the template. */
const TUPLE_EMITS = "const emit = defineEmits<{ remove: [] }>()"
const TEMPLATE_CALL = `@click="emit('remove')"`
const TABLE_ROW_EMITS = 'emits remove'

/**
 * Run the shipped `index.test.ts` over a copy of this directory in which `UiTag` still emits
 * `remove` — with `script` in place of its `defineEmits` line and `call` in place of the template
 * call — while the contract no longer says so.
 *
 * A status of 0 means every check in `index.test.ts` passed for a component that emits and is
 * documented as one that does not.
 */
function guardOn(script: string, call: string): { status: number; stdout: string } {
  const dir = mkdtempSync(join(tmpdir(), 'ui-emits-open-'))
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
    assert.ok(tag.includes(TEMPLATE_CALL), 'UiTag.vue fires it from the template')
    writeFileSync(
      join(dir, 'UiTag.vue'),
      tag.replace(TUPLE_EMITS, script).replace(TEMPLATE_CALL, call),
    )

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

/**
 * Run the shipped `index.test.ts` over a copy in which `UiTag`'s emit declaration is replaced by
 * `script` and THE CONTRACT IS LEFT ALONE. Status 0 means the guard read the new declaration as
 * agreeing with a table that was never updated.
 */
function guardReads(script: string): { status: number; stdout: string } {
  const dir = mkdtempSync(join(tmpdir(), 'ui-emits-set-'))
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
    writeFileSync(join(dir, 'UiTag.vue'), tag.replace(TUPLE_EMITS, script))
    const run = spawnSync(
      process.execPath,
      ['--test', '--experimental-strip-types', join(dir, 'index.test.ts')],
      { encoding: 'utf8', env: { ...process.env, NODE_TEST_CONTEXT: undefined } },
    )
    return { status: run.status ?? -1, stdout: `${run.stdout}${run.stderr}` }
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

// Not "none" — the WRONG SET. `emitsOf`'s type-literal regex is
//
//   /defineEmits\s*<\s*\{([\s\S]*?)\}\s*>/
//
// whose non-greedy body stops at the first `}` followed by `>`. A payload type that is a generic
// with an object type argument — `Partial<{ why: string }>` — closes with exactly that pair, so the
// body ends INSIDE the declaration and every event named after it is invisible. `names` is not
// empty, so the fail-closed assertion does not fire; the guard reports a strict subset and calls
// the table correct while the component emits an event nothing documents.
test('the drift guard reads every event, not the ones before the first `}>` inside the declaration', () => {
  const run = guardReads("const emit = defineEmits<{ remove: [reason: Partial<{ why: string }>], clear: [] }>()")
  assert.notEqual(
    run.status,
    0,
    'UiTag now emits `remove` AND `clear`, the table still says only `remove`, and the guard is ' +
      'satisfied: its regex stopped at the `}>` closing the payload type, so it compared a subset ' +
      `of the events against the full row and found them equal:\n${run.stdout}`,
  )
})

test('CONTROL: the corrected guard still catches the drift it was written for', () => {
  const run = guardOn(TUPLE_EMITS, TEMPLATE_CALL)
  assert.notEqual(run.status, 0, `the guard should fail for undocumented emits:\n${run.stdout}`)
})

test('the drift guard catches a component that emits from its template with `$emit`', () => {
  const run = guardOn('', `@click="$emit('remove')"`)
  assert.notEqual(
    run.status,
    0,
    'a component that emits `remove` with the template `$emit` Vue documents contains no ' +
      '`defineEmits` token at all, so the fail-closed assertion never fires: the row is allowed to ' +
      'say the component emits nothing, `uiEmitters` omits it, and `UiBox.vue` renders it live ' +
      `while discarding every event — the guard did not object:\n${run.stdout}`,
  )
})

test('the drift guard catches a component that declares its emits with `defineOptions`', () => {
  const run = guardOn("defineOptions({ emits: ['remove'] })", `@click="$emit('remove')"`)
  assert.notEqual(
    run.status,
    0,
    'the emits are DECLARED, in a compiler macro sitting beside `defineEmits` in the same file, ' +
      'and the guard reads the component as one that emits nothing — the fourth spelling its own ' +
      `comment promises to fail loudly on:\n${run.stdout}`,
  )
})

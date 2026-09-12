// Adversarial, pass 2: the new template-AST guard in `uibox.inert.test.ts` rejects a DELETED
// `:inert`, and accepts a NEUTERED one.
//
// The correction replaced two `assert.match` calls on file text with a walk of the compiled
// template, and says of itself:
//
//   "Measured by an adversary: replace `<div :inert="inert || undefined">` with `<div>`, or force
//    the note's `v-if` false, and the whole suite stayed green ... So the structure is read
//    instead: the element that mounts `<component :is>` must carry an `inert` binding, and the
//    note must be shown by a condition computed from the same value."
//
// The binding assertion is `assert.match(inert.exp.content, /\binert\b/)`, and the note assertion
// is the same regex over the `v-if` expression. Both ask whether the WORD `inert` occurs in an
// expression, which is a question about spelling and not about value. So the whole behaviour this
// unit exists to deliver — eleven emitter panels, `UiTable` among them, rendered non-interactive —
// can still be turned off by an edit that leaves the binding in place, and the guard cannot see it.
//
// This file measures that rather than arguing it: the shipped guard's own predicate is reproduced
// below, verbatim in effect, and run over mutated copies of `UiBox.vue` held in memory. Nothing on
// disk is touched. The CONTROL mutant — the deletion the correction was written for — is rejected,
// so a mutant that survives below survives because the guard cannot tell, not because this harness
// fails to run.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const BOX = readFileSync(new URL('./UiBox.vue', import.meta.url), 'utf8')

/**
 * The SHIPPED guard, imported rather than copied: `uibox.inert.test.ts` runs the same function over
 * the real `UiBox.vue`, so a mutant that survives here survives the check that ships. Returns the
 * assertion message when the guard rejects, and `undefined` when it accepts.
 */
async function guardRejects(source: string): Promise<string | undefined> {
  const { assertInertGuard } = await import('./uibox.inert.guard.ts')
  try {
    await assertInertGuard(source)
  } catch (error) {
    return error instanceof Error ? error.message : String(error)
  }
  return undefined
}

/** Apply a mutation and fail loudly if the text it names is not in `UiBox.vue` any more. */
function mutate(from: string, to: string): string {
  assert.ok(BOX.includes(from), `UiBox.vue still contains ${JSON.stringify(from)}`)
  return BOX.replace(from, to)
}

// The harness proves itself: the mutation the correction was written to catch is caught.
test('CONTROL: the guard rejects UiBox with the `inert` binding deleted', async () => {
  const rejected = await guardRejects(mutate('<div :inert="inert || undefined">', '<div>'))
  assert.ok(rejected, 'the deletion mutant is the one the correction closed; it must still be closed')
})

// Mutant 1. `inert && undefined` is `undefined` for every value of `inert`, so the attribute is
// never set and every emitter panel — UiTable, UiTextInput, UiSelect, all eleven — is interactive
// again, taking input the canvas discards. The word `inert` is still in the expression, so the
// guard's `/\binert\b/` is satisfied and the suite stays green.
test('the guard rejects an `inert` binding that can never be true', async () => {
  const rejected = await guardRejects(
    mutate('<div :inert="inert || undefined">', '<div :inert="inert && undefined">'),
  )
  assert.ok(
    rejected,
    'a binding whose value is `undefined` for every input passes the guard: the guard asks whether ' +
      'the word `inert` occurs in the expression, not whether the expression can ever inert anything',
  )
})

// Mutant 2. The note's condition inverted. The guard asserts only that the condition mentions
// `inert`, so `!inert` passes — and the box then shows "Display only: nothing can keep what this
// component emits" on exactly the panels that ARE live and keep nothing, and says nothing on the
// panels that are inert. The wrong set, in the most misleading direction available.
test('the guard rejects a note shown for the components that are not inert', async () => {
  const rejected = await guardRejects(
    mutate('v-if="inert && !refusal && !feed"', 'v-if="!inert && !refusal && !feed"'),
  )
  assert.ok(
    rejected,
    'the note is shown for the complement of the inert set and the guard accepts it: a live panel ' +
      'claims to be display-only and an inert one explains nothing',
  )
})

// Mutant 3. The same neutering one layer down, in the script the guard also reads. The line still
// contains `uiEmitters.has(`, which is all the script assertion checks.
test('the guard rejects an `inert` computed that is always false', async () => {
  const rejected = await guardRejects(
    mutate(
      'const inert = computed(() => (spec.value ? uiEmitters.has(spec.value.component) : false))',
      'const inert = computed(() => (spec.value ? uiEmitters.has(spec.value.component) && false : false))',
    ),
  )
  assert.ok(
    rejected,
    '`inert` is false for every box and the guard accepts it, because it matches the call and not ' +
      'the value the call decides',
  )
})

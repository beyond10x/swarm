// A disabled action says why it is disabled.
//
// The operator hit this on swarm `123`: a place made by `POST /swarms` that no `CreateSwarm` ever
// followed has no `swarm.manager.Swarm` record, `canAct` reads `record?.state` as `undefined` and
// returns false for all five actions, and the page showed five grey buttons, a badge reading
// `Uncreated` — a word the specification does not contain — and no reason at all. Grey with no
// reason is indistinguishable from a bug in the page.
//
// The reasons are checked here rather than through a mounted component because they are the thing
// that has to be right; the wiring is checked by the last case in this file, which reads
// `SwarmView.vue` itself.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { ACTIONS, OFFERED, disabledNote, whyDisabled, type SwarmAction } from './actions.ts'

const VIEW = fileURLToPath(new URL('../views/SwarmView.vue', import.meta.url))

test('an action the state offers has no reason to give', () => {
  assert.equal(whyDisabled('Created', 'start'), undefined)
  assert.equal(whyDisabled('Running', 'pause'), undefined)
  assert.equal(whyDisabled('Paused', 'resume'), undefined)
  assert.equal(whyDisabled('Running', 'stop'), undefined)
  assert.equal(whyDisabled('Stopped', 'delete'), undefined)
})

test('every action a state does not offer says which state it is in and which would do', () => {
  for (const action of ACTIONS) {
    for (const state of ['Created', 'Running', 'Paused', 'Stopped', 'Deleted']) {
      if (OFFERED[action].includes(state)) continue
      const why = whyDisabled(state, action)
      assert.ok(why, `${action} in ${state} is disabled and gives no reason`)
      assert.ok(
        why.includes(state),
        `the reason for ${action} in ${state} does not say what state it is in: ${why}`,
      )
      for (const from of OFFERED[action]) {
        assert.ok(
          why.includes(from),
          `the reason for ${action} in ${state} does not name ${from}, which would work: ${why}`,
        )
      }
    }
  }
})

test('a place with no swarm in it says that, rather than naming a state it does not have', () => {
  for (const action of ACTIONS) {
    const why = whyDisabled(undefined, action)
    assert.ok(why, `${action} on a swarm with no record gives no reason`)
    assert.ok(
      why.includes('CreateSwarm'),
      `the reason does not say what is missing: ${why}`,
    )
    assert.ok(
      !why.includes('Uncreated'),
      `the reason invents a state the specification does not declare: ${why}`,
    )
  }
})

test('the note above the buttons says the same thing once, for the state as a whole', () => {
  const uncreated = disabledNote(undefined)
  assert.ok(uncreated?.includes('CreateSwarm'), `a swarm with no record says nothing: ${uncreated}`)

  const deleted = disabledNote('Deleted')
  assert.ok(deleted?.includes('Deleted'), `a terminal swarm says nothing: ${deleted}`)

  // A state with something to do is not nagged about the things it cannot.
  assert.equal(disabledNote('Running'), undefined)
  assert.equal(disabledNote('Created'), undefined)
})

test('the reason reaches the page: SwarmView binds it to the buttons and renders the note', () => {
  const view = readFileSync(VIEW, 'utf8')
  assert.match(
    view,
    /:title="whyDisabled\(action\)"/,
    'the buttons carry no reason, so a disabled one is grey and silent again',
  )
  assert.match(view, /disabledNote/, 'the note is never rendered')
  assert.ok(
    !view.includes("'Uncreated'"),
    'the badge still shows a word the specification does not contain',
  )
})

// Belt and braces: the action list this file iterates is the one the store issues commands for, so
// an action added to one and not the other is caught here rather than by a silent grey button.
test('the actions checked here are every action the store offers', () => {
  const store = readFileSync(fileURLToPath(new URL('./swarms.ts', import.meta.url)), 'utf8')
  const commands = store.slice(store.indexOf('const COMMANDS'))
  for (const action of ACTIONS satisfies readonly SwarmAction[]) {
    assert.ok(commands.includes(`${action}:`), `${action} is not a command the store issues`)
  }
})

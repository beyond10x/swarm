// Which lifecycle actions a swarm offers, and what to say about the ones it does not.
//
// Separate from the store because a reason is a pure function of a state and an action, and because
// the store cannot be imported by a test: it pulls in pinia, vue and the runtime client. This file
// imports nothing.
//
// The lifecycle itself still belongs to `src/core/domains/manager.yaml` — the server's answer is
// the one that decides, and issuing an action the model refuses gets its `wrong-state` outcome and
// the declared error. What is here is only what the page offers and what it says when it does not.

/** Every lifecycle action the page offers, in the order it shows them. */
export const ACTIONS = ['start', 'pause', 'resume', 'stop', 'delete'] as const

export type SwarmAction = (typeof ACTIONS)[number]

/** The states each action is offered from. The same table `manager.yaml` declares transitions for. */
export const OFFERED: Record<SwarmAction, string[]> = {
  start: ['Created', 'Stopped'],
  pause: ['Running'],
  resume: ['Paused'],
  stop: ['Running', 'Paused'],
  delete: ['Created', 'Stopped'],
}

/** `['a', 'b', 'c']` as `a, b or c`. */
function or(states: string[]): string {
  if (states.length <= 1) return states[0] ?? ''
  return `${states.slice(0, -1).join(', ')} or ${states[states.length - 1]}`
}

/**
 * Why this action is disabled, or `undefined` when it is not.
 *
 * `undefined` for the state means there is no `swarm.manager.Swarm` record at all — a place made by
 * `POST /swarms` that no `CreateSwarm` followed. That is a different sentence from a wrong state,
 * and it is the one the operator needed: five grey buttons and a badge reading `Uncreated` said
 * nothing about what was missing or what to do next.
 */
export function whyDisabled(state: string | undefined, action: SwarmAction): string | undefined {
  if (state === undefined) {
    return `Nothing has been created here yet: this is a place for a swarm, and ${action} needs the CreateSwarm command to have landed.`
  }
  if (OFFERED[action].includes(state)) return undefined
  return `This swarm is ${state}; ${action} acts from ${or(OFFERED[action])}.`
}

/**
 * One line above the buttons for a swarm that can do nothing at all, or `undefined` when at least
 * one action is offered.
 *
 * A per-button reason is only reachable by pointing at the button. When every action is disabled
 * there is nothing to point at that would help, which is exactly the two cases below.
 */
export function disabledNote(state: string | undefined): string | undefined {
  if (state === undefined) {
    return 'This is a place for a swarm, not a swarm: no CreateSwarm has landed here, so there is nothing to start, pause, resume, stop or delete. Removing the place is DELETE /swarms/{slug}.'
  }
  if (ACTIONS.some((action) => OFFERED[action].includes(state))) return undefined
  return `This swarm is ${state}, which the specification calls an end: no lifecycle action acts from it. Removing what is left on disk is DELETE /swarms/{slug}.`
}

// The swarm store, backed by the runtime.
//
// There is no seed file and no localStorage. A swarm is what the server's log says it is, every
// change is a command the specification declares, and the canvas redraws because the server said
// something happened — not because this store guessed.
//
// The state machine is not here either. `Created → Running → Paused …` lives in
// `src/core/domains/manager.yaml`, and asking the server to start a swarm that cannot be started
// gets a refusal from the specification rather than from a table in this file. A copy of those
// rules here would be a second answer to a question the model already answers.
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import * as runtime from '@/runtime'
import type { Instance } from '@/runtime'

/** Which command each action issues. The lifecycle behind them belongs to the specification. */
const COMMANDS = {
  start: 'swarm.manager.StartSwarm',
  pause: 'swarm.manager.PauseSwarm',
  resume: 'swarm.manager.ResumeSwarm',
  stop: 'swarm.manager.StopSwarm',
  delete: 'swarm.manager.DeleteSwarm',
} as const

export type SwarmAction = keyof typeof COMMANDS

const SWARM = 'swarm.manager.Swarm'
const GOAL = 'swarm.goal.Goal'

/** One swarm as the UI holds it: its slug, its record, and everything on its canvas. */
export interface Held {
  slug: string
  /** The `swarm.manager.Swarm` instance, once `CreateSwarm` has made one. */
  record?: Instance
  /** Every instance, by entity. */
  canvas: runtime.Canvas
}

/** A field of an instance, or undefined when nothing has written it. */
export function field(instance: Instance | undefined, name: string): unknown {
  const value = instance?.fields[name]
  return value === null ? undefined : value
}

export const useSwarmStore = defineStore('swarms', () => {
  const held = ref<Held[]>([])
  const loaded = ref(false)
  const problem = ref<string | undefined>()
  const shape = ref<runtime.Shape | undefined>()
  const watching = new Map<string, () => void>()

  /** Every swarm that has not been deleted. */
  const visible = computed(() =>
    held.value.filter((swarm) => swarm.record?.state !== 'Deleted'),
  )

  function getSwarm(slug: string): Held | undefined {
    return held.value.find((swarm) => swarm.slug === slug)
  }

  /** Reads one swarm's canvas from the server, replacing whatever was held. */
  async function refresh(slug: string): Promise<void> {
    const canvas = await runtime.canvas(slug)
    const record = canvas[SWARM]?.[0]
    const existing = getSwarm(slug)
    if (existing) {
      existing.canvas = canvas
      existing.record = record
    } else {
      held.value.push({ slug, canvas, record })
    }
  }

  /** Opens the stream, so the canvas redraws when the server says something happened. */
  function follow(slug: string): void {
    if (watching.has(slug)) return
    const stop = runtime.watch(
      slug,
      () => void refresh(slug),
      // Dropped: the server would rather we reload than be fed a gap we cannot see.
      () => void refresh(slug),
    )
    watching.set(slug, stop)
  }

  /** Stops watching every swarm. */
  function unfollow(): void {
    for (const stop of watching.values()) stop()
    watching.clear()
  }

  /** Reads the shape of the system and every swarm the server holds. */
  async function load(): Promise<void> {
    problem.value = undefined
    try {
      shape.value = await runtime.shape()
      const slugs = await runtime.swarms()
      held.value = []
      for (const slug of slugs) {
        await refresh(slug)
        follow(slug)
      }
      loaded.value = true
    } catch (why) {
      // A runtime that is not running is the likeliest reason to be here, and saying so beats an
      // empty screen that looks like a swarm nobody has made yet.
      problem.value =
        why instanceof Error ? why.message : 'the runtime could not be reached'
      loaded.value = true
    }
  }

  /** Makes a place for a swarm, then the record itself. */
  async function createSwarm(input: {
    displayName: string
    goal?: string
  }): Promise<string | undefined> {
    const slug = slugify(input.displayName)
    problem.value = undefined
    try {
      await runtime.open(slug)
      await runtime.issue(slug, 'swarm.manager.CreateSwarm', {
        display_name: input.displayName,
        tmux_session: slug,
        home: `swarms/${slug}`,
        // The model does not read a clock; the caller hands one in.
        created_at: new Date().toISOString(),
      })
      await refresh(slug)
      follow(slug)

      // A swarm with no goal has nothing to pursue, and the loop has nothing to turn.
      if (input.goal?.trim()) {
        const record = getSwarm(slug)?.record
        if (record) {
          await runtime.issue(slug, 'swarm.goal.SetGoal', {
            swarm_id: record.id,
            text: input.goal.trim(),
          })
          await refresh(slug)
        }
      }
      return slug
    } catch (why) {
      problem.value = why instanceof Error ? why.message : 'the swarm could not be created'
      return undefined
    }
  }

  /**
   * Issues one of the lifecycle commands.
   *
   * Whether it is allowed is the specification's answer, not this store's: a swarm that cannot be
   * started answers with its own `wrong-state` outcome and the declared error.
   */
  async function act(slug: string, action: SwarmAction): Promise<boolean> {
    const swarm = getSwarm(slug)
    if (!swarm?.record) return false
    problem.value = undefined

    const stamp = new Date().toISOString()
    const input: Record<string, unknown> = { swarm_id: swarm.record.id }
    if (action === 'start') input.started_at = stamp
    if (action === 'stop') input.stopped_at = stamp

    try {
      const issued = await runtime.issue(slug, COMMANDS[action], input)
      await refresh(slug)
      if (issued.error) {
        problem.value = `${action} was refused: ${issued.error}`
        return false
      }
      return true
    } catch (why) {
      problem.value = why instanceof Error ? why.message : `the swarm could not ${action}`
      return false
    }
  }

  const startSwarm = (slug: string) => act(slug, 'start')
  const pauseSwarm = (slug: string) => act(slug, 'pause')
  const resumeSwarm = (slug: string) => act(slug, 'resume')
  const stopSwarm = (slug: string) => act(slug, 'stop')
  const deleteSwarm = (slug: string) => act(slug, 'delete')

  /** The goal a swarm is pursuing, when it has one. */
  function goalOf(slug: string): Instance | undefined {
    return getSwarm(slug)?.canvas[GOAL]?.[0]
  }

  /** Whether an action is worth offering. The server decides; this only avoids the obvious. */
  function canAct(slug: string, action: SwarmAction): boolean {
    const state = getSwarm(slug)?.record?.state
    if (!state) return false
    const offered: Record<SwarmAction, string[]> = {
      start: ['Created', 'Stopped'],
      pause: ['Running'],
      resume: ['Paused'],
      stop: ['Running', 'Paused'],
      delete: ['Created', 'Stopped'],
    }
    return offered[action].includes(state)
  }

  return {
    held, loaded, visible, problem, shape,
    load, refresh, getSwarm, goalOf, canAct, createSwarm, unfollow,
    startSwarm, pauseSwarm, resumeSwarm, stopSwarm, deleteSwarm,
  }
})

/** A display name, as a slug the filesystem and the log will both accept. */
export function slugify(value: string): string {
  const slug = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
  return slug || `swarm-${Date.now()}`
}

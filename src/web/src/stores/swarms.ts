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
import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import * as runtime from '@/runtime'
import type { AgentEvent, Change, Instance, Recorded, Spent, Status } from '@/runtime'

/** How many live changes are kept per swarm before the oldest is dropped. */
const RING = 400
/** How long a just-changed instance stays highlighted on the canvas, in ms. */
const GLOW_MS = 1800
/** How often the runtime's status is re-read, in ms. */
const STATUS_MS = 2000
/** How many coordinator turns are held in memory per swarm. Older ones are on disk, via /turns. */
const TURNS_HELD = 6

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

/** What is being watched, live, for one swarm. */
export interface Live {
  /** Whether the event stream is open right now. */
  connected: boolean
  /** Every change heard since the stream opened, oldest first, capped at `RING`. */
  changes: Change[]
  /** What the log held when the swarm was opened, oldest first. */
  history: Recorded[]
  /** When each instance last changed, by id, for the canvas to glow. */
  changedAt: Record<string, number>
  /** Coordinator turns heard on the stream, newest first, plus any loaded from disk. */
  turns: Turn[]
}

function fresh(): Live {
  return { connected: false, changes: [], history: [], changedAt: {}, turns: [] }
}

/** One coordinator turn as watched: what it was asked, what it did, what it cost. */
export interface Turn {
  /** `<goal>:<iterations>`, which is also how the server names it. */
  key: string
  goal: string
  iterations: number
  phase: 'asking' | 'answered' | 'unfinished' | 'recorded'
  startedAt: string
  endedAt?: string
  reached?: boolean
  note?: string
  error?: string
  spent?: Spent
  events: AgentEvent[]
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
  const live = ref<Record<string, Live>>({})
  const status = shallowRef<Status | undefined>()
  /** When the status was last read, so a stale one can be told from a live one. */
  const statusAt = ref(0)
  /** Now, ticking once a second, so countdowns and "3s ago" are reactive without N timers. */
  const clock = ref(Date.now())
  let clockTimer: ReturnType<typeof setInterval> | undefined
  let statusTimer: ReturnType<typeof setInterval> | undefined
  const pendingRefresh = new Map<string, ReturnType<typeof setTimeout>>()

  /** Every swarm that has not been deleted. */
  const visible = computed(() =>
    held.value.filter((swarm) => swarm.record?.state !== 'Deleted'),
  )

  function getSwarm(slug: string): Held | undefined {
    return held.value.find((swarm) => swarm.slug === slug)
  }

  /**
   * Rows of the views something on the canvas is drawn from, by slug then view name.
   *
   * A `Box{kind: Ui}` may name a view to be fed from, and a view is computed by the server against
   * its own world — it is not on the canvas payload and cannot be derived from it. Only views
   * somebody asked for are read: a swarm declares many and a panel watches one.
   */
  const views = ref<Record<string, Record<string, ViewRead>>>({})
  /** Which views are being drawn, per swarm, so a change re-reads exactly those. */
  const viewed = new Map<string, Set<string>>()

  /** Asks for one view's rows, now and after every change. Idempotent. */
  function followView(slug: string, name: string): void {
    let wanted = viewed.get(slug)
    if (!wanted) viewed.set(slug, (wanted = new Set()))
    if (wanted.has(name)) return
    wanted.add(name)
    void readView(slug, name)
  }

  /**
   * One view as last read: its rows, or why they could not be read.
   *
   * Not bare rows. A view that was refused and a view with nothing in it both end up as `[]`, and
   * a panel showing "No rows" for the first is stating the one thing that is certainly untrue.
   */
  function viewState(slug: string, name: string): ViewRead | undefined {
    return views.value[slug]?.[name]
  }

  async function readView(slug: string, name: string): Promise<void> {
    const held = (views.value[slug] ??= {})
    try {
      held[name] = { rows: await runtime.view(slug, name) }
    } catch (why) {
      // The rows already read are kept beside the failure: a view that has gone away had rows a
      // moment ago, and blanking the panel would hide that it ever did.
      held[name] = {
        rows: held[name]?.rows,
        error: why instanceof Error ? why.message : 'the runtime could not be reached',
      }
    }
  }

  /** Re-reads every view being drawn for one swarm. */
  function refreshViews(slug: string): void {
    for (const name of viewed.get(slug) ?? []) void readView(slug, name)
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
    // Whatever is drawn from a view is redrawn with the canvas: one change may have written rows
    // the canvas payload says nothing about.
    refreshViews(slug)
  }

  function liveOf(slug: string): Live {
    // Read back through the ref rather than returning what `??=` yields: the assignment expression
    // is the raw object, and a write to that is a write Vue never sees.
    if (!live.value[slug]) live.value[slug] = fresh()
    return live.value[slug]!
  }

  /** Puts one instance on the canvas as the stream reported it, without a round trip. */
  function patch(slug: string, instance: Instance): void {
    const swarm = getSwarm(slug)
    if (!swarm) return
    const held = (swarm.canvas[instance.entity] ??= [])
    const at = held.findIndex((existing) => existing.id === instance.id)
    if (at >= 0) held[at] = instance
    else held.push(instance)
    if (instance.entity === SWARM) swarm.record = instance
    liveOf(slug).changedAt[instance.id] = Date.now()
  }

  /** A refresh soon, coalescing a burst of changes into one read. */
  function refreshSoon(slug: string): void {
    const pending = pendingRefresh.get(slug)
    if (pending) clearTimeout(pending)
    pendingRefresh.set(
      slug,
      setTimeout(() => {
        pendingRefresh.delete(slug)
        void refresh(slug)
      }, 150),
    )
  }

  /** The turn a change belongs to, made if it is the first heard of it. */
  function turnOf(watched: Live, goal: string, iterations: number, at: string): Turn {
    const key = `${goal}:${iterations}`
    let turn = watched.turns.find((held) => held.key === key)
    if (!turn) {
      turn = { key, goal, iterations, phase: 'asking', startedAt: at, events: [] }
      watched.turns.unshift(turn)
      if (watched.turns.length > TURNS_HELD) watched.turns.length = TURNS_HELD
    }
    return turn
  }

  function heard(slug: string, change: Change): void {
    const watched = liveOf(slug)
    if (change.kind === 'agent') {
      // The run itself is kept with its turn, not in the ring: one run is hundreds of lines.
      const turn = turnOf(watched, change.goal, change.iterations, change.at)
      turn.events.push({ seq: change.seq, ...change.event })
      turn.spent = change.spent
      return
    }
    watched.changes.push(change)
    if (watched.changes.length > RING) watched.changes.splice(0, watched.changes.length - RING)
    if (change.kind === 'turn') {
      const turn = turnOf(watched, change.goal, change.iterations, change.at)
      if (change.phase !== 'asking') {
        turn.phase = change.phase
        turn.endedAt = change.at
        turn.reached = change.reached ?? undefined
        turn.note = change.note ?? undefined
        turn.error = change.error ?? undefined
      }
      if (change.spent) turn.spent = change.spent
    }
    if ('instance' in change && change.instance) patch(slug, change.instance)
    // A turn names no instance, and a reload may have changed anything: read the canvas again.
    refreshSoon(slug)
  }

  /** Loads a recorded turn from disk into the held list, unless it is already there. */
  async function loadTurn(slug: string, record: runtime.TurnRecord): Promise<void> {
    const watched = liveOf(slug)
    const key = `${record.goal}:${record.iterations}`
    if (watched.turns.some((held) => held.key.startsWith(record.goal) && held.iterations === record.iterations)) return
    const events = await runtime.turn(slug, record.name)
    const spent = sumSpent(events)
    const first = events[0]
    watched.turns.push({
      key,
      goal: record.goal,
      iterations: record.iterations,
      phase: 'recorded',
      startedAt: (first?.at as string | undefined) ?? '',
      endedAt: record.ended_at ?? undefined,
      reached: record.reached ?? undefined,
      note: record.note ?? undefined,
      spent,
      events,
    })
    watched.turns.sort((a, b) => b.iterations - a.iterations)
  }

  /** The turn running right now, when one is. */
  function liveTurn(slug: string): Turn | undefined {
    return live.value[slug]?.turns.find((turn) => turn.phase === 'asking')
  }

  /** Opens the stream, so the canvas redraws when the server says something happened. */
  function follow(slug: string): void {
    if (watching.has(slug)) return
    const watched = liveOf(slug)
    void runtime.log(slug).then((history) => {
      watched.history = history
    })
    const stop = runtime.watch(slug, {
      onOpen: () => {
        watched.connected = true
        // Whatever happened while the stream was down is in the log, not in the stream.
        void refresh(slug)
      },
      onChange: (change) => heard(slug, change),
      // Dropped: the server would rather we reload than be fed a gap we cannot see.
      onDropped: () => {
        watched.connected = false
        refreshSoon(slug)
      },
    })
    watching.set(slug, stop)
  }

  /** Stops watching every swarm. */
  function unfollow(): void {
    for (const stop of watching.values()) stop()
    watching.clear()
    for (const watched of Object.values(live.value)) watched.connected = false
  }

  /** Reads where the runtime is. Failing is recorded as a stale status, not as a problem. */
  async function pollStatus(): Promise<void> {
    try {
      status.value = await runtime.status()
      statusAt.value = Date.now()
      problem.value = undefined
    } catch {
      // The bar shows the status as stale; there is nothing else to say until it comes back.
    }
  }

  /** Whether the runtime answered recently. */
  const reachable = computed(() => clock.value - statusAt.value < STATUS_MS * 2.5)

  /** Seconds until the trigger next fires, or undefined when it has not said. */
  const nextTickIn = computed(() => {
    const at = status.value?.next_tick_at
    if (!at) return undefined
    return Math.max(0, Math.round((Date.parse(at) - clock.value) / 1000))
  })

  /** Starts the clock and the status poll. Idempotent. */
  function wake(): void {
    clockTimer ??= setInterval(() => {
      clock.value = Date.now()
    }, 1000)
    if (!statusTimer) {
      void pollStatus()
      statusTimer = setInterval(() => void pollStatus(), STATUS_MS)
    }
  }

  /** The instances that changed within the last moment, for the canvas to glow. */
  function recentlyChanged(slug: string): Set<string> {
    const now = clock.value
    const changed = live.value[slug]?.changedAt ?? {}
    return new Set(Object.keys(changed).filter((id) => now - changed[id]! < GLOW_MS))
  }

  /**
   * Whether the loop has stopped asking about this goal, and why.
   *
   * Read from the status rather than from the change stream, so a page opened after the cap was
   * announced still says which cap and what it had spent.
   */
  function capOn(slug: string, goalId: string | undefined) {
    if (!goalId) return undefined
    return (status.value?.capped ?? []).find(
      (entry) => entry.swarm === slug && entry.goal === goalId,
    )
  }

  /** The last thing the coordinator was heard saying about a goal. */
  function lastTurn(slug: string, goalId: string | undefined) {
    const changes = live.value[slug]?.changes ?? []
    for (let index = changes.length - 1; index >= 0; index -= 1) {
      const change = changes[index]!
      if (change.kind === 'turn' && (!goalId || change.goal === goalId)) return change
    }
    return undefined
  }

  /** Reads the shape of the system and every swarm the server holds. */
  async function load(): Promise<void> {
    problem.value = undefined
    try {
      shape.value = await runtime.shape()
      const slugs = await runtime.swarms()
      held.value = []
      // Read every canvas, but follow none: a stream is a connection held open, and a browser
      // allows six per host. The page that shows a swarm follows it; the list reads the status.
      await Promise.all(slugs.map((slug) => refresh(slug)))
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
      void pollStatus()
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
    held, loaded, visible, problem, shape, live, status, statusAt, clock, reachable, nextTickIn,
    load, refresh, getSwarm, goalOf, canAct, createSwarm, follow, unfollow, wake, pollStatus,
    recentlyChanged, lastTurn, loadTurn, liveTurn, capOn,
    views, followView, viewState,
    startSwarm, pauseSwarm, resumeSwarm, stopSwarm, deleteSwarm,
  }
})

/** One view as last read: the rows it held, and the reason if the last read failed. */
export interface ViewRead {
  rows?: Record<string, unknown>[]
  error?: string
}

/** What a recorded run cost, summed the same way the server sums a live one. */
function sumSpent(events: AgentEvent[]): Spent {
  const spent: Spent = {
    input_tokens: 0, output_tokens: 0, cache_read_tokens: 0, cache_write_tokens: 0,
    cost_usd: null, requests: 0, tool_calls: 0, model: null, duration_ms: null, thinking_tokens: null,
  }
  const seen = new Set<string>()
  for (const event of events) {
    if (event.event === 'usage') {
      // One request's usage arrives once per content block; count the request once.
      const request = event.request_id
      if (typeof request === 'string') {
        if (seen.has(request)) continue
        seen.add(request)
      }
      const usage = (event.usage ?? {}) as Record<string, number | null>
      spent.requests += 1
      spent.input_tokens += usage.input_tokens ?? 0
      spent.output_tokens += usage.output_tokens ?? 0
      spent.cache_read_tokens += usage.cache_read_input_tokens ?? 0
      spent.cache_write_tokens += usage.cache_creation_input_tokens ?? 0
      if (typeof event.model === 'string') spent.model = event.model
    } else if (event.event === 'tool.requested') {
      spent.tool_calls += 1
    } else if (event.event === 'session.ended') {
      spent.cost_usd = (event.total_cost_usd as number | null) ?? null
      spent.duration_ms = (event.duration_ms as number | null) ?? null
      // The vendor's own total for the run replaces the sum.
      const usage = event.usage as Record<string, number | null> | null | undefined
      if (usage) {
        spent.input_tokens = usage.input_tokens ?? spent.input_tokens
        spent.output_tokens = usage.output_tokens ?? spent.output_tokens
        spent.cache_read_tokens = usage.cache_read_input_tokens ?? spent.cache_read_tokens
        spent.cache_write_tokens = usage.cache_creation_input_tokens ?? spent.cache_write_tokens
        spent.thinking_tokens = usage.thinking_tokens ?? null
      }
    }
  }
  return spent
}

/** A display name, as a slug the filesystem and the log will both accept. */
export function slugify(value: string): string {
  const slug = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
  return slug || `swarm-${Date.now()}`
}

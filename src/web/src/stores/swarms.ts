// The swarm store. Seed swarms come from /data/seed.json and are never persisted; swarms created in
// the UI live in localStorage under `swarm.swarms`. Lifecycle transitions follow SwarmState:
//   Created → Running (start; the orchestrator bootstraps everything)
//   Running → Paused (pause), Paused → Running (resume)
//   Running | Paused → Stopped (stop), Created | Stopped → Deleted (delete)
// Anything else is refused by returning false.
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { Limits, Seed, Swarm, SwarmState } from '@/model'
import {
  SWARM_HOME, bootstrapBoxes, bootstrapFlows, bootstrapSchemas, orchestratorAgent, slugify,
} from '@/composables/useBootstrap'

export const STORAGE_KEY = 'swarm.swarms'
const SEED_URL = `${import.meta.env.BASE_URL}data/seed.json`

export type SwarmAction = 'start' | 'pause' | 'resume' | 'stop' | 'delete'

const TRANSITIONS: Record<SwarmAction, { from: SwarmState[]; to: SwarmState }> = {
  start: { from: ['Created'], to: 'Running' },
  pause: { from: ['Running'], to: 'Paused' },
  resume: { from: ['Paused'], to: 'Running' },
  stop: { from: ['Running', 'Paused'], to: 'Stopped' },
  delete: { from: ['Created', 'Stopped'], to: 'Deleted' },
}

export function canTransition(swarm: Swarm | undefined, action: SwarmAction): boolean {
  return !!swarm && TRANSITIONS[action].from.includes(swarm.state)
}

export interface CreateSwarmInput {
  displayName: string
  objective: string
  limits: Limits
}

export const useSwarmStore = defineStore('swarms', () => {
  const swarms = ref<Swarm[]>([])
  const loaded = ref(false)
  const seedIds = new Set<string>()
  let loading: Promise<void> | undefined

  const visible = computed(() => swarms.value.filter((s) => s.state !== 'Deleted'))

  function readLocal(): Swarm[] {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      const parsed: unknown = raw ? JSON.parse(raw) : []
      return Array.isArray(parsed) ? (parsed as Swarm[]) : []
    } catch {
      return []
    }
  }

  function persist(): void {
    const own = swarms.value.filter((s) => !seedIds.has(s.swarmId) && s.state !== 'Deleted')
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(own))
    } catch {
      /* storage full or unavailable: the in-memory state still stands */
    }
  }

  /** Fetches the seed once and merges in the swarms saved in localStorage. Safe to call repeatedly. */
  async function load(): Promise<void> {
    if (loaded.value) return
    if (loading) return loading
    loading = (async () => {
      let seed: Swarm[] = []
      try {
        const res = await fetch(SEED_URL)
        if (res.ok) seed = ((await res.json()) as Seed).swarms ?? []
      } catch {
        seed = []
      }
      seedIds.clear()
      for (const s of seed) seedIds.add(s.swarmId)
      const local = readLocal().filter((s) => !seedIds.has(s.swarmId))
      swarms.value = [...seed, ...local]
      loaded.value = true
    })()
    return loading
  }

  function getSwarm(id: string): Swarm | undefined {
    return swarms.value.find((s) => s.swarmId === id)
  }

  function isSeed(id: string): boolean {
    return seedIds.has(id)
  }

  function createSwarm(input: CreateSwarmInput): Swarm {
    const swarm: Swarm = {
      swarmId: crypto.randomUUID(),
      displayName: input.displayName.trim(),
      objective: input.objective.trim(),
      tmuxSession: slugify(input.displayName),
      home: SWARM_HOME,
      state: 'Created',
      limits: { ...input.limits },
      createdAt: new Date().toISOString(),
      agents: [],
      boxes: [],
      connections: [],
      flows: [],
      schemas: [],
      decisions: [],
    }
    swarms.value.push(swarm)
    persist()
    return swarm
  }

  function transition(id: string, action: SwarmAction): Swarm | undefined {
    const swarm = getSwarm(id)
    if (!canTransition(swarm, action)) return undefined
    swarm!.state = TRANSITIONS[action].to
    return swarm
  }

  /** Created → Running. The orchestrator bootstraps everything the swarm needs to communicate. */
  function startSwarm(id: string): boolean {
    const swarm = transition(id, 'start')
    if (!swarm) return false
    swarm.startedAt = new Date().toISOString()
    if (!swarm.agents.some((a) => a.agentId === 'orchestrator')) swarm.agents.unshift(orchestratorAgent())
    if (swarm.schemas.length === 0) swarm.schemas = bootstrapSchemas()
    if (swarm.boxes.length === 0) swarm.boxes = bootstrapBoxes()
    if (swarm.flows.length === 0) swarm.flows = bootstrapFlows()
    persist()
    return true
  }

  function pauseSwarm(id: string): boolean {
    if (!transition(id, 'pause')) return false
    persist()
    return true
  }

  function resumeSwarm(id: string): boolean {
    if (!transition(id, 'resume')) return false
    persist()
    return true
  }

  function stopSwarm(id: string): boolean {
    if (!transition(id, 'stop')) return false
    persist()
    return true
  }

  function deleteSwarm(id: string): boolean {
    if (!transition(id, 'delete')) return false
    persist()
    return true
  }

  return {
    swarms, loaded, visible,
    load, getSwarm, isSeed, createSwarm,
    startSwarm, pauseSwarm, resumeSwarm, stopSwarm, deleteSwarm,
  }
})

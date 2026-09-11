<script setup lang="ts">
// One swarm: its lifecycle in the toolbar, then tabs — canvas (W2's SwarmCanvas), agents, flows,
// decisions, limits.
import { computed, onMounted, ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import {
  UiButton, UiEmptyState, UiKeyValue, UiSpinner, UiStateBadge, UiTable, UiTabs, UiToolbar,
} from '@/components/ui'
import type { Swarm } from '@/model'
import { canTransition, useSwarmStore, type SwarmAction } from '@/stores/swarms'
import SwarmCanvas from '@/views/SwarmCanvas.vue'

const props = defineProps<{ id: string }>()
const store = useSwarmStore()
const router = useRouter()
onMounted(() => { void store.load() })

const swarm = computed<Swarm | undefined>(() => store.getSwarm(props.id))

const tab = ref('canvas')
const tabs = [
  { value: 'canvas', label: 'Canvas' },
  { value: 'agents', label: 'Agents' },
  { value: 'flows', label: 'Flows' },
  { value: 'decisions', label: 'Decisions' },
  { value: 'limits', label: 'Limits' },
]

// --- lifecycle ------------------------------------------------------------------------------
const can = (action: SwarmAction): boolean => canTransition(swarm.value, action)

function act(action: SwarmAction): void {
  const id = props.id
  switch (action) {
    case 'start': store.startSwarm(id); break        // the orchestrator bootstraps everything
    case 'pause': store.pauseSwarm(id); break
    case 'resume': store.resumeSwarm(id); break
    case 'stop': store.stopSwarm(id); break
    case 'delete':
      if (store.deleteSwarm(id)) void router.push({ name: 'splash' })
      break
  }
}

function goHome(): void {
  void router.push({ name: 'splash' })
}

// --- tables ---------------------------------------------------------------------------------
type Column = { key: string; label: string; width?: string; align?: 'left' | 'center' | 'right' }
const agentColumns: Column[] = [
  { key: 'agentId', label: 'Agent' },
  { key: 'role', label: 'Role' },
  { key: 'harness', label: 'Harness', width: '90px' },
  { key: 'state', label: 'State', width: '120px' },
  { key: 'window', label: 'Window', width: '90px', align: 'right' },
  { key: 'blockedOn', label: 'Blocked on' },
  { key: 'unread', label: 'Unread', width: '80px', align: 'right' },
]
const agentRows = computed<Record<string, unknown>[]>(() =>
  (swarm.value?.agents ?? []).map((a) => ({
    agentId: a.agentId,
    role: a.role,
    harness: a.harness,
    state: a.state,
    window: a.host?.windowIndex ?? a.host?.tmuxWindowId ?? '',
    blockedOn: a.blockedOn ?? '',
    unread: a.unread ?? 0,
  })),
)

const flowColumns: Column[] = [
  { key: 'name', label: 'Flow' },
  { key: 'binds', label: 'Binds' },
  { key: 'steps', label: 'Steps', width: '80px', align: 'right' },
]
const flowRows = computed<Record<string, unknown>[]>(() =>
  (swarm.value?.flows ?? []).map((f) => ({
    flowId: f.flowId,
    name: f.name,
    binds: f.binds ? `${f.binds.entity} · ${f.binds.state}` : '',
    steps: f.steps.length,
  })),
)

const decisionColumns: Column[] = [
  { key: 'number', label: '#', width: '56px', align: 'right' },
  { key: 'question', label: 'Decision' },
  { key: 'defaultOnSilence', label: 'Default on silence' },
  { key: 'state', label: 'State', width: '110px' },
]
const decisionRows = computed<Record<string, unknown>[]>(() =>
  (swarm.value?.decisions ?? []).map((d) => ({
    number: d.number,
    question: d.question,
    defaultOnSilence: d.defaultOnSilence,
    state: d.state,
  })),
)

const fmt = (n: number | undefined, unit = ''): string =>
  n === undefined ? '—' : `${n.toLocaleString()}${unit ? ` ${unit}` : ''}`

const limitItems = computed(() => {
  const s = swarm.value
  if (!s) return []
  return [
    { key: 'budget', value: fmt(s.limits.budgetTokens, 'tokens') },
    { key: 'max agents', value: fmt(s.limits.maxAgents) },
    { key: 'disk floor', value: fmt(s.limits.diskFloorGb, 'GB') },
    { key: 'memory file', value: fmt(s.limits.memoryFileBytes, 'bytes') },
    { key: 'max defers', value: fmt(s.limits.maxDefers) },
    { key: 'tmux session', value: s.tmuxSession },
    { key: 'home', value: s.home },
    { key: 'created', value: s.createdAt },
    { key: 'started', value: s.startedAt ?? '—' },
  ]
})
</script>

<template>
  <div class="page">
    <UiSpinner v-if="!store.loaded" size="md" />

    <UiEmptyState v-else-if="!swarm" title="Swarm not found" :text="`No swarm with id ${id}.`" icon="?">
      <UiButton variant="secondary" @click="goHome">Back to swarms</UiButton>
    </UiEmptyState>

    <template v-else>
      <UiToolbar>
        <div class="head">
          <div class="title">
            <h1 class="name">{{ swarm.displayName }}</h1>
            <UiStateBadge :state="swarm.state" />
          </div>
          <p class="objective">{{ swarm.objective }}</p>
        </div>
        <template #right>
          <UiButton variant="primary" :disabled="!can('start')" @click="act('start')">Start</UiButton>
          <UiButton variant="secondary" :disabled="!can('pause')" @click="act('pause')">Pause</UiButton>
          <UiButton variant="secondary" :disabled="!can('resume')" @click="act('resume')">Resume</UiButton>
          <UiButton variant="secondary" :disabled="!can('stop')" @click="act('stop')">Stop</UiButton>
          <UiButton variant="danger" :disabled="!can('delete')" @click="act('delete')">Delete</UiButton>
        </template>
      </UiToolbar>

      <UiTabs v-model="tab" :tabs="tabs" />

      <section class="body">
        <SwarmCanvas v-if="tab === 'canvas'" :swarm="swarm" />

        <UiTable v-else-if="tab === 'agents'" :columns="agentColumns" :rows="agentRows" row-key="agentId">
          <template #cell-state="{ row }">
            <UiStateBadge :state="String(row.state)" />
          </template>
          <template #empty>No agents yet — Start the swarm and the orchestrator spawns them.</template>
        </UiTable>

        <UiTable v-else-if="tab === 'flows'" :columns="flowColumns" :rows="flowRows" row-key="flowId">
          <template #cell-name="{ row }">
            <RouterLink class="link" :to="{ name: 'flow', params: { id: swarm.swarmId, flowId: String(row.flowId) } }">
              {{ row.name }}
            </RouterLink>
          </template>
          <template #empty>No flows yet — Start the swarm and the orchestrator installs its four.</template>
        </UiTable>

        <UiTable v-else-if="tab === 'decisions'" :columns="decisionColumns" :rows="decisionRows" row-key="number">
          <template #cell-state="{ row }">
            <UiStateBadge :state="String(row.state)" />
          </template>
          <template #empty>Nothing waits on Timo.</template>
        </UiTable>

        <UiKeyValue v-else-if="tab === 'limits'" :items="limitItems" mono />
      </section>
    </template>
  </div>
</template>

<style scoped>
.page { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.head { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.title { display: flex; align-items: center; gap: var(--space-3); }
.name { margin: 0; font-size: 18px; font-weight: 600; }
.objective { margin: 0; color: var(--color-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.body { flex: 1; display: flex; flex-direction: column; min-height: 0; padding: var(--space-4); }
.link { color: var(--color-accent); text-decoration: none; }
.link:hover { text-decoration: underline; }
</style>

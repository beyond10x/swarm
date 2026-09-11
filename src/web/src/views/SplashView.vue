<script setup lang="ts">
// The splash: big tiles in the middle, one per swarm, plus one that creates a new swarm from a
// name, an objective and its limits. The orchestrator bootstraps the rest on Start (SwarmView).
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  UiButton, UiEmptyState, UiModal, UiNumberInput, UiSpinner, UiStateBadge, UiTextArea, UiTextInput, UiTile,
} from '@/components/ui'
import type { Limits, Swarm } from '@/model'
import { useSwarmStore } from '@/stores/swarms'

const store = useSwarmStore()
const router = useRouter()
onMounted(() => { void store.load() })

const swarms = computed(() => store.visible)

function openDecisions(s: Swarm): number {
  return s.decisions.filter((d) => d.state === 'Open').length
}

function accentFor(s: Swarm): string {
  switch (s.state) {
    case 'Running': return 'var(--color-ok)'
    case 'Paused': return 'var(--color-warn)'
    case 'Created': return 'var(--color-info)'
    default: return 'var(--color-text-muted)'
  }
}

function open(s: Swarm): void {
  void router.push({ name: 'swarm', params: { id: s.swarmId } })
}

// --- new-swarm form -------------------------------------------------------------------------
// Defaults are the real swarm's: CHARTER.md §4 (8192 bytes), §5.7 (3 defers), AGENTS.md §6 (40G floor).
interface FormState {
  displayName: string
  objective: string
  budgetTokens: number | undefined
  maxAgents: number | undefined
  diskFloorGb: number | undefined
  memoryFileBytes: number | undefined
  maxDefers: number | undefined
}
const DEFAULTS: FormState = {
  displayName: '', objective: '',
  budgetTokens: undefined, maxAgents: undefined, diskFloorGb: 40, memoryFileBytes: 8192, maxDefers: 3,
}
const form = reactive<FormState>({ ...DEFAULTS })
const modalOpen = ref(false)
const submitted = ref(false)

const valid = computed(() => form.displayName.trim() !== '' && form.objective.trim() !== '')
const errors = computed(() => ({
  displayName: submitted.value && form.displayName.trim() === '' ? 'A name is required' : '',
  objective: submitted.value && form.objective.trim() === '' ? 'An objective is required' : '',
}))

function openNew(): void {
  Object.assign(form, DEFAULTS)
  submitted.value = false
  modalOpen.value = true
}

function submit(): void {
  submitted.value = true
  if (!valid.value) return
  const limits: Limits = {}
  if (form.budgetTokens !== undefined) limits.budgetTokens = form.budgetTokens
  if (form.maxAgents !== undefined) limits.maxAgents = form.maxAgents
  if (form.diskFloorGb !== undefined) limits.diskFloorGb = form.diskFloorGb
  if (form.memoryFileBytes !== undefined) limits.memoryFileBytes = form.memoryFileBytes
  if (form.maxDefers !== undefined) limits.maxDefers = form.maxDefers
  const swarm = store.createSwarm({ displayName: form.displayName, objective: form.objective, limits })
  modalOpen.value = false
  void router.push({ name: 'swarm', params: { id: swarm.swarmId } })
}
</script>

<template>
  <section class="splash">
    <UiSpinner v-if="!store.loaded" size="md" />

    <template v-else>
      <div v-if="swarms.length" class="grid">
        <UiTile
          v-for="s in swarms"
          :key="s.swarmId"
          :title="s.displayName"
          :subtitle="s.objective"
          :accent="accentFor(s)"
          big
          clickable
          @click="open(s)"
        >
          <template #footer>
            <span class="foot">
              <UiStateBadge :state="s.state" />
              <span class="stat">{{ s.agents.length }} agents</span>
              <span class="stat">{{ openDecisions(s) }} open decisions</span>
            </span>
          </template>
        </UiTile>

        <UiTile
          title="＋ New swarm"
          subtitle="Name, objective, limits. The orchestrator bootstraps the rest."
          accent="var(--color-accent-2)"
          big
          clickable
          @click="openNew"
        />
      </div>

      <UiEmptyState
        v-else
        title="No swarms yet"
        text="Give one a name, an objective and its limits; the orchestrator bootstraps everything else and makes sure the members can communicate."
        icon="＋"
      >
        <UiButton variant="primary" size="lg" @click="openNew">New swarm</UiButton>
      </UiEmptyState>
    </template>

    <UiModal :open="modalOpen" title="New swarm" width="600px" @close="modalOpen = false">
      <form class="form" @submit.prevent="submit">
        <UiTextInput
          v-model="form.displayName"
          label="Name"
          placeholder="b10x forward"
          :error="errors.displayName"
        />
        <UiTextArea
          v-model="form.objective"
          label="Objective"
          placeholder="What this swarm is for — the goals, in its owner's order"
          :rows="3"
          :error="errors.objective"
        />
        <div class="limits">
          <UiNumberInput v-model="form.budgetTokens" label="Budget" unit="tokens" :min="0" :step="1000" />
          <UiNumberInput v-model="form.maxAgents" label="Max agents" :min="1" :step="1" />
          <UiNumberInput v-model="form.diskFloorGb" label="Disk floor" unit="GB" :min="0" :step="1" />
          <UiNumberInput v-model="form.memoryFileBytes" label="Memory file" unit="bytes" :min="0" :step="512" />
          <UiNumberInput v-model="form.maxDefers" label="Max defers" :min="0" :step="1" />
        </div>
      </form>
      <template #footer>
        <UiButton variant="ghost" @click="modalOpen = false">Cancel</UiButton>
        <UiButton variant="primary" :disabled="submitted && !valid" @click="submit">Create swarm</UiButton>
      </template>
    </UiModal>
  </section>
</template>

<style scoped>
.splash {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-6);
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 380px));
  justify-content: center;
  gap: var(--space-5);
  width: min(100%, 1240px);
}
.foot { display: inline-flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; }
.stat { color: var(--color-text-muted); }
.form { display: flex; flex-direction: column; gap: var(--space-4); }
.limits {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--space-3);
}
</style>

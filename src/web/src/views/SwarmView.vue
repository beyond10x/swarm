<script setup lang="ts">
// One swarm: what it is pursuing, what is on its canvas, and the lifecycle commands.
//
// The tabs are not a fixed list. One is the canvas; the rest are the entities the specification
// declares, so a domain added to `src/core` gets a tab without this file being edited. That is the
// same argument the canvas makes about node types, one level up.
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  UiButton, UiEmptyState, UiKeyValue, UiSpinner, UiStateBadge, UiTable, UiTabs, UiToolbar,
} from '@/components/ui'
import { useSwarmStore, field, type SwarmAction } from '@/stores/swarms'
import SwarmCanvas from '@/views/SwarmCanvas.vue'

const props = defineProps<{ id: string }>()
const store = useSwarmStore()
const router = useRouter()

onMounted(() => void store.load())
onBeforeUnmount(() => store.unfollow())

const swarm = computed(() => store.getSwarm(props.id))
const record = computed(() => swarm.value?.record)
const goal = computed(() => swarm.value?.canvas['swarm.goal.Goal']?.[0])

const displayName = computed(
  () => (field(record.value, 'display_name') as string | undefined) ?? props.id,
)

/**
 * What the swarm is waiting on, in one sentence.
 *
 * A loop that is between turns looks exactly like a loop that is broken, so this says which it is.
 * Every case here is a real resting place in the goal's lifecycle rather than a guess.
 */
const waitingOn = computed(() => {
  const state = record.value?.state
  if (!record.value) return 'This swarm has no record yet.'
  if (state !== 'Running')
    return `The swarm is ${state}. The loop only turns while it is Running — press start.`
  if (!goal.value) return 'No goal yet. A swarm with nothing to pursue has nothing to turn.'

  switch (goal.value.state) {
    case 'Open':
      return 'Waiting for the first tick. The loop turns every 30 seconds.'
    case 'Pursuing':
      return 'A turn is under way: the coordinator is deciding whether the goal is met. Without one configured, it waits here.'
    case 'Evaluating':
      return 'The last turn reported "not yet". The next tick picks it up again.'
    case 'Reached':
      return 'The goal is met and the loop has stopped.'
    case 'Abandoned':
      return 'The goal was given up on.'
    default:
      return ''
  }
})

/** Every entity that holds at least one instance, in the order the specification declares them. */
const populated = computed(() =>
  Object.entries(swarm.value?.canvas ?? {})
    .filter(([, held]) => held.length > 0)
    .map(([entity, held]) => ({
      entity,
      label: entity.split('.').pop() ?? entity,
      count: held.length,
      held,
    })),
)

const tab = ref('canvas')
const tabs = computed(() => [
  { value: 'canvas', label: 'Canvas' },
  ...populated.value.map((group) => ({
    value: group.entity,
    label: `${group.label} (${group.count})`,
  })),
])

const shown = computed(() => populated.value.find((group) => group.entity === tab.value))

/** A table of instances: identity, state, and whatever fields something has written. */
const fieldNames = computed(() => {
  if (!shown.value) return [] as string[]
  const names = new Set<string>()
  for (const instance of shown.value.held) {
    for (const [name, value] of Object.entries(instance.fields)) {
      if (value !== null) names.add(name)
    }
  }
  return [...names]
})

const columns = computed(() => [
  { key: 'id', label: 'id' },
  { key: 'state', label: 'state' },
  ...fieldNames.value.map((name) => ({ key: name, label: name })),
])

const rows = computed(() =>
  (shown.value?.held ?? []).map((instance) => {
    const row: Record<string, unknown> = {
      id: instance.id.split('-')[0],
      state: instance.state,
    }
    for (const name of fieldNames.value) row[name] = instance.fields[name] ?? '—'
    return row
  }),
)

const ACTIONS: SwarmAction[] = ['start', 'pause', 'resume', 'stop', 'delete']
const can = (action: SwarmAction): boolean => store.canAct(props.id, action)

async function act(action: SwarmAction): Promise<void> {
  const ok = await store[
    `${action}Swarm` as 'startSwarm' | 'pauseSwarm' | 'resumeSwarm' | 'stopSwarm' | 'deleteSwarm'
  ](props.id)
  if (ok && action === 'delete') void router.push({ name: 'splash' })
}
</script>

<template>
  <section class="swarm">
    <UiSpinner v-if="!store.loaded" size="md" />

    <UiEmptyState
      v-else-if="!swarm"
      title="No such swarm"
      text="The runtime does not hold one by that name."
    />

    <template v-else>
      <UiToolbar>
        <h1>{{ displayName }}</h1>
        <UiStateBadge :state="record?.state ?? 'Uncreated'" />

        <template #right>
          <UiButton
            v-for="action in ACTIONS"
            :key="action"
            :variant="action === 'delete' ? 'danger' : 'secondary'"
            :disabled="!can(action)"
            @click="act(action)"
          >
            {{ action }}
          </UiButton>
        </template>
      </UiToolbar>

      <p v-if="store.problem" class="problem" role="alert">{{ store.problem }}</p>

      <!-- What the swarm is for, and how far the loop has got. -->
      <UiKeyValue
        v-if="goal"
        class="goal"
        :items="[
          { key: 'Goal', value: String(field(goal, 'text') ?? '—') },
          { key: 'State', value: goal.state },
          { key: 'Turns', value: String(field(goal, 'iterations') ?? 'none yet') },
        ]"
      />

      <p class="waiting">{{ waitingOn }}</p>

      <UiTabs v-model="tab" :tabs="tabs" />

      <div class="panel">
        <SwarmCanvas
          v-if="tab === 'canvas'"
          :canvas="swarm.canvas"
          :shape="store.shape"
          :flow-id="props.id"
        />
        <UiTable v-else-if="shown" :columns="columns" :rows="rows" />
      </div>
    </template>
  </section>
</template>

<style scoped>
.swarm {
  display: grid;
  grid-template-rows: auto auto auto auto auto 1fr;
  gap: 0.75rem;
  height: 100%;
  padding: 1rem;
  align-content: start;
}

h1 {
  font-size: 1.125rem;
  margin: 0;
}

.goal {
  max-width: 60rem;
}

.panel {
  min-height: 24rem;
  border: 1px solid var(--border, #2a3240);
  border-radius: 0.5rem;
  overflow: hidden;
}

.waiting {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-left: 2px solid var(--color-accent, #4f8cff);
  background: color-mix(in srgb, var(--color-accent, #4f8cff) 8%, transparent);
  border-radius: 0 0.25rem 0.25rem 0;
  font-size: 0.875rem;
}

.problem {
  border: 1px solid var(--color-fault, #ff6b6b);
  border-radius: 0.5rem;
  padding: 0.625rem 0.875rem;
  margin: 0;
}
</style>

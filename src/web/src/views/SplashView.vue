<script setup lang="ts">
// Every swarm the runtime holds, and the way to make another.
//
// A swarm is named and given a goal, and that is all. There are no limits to set and no roster to
// pick, because a swarm starts bare: one coordinator, one goal, and whatever it builds for itself.
// The form asks for the two things nothing else can supply.
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useSwarmStore, field, type Held } from '@/stores/swarms'
import {
  UiButton, UiField, UiModal, UiSpinner, UiStateBadge, UiTextArea, UiTextInput, UiTile,
} from '@/components/ui'

const store = useSwarmStore()
const router = useRouter()

const swarms = computed(() => store.visible)
const modalOpen = ref(false)
const submitted = ref(false)
const busy = ref(false)

const DEFAULTS = { displayName: '', goal: '' }
const form = reactive({ ...DEFAULTS })

const valid = computed(() => form.displayName.trim().length > 0)

onMounted(() => void store.load())

function openNew(): void {
  Object.assign(form, DEFAULTS)
  submitted.value = false
  modalOpen.value = true
}

async function submit(): Promise<void> {
  submitted.value = true
  if (!valid.value || busy.value) return
  busy.value = true
  const slug = await store.createSwarm({ displayName: form.displayName, goal: form.goal })
  busy.value = false
  if (!slug) return
  modalOpen.value = false
  void router.push({ name: 'swarm', params: { id: slug } })
}

function open(swarm: Held): void {
  void router.push({ name: 'swarm', params: { id: swarm.slug } })
}

/** What the swarm is pursuing, when it has been given something. */
function goalText(swarm: Held): string {
  const goal = swarm.canvas['swarm.goal.Goal']?.[0]
  return (field(goal, 'text') as string | undefined) ?? 'no goal yet'
}

/** How many instances the swarm holds — the honest measure of how much it has built. */
function built(swarm: Held): number {
  return Object.values(swarm.canvas).reduce((total, held) => total + held.length, 0)
}

/** The runtime's row for this swarm, polled every couple of seconds. */
function live(swarm: Held) {
  return store.status?.swarms.find((row) => row.slug === swarm.slug)
}


function displayName(swarm: Held): string {
  return (field(swarm.record, 'display_name') as string | undefined) ?? swarm.slug
}
</script>

<template>
  <section class="splash">
    <UiSpinner v-if="!store.loaded" size="md" />

    <template v-else>
      <p v-if="store.problem" class="problem" role="alert">
        {{ store.problem }}
        <span class="hint">Is the runtime running? <code>cargo run -p swarm-server</code></span>
      </p>

      <div class="grid">
        <UiTile
          v-for="s in swarms"
          :key="s.slug"
          :title="displayName(s)"
          :subtitle="goalText(s)"
          big
          clickable
          @click="open(s)"
        >
          <template #footer>
            <span class="foot">
              <UiStateBadge :state="s.record?.state ?? 'Uncreated'" />
              <UiStateBadge v-if="live(s)?.goal" :state="live(s)!.goal!.state" />
              <span v-if="live(s)?.goal" class="stat">{{ live(s)!.goal!.iterations }} turns</span>
              <span class="stat">{{ built(s) }} on the canvas</span>
              <span class="stat">{{ live(s)?.events ?? 0 }} events</span>
              <span v-if="live(s)?.spent.cost_usd != null" class="stat mono">${{ live(s)!.spent.cost_usd!.toFixed(3) }}</span>
            </span>
          </template>
        </UiTile>

        <UiTile
          title="＋ New swarm"
          subtitle="A name and a goal. It builds the rest."
          big
          clickable
          @click="openNew"
        />
      </div>
    </template>

    <UiModal :open="modalOpen" title="New swarm" @close="modalOpen = false">
      <form class="form" @submit.prevent="submit">
        <UiField
          label="Name"
          :error="submitted && !valid ? 'A swarm needs a name.' : undefined"
        >
          <UiTextInput v-model="form.displayName" placeholder="What to call it" />
        </UiField>

        <UiField
          label="Goal"
          hint="What it is for, in words. The coordinator reads this and decides when it is met."
        >
          <UiTextArea v-model="form.goal" :rows="3" placeholder="Leave empty to set one later" />
        </UiField>
      </form>

      <template #footer>
        <UiButton variant="ghost" @click="modalOpen = false">Cancel</UiButton>
        <UiButton :disabled="busy" @click="submit">
          {{ busy ? 'Creating…' : 'Create' }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>

<style scoped>
.splash {
  padding: 1.5rem;
  display: grid;
  gap: 1rem;
  align-content: start;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
  gap: 1rem;
}

.foot {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  flex-wrap: wrap;
}

.stat {
  font-size: 0.8125rem;
  opacity: 0.7;
}

.mono {
  font-family: var(--font-mono);
  opacity: 1;
}

.problem {
  border: 1px solid var(--color-fault, #ff6b6b);
  border-radius: 0.5rem;
  padding: 0.75rem 1rem;
  display: grid;
  gap: 0.25rem;
}

.problem .hint {
  font-size: 0.8125rem;
  opacity: 0.75;
}

.form {
  display: grid;
  gap: 1rem;
}
</style>

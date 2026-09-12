<script setup lang="ts">
// The loop, drawn: `[loop] → [coordinator] → [goal]`, with each stage showing where it is now.
//
// This is the whole of what a bare swarm does, so it gets its own drawing rather than being read
// off the canvas. The loop stage counts down to the next tick; the coordinator stage shows a turn
// being asked, answered or failing; the goal stage shows the state and the turn count. The arrow
// that is live is lit, so a person can see the data moving rather than infer it.
import { computed } from 'vue'
import type { Change, Instance } from '@/runtime'
import { field, useSwarmStore } from '@/stores/swarms'
import { UiBadge, UiSpinner, UiStateBadge } from '@/components/ui'

const props = defineProps<{ slug: string; record?: Instance; goal?: Instance }>()
const store = useSwarmStore()

type Turn = Extract<Change, { kind: 'turn' }>

const running = computed(() => props.record?.state === 'Running')
const goalState = computed(() => props.goal?.state)
const turns = computed(() => (field(props.goal, 'iterations') as number | undefined) ?? 0)
const ended = computed(() => goalState.value === 'Reached' || goalState.value === 'Abandoned')

/** The last thing heard from the coordinator about this goal. */
const turn = computed<Turn | undefined>(() => store.lastTurn(props.slug, props.goal?.id))

/** What the current or last turn has cost so far, live while it runs. */
const spent = computed(() => {
  const running = store.liveTurn(props.slug)
  if (running?.spent) return running.spent
  return turn.value?.spent ?? undefined
})

const money = (usd: number | null | undefined): string =>
  usd == null ? '—' : `$${usd.toFixed(usd < 0.01 ? 4 : 3)}`

/** Everything the swarm has spent across every finished turn, from the status. */
const total = computed(() => store.status?.swarms.find((row) => row.slug === props.slug)?.spent)

/** How long the coordinator has been thinking, when it is. */
const thinkingFor = computed(() => {
  if (turn.value?.phase !== 'asking') return undefined
  return Math.max(0, Math.round((store.clock - Date.parse(turn.value.at)) / 1000))
})

/** Which arrow carries data right now. */
const live = computed<'tick' | 'ask' | 'verdict' | undefined>(() => {
  if (!running.value || !props.goal || ended.value || cap.value) return undefined
  if (turn.value?.phase === 'asking') return 'ask'
  if (goalState.value === 'Pursuing') return 'ask'
  if (turn.value?.phase === 'answered' && store.clock - Date.parse(turn.value.at) < 3000)
    return 'verdict'
  return 'tick'
})

const waiting = computed(() => {
  if (!props.record) return 'no swarm record yet'
  if (cap.value) return cap.value.why
  if (!running.value) return `the swarm is ${props.record.state}; the loop turns only while Running`
  if (!props.goal) return 'no goal; nothing to pursue'
  if (ended.value) return `the goal is ${goalState.value}; the loop has stopped`
  if (goalState.value === 'Pursuing') return 'a turn is under way'
  return store.nextTickIn !== undefined ? `next tick in ${store.nextTickIn}s` : 'waiting for a tick'
})

const coordinatorConfigured = computed(() => store.status?.coordinator.configured ?? false)

/** Whether the loop has stopped asking about this goal because a cap was reached. */
const cap = computed(() => store.capOn(props.slug, props.goal?.id))
</script>

<template>
  <section class="loop-panel">
    <div class="stages">
      <div class="stage" :class="{ active: live === 'tick' }">
        <header>
          <span class="name">loop</span>
          <UiBadge v-if="cap" tone="warn" text="capped" />
          <UiBadge
            v-else-if="running && !ended && goal"
            tone="ok"
            :text="store.nextTickIn !== undefined ? `tick in ${store.nextTickIn}s` : 'armed'"
          />
          <UiBadge v-else tone="muted" text="idle" />
        </header>
        <p v-if="cap" class="detail fault">{{ cap.turns }} turns · {{ money(cap.spent_usd) }} spent</p>
        <p v-else class="detail">
          every {{ store.status?.periodic[0]?.every_s ?? '?' }}s · {{ store.status?.ticks ?? 0 }} ticks since start
          <template v-if="store.status?.caps.max_turns"> · cap {{ store.status.caps.max_turns }} turns</template>
          <template v-if="store.status?.caps.max_spend_usd"> / ${{ store.status.caps.max_spend_usd.toFixed(2) }}</template>
        </p>
      </div>

      <div class="arrow" :class="{ live: live === 'tick' }"><span class="pulse" /></div>

      <div class="stage" :class="{ active: live === 'ask' }">
        <header>
          <span class="name">coordinator <span v-if="total?.cost_usd != null" class="total" title="spent across every finished turn">{{ money(total.cost_usd) }} total</span></span>
          <span v-if="turn?.phase === 'asking'" class="thinking">
            <UiSpinner size="sm" /> {{ thinkingFor }}s
          </span>
          <UiBadge v-else-if="!coordinatorConfigured" tone="warn" text="none" />
          <UiBadge v-else-if="turn?.phase === 'answered'" :tone="turn.reached ? 'ok' : 'info'" :text="turn.reached ? 'reached' : 'not yet'" />
          <UiBadge v-else-if="turn?.phase === 'unfinished'" tone="fault" text="failed" />
          <UiBadge v-else tone="muted" text="idle" />
        </header>
        <p v-if="turn?.phase === 'answered'" class="detail" :title="turn.note ?? ''">
          turn {{ turn.iterations }} · {{ money(spent?.cost_usd) }} · {{ ((turn.took_ms ?? 0) / 1000).toFixed(1) }}s<template v-if="turn.note"> · {{ turn.note }}</template>
        </p>
        <p v-else-if="turn?.phase === 'unfinished'" class="detail fault" :title="turn.error ?? ''">
          {{ turn.error }}
        </p>
        <p v-else-if="turn?.phase === 'asking'" class="detail">
          turn {{ turn.iterations }} · {{ spent?.requests ?? 0 }} requests · {{ spent?.tool_calls ?? 0 }} tools
          · {{ ((spent?.input_tokens ?? 0) + (spent?.output_tokens ?? 0)).toLocaleString() }} tokens
          · {{ (spent?.cache_read_tokens ?? 0).toLocaleString() }} cached
        </p>
        <p v-else class="detail">{{ coordinatorConfigured ? 'waiting to be asked' : 'set SWARM_COORDINATOR and restart the runtime' }}</p>
      </div>

      <div class="arrow" :class="{ live: live === 'verdict' || live === 'ask' }"><span class="pulse" /></div>

      <div class="stage" :class="{ active: live === 'verdict', ended }">
        <header>
          <span class="name">goal</span>
          <UiStateBadge v-if="goal" :state="goal.state" />
          <UiBadge v-else tone="muted" text="none" />
        </header>
        <p class="detail" :title="String(field(goal, 'text') ?? '')">
          <template v-if="goal">{{ turns }} {{ turns === 1 ? 'turn' : 'turns' }} · {{ field(goal, 'text') ?? '—' }}</template>
          <template v-else>set one when creating the swarm</template>
        </p>
      </div>
    </div>

    <p class="waiting">{{ waiting }}</p>
  </section>
</template>

<style scoped>
.loop-panel {
  display: grid;
  gap: var(--space-2);
}

.stages {
  display: grid;
  grid-template-columns: 1fr auto 1fr auto 1fr;
  align-items: stretch;
  gap: 0;
}

.stage {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  padding: var(--space-2) var(--space-3);
  display: grid;
  gap: var(--space-1);
  min-width: 0;
  transition: border-color 0.3s, box-shadow 0.3s;
}

.stage.active {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-accent) 40%, transparent), 0 0 18px color-mix(in srgb, var(--color-accent) 25%, transparent);
}

.stage.ended {
  opacity: 0.75;
}

.stage header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.name {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-muted);
}

.total {
  text-transform: none;
  letter-spacing: 0;
  font-family: var(--font-mono);
  font-weight: 400;
  margin-left: var(--space-2);
}

.thinking {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  font-size: 12px;
  color: var(--color-accent);
  font-family: var(--font-mono);
}

.detail {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail.fault {
  color: var(--color-fault);
}

.arrow {
  width: 3.5rem;
  position: relative;
  align-self: center;
  height: 2px;
  background: var(--color-border);
  overflow: visible;
}

.arrow::after {
  content: '';
  position: absolute;
  right: -1px;
  top: -4px;
  border: 5px solid transparent;
  border-left: 7px solid var(--color-border);
}

.arrow.live {
  background: var(--color-accent);
}

.arrow.live::after {
  border-left-color: var(--color-accent);
}

.arrow .pulse {
  display: none;
  position: absolute;
  top: -3px;
  left: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-accent);
  box-shadow: 0 0 8px var(--color-accent);
}

.arrow.live .pulse {
  display: block;
  animation: travel 1.2s linear infinite;
}

@keyframes travel {
  from { left: 0; opacity: 0; }
  20% { opacity: 1; }
  80% { opacity: 1; }
  to { left: calc(100% - 8px); opacity: 0; }
}

.waiting {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-muted);
  padding-left: var(--space-1);
}
</style>

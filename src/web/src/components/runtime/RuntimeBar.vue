<script setup lang="ts">
// Where the runtime is, in one line, on every page.
//
// Reachable or not; what it runs; how long it has been up; whether a coordinator is configured;
// when the loop turns next. These are the five things that explain a screen on which nothing is
// happening, and they are shown all the time so that "nothing is happening" is never a mystery.
import { computed } from 'vue'
import { useSwarmStore } from '@/stores/swarms'
import { UiBadge } from '@/components/ui'

const store = useSwarmStore()

const status = computed(() => store.status)

const uptime = computed(() => {
  if (!status.value) return '—'
  const seconds = Math.max(0, Math.round((store.clock - Date.parse(status.value.started_at)) / 1000))
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  return h ? `${h}h ${m}m` : m ? `${m}m ${s}s` : `${s}s`
})

/** The coordinator's program, shortened to what a person would call it. */
const coordinator = computed(() => {
  const program = status.value?.coordinator.program
  if (!program) return undefined
  return program.split(/\s+/)[0]!.split('/').pop()
})

const every = computed(() => status.value?.periodic[0]?.every_s)

/** Everything every swarm has spent, from what the vendor billed per turn. */
const spent = computed(() => {
  const rows = status.value?.swarms ?? []
  const priced = rows.filter((row) => row.spent.cost_usd != null)
  if (!priced.length) return undefined
  return priced.reduce((sum, row) => sum + (row.spent.cost_usd ?? 0), 0)
})
</script>

<template>
  <div class="runtime-bar" :class="{ down: !store.reachable }">
    <span class="dot" :title="store.reachable ? 'runtime reachable' : 'runtime unreachable'" />

    <template v-if="store.reachable && status">
      <span class="item">
        <span class="label">system</span>
        <span class="value mono">{{ status.system }}</span>
      </span>
      <span class="item">
        <span class="label">up</span>
        <span class="value">{{ uptime }}</span>
      </span>
      <span class="item">
        <span class="label">loop</span>
        <span class="value">
          every {{ every ?? '?' }}s
          <template v-if="store.nextTickIn !== undefined">
            · next in <b class="mono">{{ store.nextTickIn }}s</b>
          </template>
          · {{ status.ticks }} ticks
        </span>
      </span>
      <span class="item">
        <span class="label">coordinator</span>
        <UiBadge v-if="coordinator" tone="ok" :text="coordinator" />
        <UiBadge v-else tone="warn" text="none configured" />
      </span>
      <span class="item">
        <span class="label">swarms</span>
        <span class="value">{{ status.swarms.length }}</span>
      </span>
      <span class="item">
        <span class="label">spent</span>
        <span class="value mono">{{ spent === undefined ? '$0' : `$${spent.toFixed(3)}` }}</span>
        <UiBadge v-if="status.coordinator.in_flight" tone="info" :text="`${status.coordinator.in_flight} running`" />
      </span>
    </template>

    <span v-else class="item">
      <span class="value">runtime unreachable</span>
      <code>cargo run -p swarm-server</code>
    </span>
  </div>
</template>

<style scoped>
.runtime-bar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  font-size: 12px;
  color: var(--color-text-muted);
  min-width: 0;
  flex: 1;
  overflow: hidden;
  white-space: nowrap;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-ok);
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-ok) 60%, transparent);
  animation: beat 2s ease-out infinite;
  flex: none;
}

.down .dot {
  background: var(--color-fault);
  animation: none;
}

@keyframes beat {
  0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-ok) 60%, transparent); }
  70% { box-shadow: 0 0 0 6px transparent; }
  100% { box-shadow: 0 0 0 0 transparent; }
}

.item {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.label {
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-size: 10px;
  opacity: 0.7;
}

.value {
  color: var(--color-text);
}

.mono {
  font-family: var(--font-mono);
}

code {
  font-family: var(--font-mono);
  background: var(--color-surface-2);
  padding: 0 var(--space-1);
  border-radius: var(--radius-1);
}
</style>

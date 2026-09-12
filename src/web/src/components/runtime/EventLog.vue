<script setup lang="ts">
// Everything that happened to one swarm, newest first.
//
// Two sources, one list. The LOG is what the store holds and is read once when the swarm is
// opened, so a reader arriving late sees history. The STREAM is what the server announces from
// then on, and it carries things the log does not — a tick that found nothing to do, a coordinator
// being asked, a reload — because those happened without an event being appended. A row from the
// stream is marked live for a moment, so what just happened is visible without reading timestamps.
import { computed, ref } from 'vue'
import type { Change, Recorded } from '@/runtime'
import { useSwarmStore } from '@/stores/swarms'
import { UiBadge } from '@/components/ui'

const props = defineProps<{ slug: string }>()
const store = useSwarmStore()

type Tone = 'ok' | 'warn' | 'fault' | 'info' | 'muted'

interface Row {
  key: string
  at: string
  kind: string
  tone: Tone
  title: string
  detail?: string
  fields?: Record<string, unknown> | null
  live: boolean
}

const short = (name: string): string => name.split('.').pop() ?? name
const id8 = (id: string): string => id.split('-')[0] ?? id

function fromRecorded(event: Recorded): Row {
  return {
    key: `log:${event.seq}`,
    at: event.at,
    kind: 'event',
    tone: 'muted',
    title: short(event.name),
    detail: `${short(event.entity)} ${id8(event.id)} v${event.version} · by ${event.actor}`,
    fields: event.fields,
    live: false,
  }
}

function fromChange(change: Change, index: number): Row {
  const key = `live:${index}`
  switch (change.kind) {
    case 'applied': {
      const refused = change.outcome.includes('wrong') || change.events.length === 0
      return {
        key,
        at: change.at,
        kind: 'command',
        tone: refused ? 'warn' : 'ok',
        title: short(change.command),
        detail: `→ ${change.outcome}${change.actor ? ` · by ${short(change.actor)}` : ''}${
          change.events.length ? ` · ${change.events.map((event) => short(event.name)).join(', ')}` : ' · no event'
        }`,
        fields: change.events[0]?.fields ?? null,
        live: true,
      }
    }
    case 'routed':
      return {
        key,
        at: change.at,
        kind: 'routed',
        tone: 'info',
        title: `${change.binding} → ${short(change.command)}`,
        detail: `→ ${change.outcome}`,
        live: true,
      }
    case 'ticked':
      return {
        key,
        at: change.at,
        kind: 'tick',
        tone: 'info',
        title: `${change.binding} → ${short(change.command)}`,
        detail: `goal ${id8(change.goal)} · turn ${change.iterations}`,
        live: true,
      }
    case 'turn':
      return {
        key,
        at: change.at,
        kind: 'coordinator',
        tone: change.phase === 'unfinished' ? 'fault' : change.reached ? 'ok' : 'info',
        title:
          change.phase === 'asking'
            ? `asked about turn ${change.iterations}`
            : change.phase === 'answered'
              ? `answered turn ${change.iterations}: ${change.reached ? 'reached' : 'not yet'}`
              : `turn ${change.iterations} unfinished`,
        detail: change.error ?? change.note ?? (change.took_ms != null ? `${change.took_ms}ms` : undefined),
        live: true,
      }
    case 'reloaded':
      return { key, at: change.at, kind: 'reload', tone: 'warn', title: 'world rebuilt from the log', live: true }
    case 'capped':
      return {
        key,
        at: change.at,
        kind: 'capped',
        tone: 'warn',
        title: `the loop stopped asking about goal ${id8(change.goal)}`,
        detail: change.why,
        live: true,
      }
    case 'agent':
      // Kept with its turn, not in this list; the store never puts one here.
      return { key, at: change.at, kind: 'agent', tone: 'muted', title: change.event.event, live: true }
  }
}

const watched = computed(() => store.live[props.slug])

const rows = computed<Row[]>(() => {
  const history = (watched.value?.history ?? []).map(fromRecorded)
  const changes = (watched.value?.changes ?? []).map(fromChange)
  // History covers everything up to the moment the stream opened; from then on the stream is
  // the fuller account, so a command's appended events are shown once, on the command's row.
  const opened = changes[0]?.at
  const before = opened ? history.filter((row) => row.at < opened) : history
  return [...before, ...changes].reverse()
})

const expanded = ref<string | undefined>()
const toggle = (key: string): void => {
  expanded.value = expanded.value === key ? undefined : key
}

const clock = computed(() => store.clock)

function when(at: string): string {
  const ms = clock.value - Date.parse(at)
  if (ms < 1000) return 'now'
  if (ms < 60_000) return `${Math.round(ms / 1000)}s ago`
  if (ms < 3_600_000) return `${Math.round(ms / 60_000)}m ago`
  return new Date(at).toLocaleTimeString()
}

const isNew = (row: Row): boolean => row.live && clock.value - Date.parse(row.at) < 2500
</script>

<template>
  <section class="event-log">
    <header class="head">
      <span class="title">events</span>
      <span class="count">{{ rows.length }}</span>
      <span class="spacer" />
      <UiBadge v-if="watched?.connected" tone="ok" text="live" />
      <UiBadge v-else tone="fault" text="stream down" />
    </header>

    <ol class="rows">
      <li v-if="!rows.length" class="empty">nothing has happened yet</li>
      <li
        v-for="row in rows"
        :key="row.key"
        class="row"
        :class="{ fresh: isNew(row), open: expanded === row.key }"
        @click="toggle(row.key)"
      >
        <span class="when" :title="row.at">{{ when(row.at) }}</span>
        <UiBadge :tone="row.tone" :text="row.kind" />
        <span class="text">
          <span class="line">{{ row.title }}</span>
          <span v-if="row.detail" class="detail">{{ row.detail }}</span>
        </span>
        <pre v-if="expanded === row.key && row.fields" class="fields">{{ JSON.stringify(row.fields, null, 2) }}</pre>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.event-log {
  display: grid;
  grid-template-rows: auto 1fr;
  min-height: 0;
  height: 100%;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  overflow: hidden;
}

.head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--color-border);
}

.title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-muted);
}

.count {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-muted);
}

.spacer {
  flex: 1;
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  min-height: 0;
}

.empty {
  padding: var(--space-4);
  color: var(--color-text-muted);
  font-size: 12px;
  text-align: center;
}

.row {
  display: grid;
  grid-template-columns: 4rem auto 1fr;
  gap: var(--space-2);
  align-items: start;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.6s ease-out;
}

.row:hover {
  background: var(--color-surface-2);
}

.row.fresh {
  background: color-mix(in srgb, var(--color-accent) 16%, transparent);
  animation: flash 2.5s ease-out forwards;
}

@keyframes flash {
  from { background: color-mix(in srgb, var(--color-accent) 28%, transparent); }
  to { background: transparent; }
}

.when {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-muted);
  padding-top: 2px;
}

.text {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.line {
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail {
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.open .line,
.open .detail {
  white-space: normal;
}

.fields {
  grid-column: 1 / -1;
  margin: 0;
  padding: var(--space-2);
  background: var(--color-bg);
  border-radius: var(--radius-1);
  font-family: var(--font-mono);
  font-size: 11px;
  overflow-x: auto;
  cursor: text;
}
</style>

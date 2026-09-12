<script setup lang="ts">
// The coordinator's runs: what it said, what it called, what each request cost.
//
// One turn at a time. The live one first when there is one, then whatever earlier turns are held
// or recorded on disk. Every row is one `metaharness.event/1` record, drawn by kind: text as the
// model's words, thinking folded away, a tool call with its result attached once it arrives, and
// each request's usage as a small line under the words it paid for. The header is the running
// total, read from the same events and never multiplied out here.
import { computed, onMounted, ref, watch } from 'vue'
import type { AgentEvent, Spent, TurnRecord } from '@/runtime'
import * as runtime from '@/runtime'
import { useSwarmStore, type Turn } from '@/stores/swarms'
import { UiBadge, UiSpinner } from '@/components/ui'

const props = defineProps<{ slug: string }>()
const store = useSwarmStore()

const recorded = ref<TurnRecord[]>([])
const selected = ref<string | undefined>()
const loading = ref(false)

const held = computed<Turn[]>(() => store.live[props.slug]?.turns ?? [])

/** Every turn a person can pick: held ones, plus recorded ones not yet loaded. */
const choices = computed(() => {
  const seen = new Set(held.value.map((turn) => turn.iterations))
  const fromDisk = recorded.value
    .filter((record) => !seen.has(record.iterations))
    .map((record) => ({ key: `disk:${record.name}`, iterations: record.iterations, record, turn: undefined as Turn | undefined }))
  const live = held.value.map((turn) => ({ key: turn.key, iterations: turn.iterations, record: undefined as TurnRecord | undefined, turn }))
  return [...live, ...fromDisk].sort((a, b) => b.iterations - a.iterations)
})

const shown = computed<Turn | undefined>(() => {
  const pick = choices.value.find((choice) => choice.key === selected.value) ?? choices.value[0]
  return pick?.turn
})

async function refreshRecorded(): Promise<void> {
  try {
    recorded.value = await runtime.turns(props.slug)
  } catch {
    recorded.value = []
  }
}

async function choose(key: string): Promise<void> {
  selected.value = key
  const pick = choices.value.find((choice) => choice.key === key)
  if (pick?.record && !pick.turn) {
    loading.value = true
    await store.loadTurn(props.slug, pick.record)
    loading.value = false
    const loaded = held.value.find((turn) => turn.iterations === pick.iterations)
    if (loaded) selected.value = loaded.key
  }
}

onMounted(async () => {
  await refreshRecorded()
  // Nothing live and nothing held: open the newest recorded turn rather than an empty panel.
  if (!shown.value && choices.value[0]) await choose(choices.value[0].key)
})
// A turn that just ended is a file that just appeared.
watch(
  () => held.value.map((turn) => turn.phase).join(),
  () => void refreshRecorded(),
)
// Follow the live turn when one starts.
watch(
  () => store.liveTurn(props.slug)?.key,
  (key) => {
    if (key) selected.value = key
  },
)

const spent = computed<Spent | undefined>(() => shown.value?.spent)

const money = (usd: number | null | undefined): string =>
  usd == null ? '—' : `$${usd.toFixed(usd < 0.01 ? 4 : 3)}`
const count = (n: number | undefined): string => (n ?? 0).toLocaleString()

/** Tool results, by the call they answer, so a call is drawn with its answer. */
const results = computed(() => {
  const byCall = new Map<string, AgentEvent>()
  for (const event of shown.value?.events ?? []) {
    if (event.event === 'tool.result' && typeof event.call_id === 'string') byCall.set(event.call_id, event)
  }
  return byCall
})

/**
 * Rows worth drawing. Decisions, step markers and the stream's own bookkeeping are not; nor is a
 * thinking record with no text, nor the second copy of a request's usage.
 */
const rows = computed(() => {
  const usageSeen = new Set<string>()
  return (shown.value?.events ?? []).filter((event) => {
    if (!['session.started', 'text', 'thinking', 'tool.requested', 'usage', 'session.ended', 'warning', 'auth.expired'].includes(event.event)) return false
    if (event.event === 'thinking') return typeof event.text === 'string' && event.text.trim().length > 0
    if (event.event === 'usage' && typeof event.request_id === 'string') {
      if (usageSeen.has(event.request_id)) return false
      usageSeen.add(event.request_id)
    }
    return true
  })
})

const expanded = ref(new Set<number>())
function toggle(seq: number | undefined): void {
  if (seq === undefined) return
  const next = new Set(expanded.value)
  if (next.has(seq)) next.delete(seq)
  else next.add(seq)
  expanded.value = next
}

function short(value: unknown, max = 160): string {
  const text = typeof value === 'string' ? value : JSON.stringify(value)
  return text.length > max ? `${text.slice(0, max)}…` : text
}

/** The one line of a tool call worth reading without expanding it. */
function callSummary(event: AgentEvent): string {
  const input = (event.input ?? {}) as Record<string, unknown>
  const first = input.command ?? input.file_path ?? input.pattern ?? input.query ?? input.prompt ?? input.description
  return first === undefined ? short(input, 120) : short(first, 120)
}

function usageOf(event: AgentEvent): Record<string, number | null> {
  return (event.usage ?? {}) as Record<string, number | null>
}

function when(at: unknown): string {
  return typeof at === 'string' && at ? new Date(at).toLocaleTimeString() : ''
}

const phaseTone = (turn: Turn | undefined) =>
  !turn ? 'muted'
  : turn.phase === 'asking' ? 'info'
  : turn.phase === 'unfinished' ? 'fault'
  : turn.reached ? 'ok'
  : turn.reached === false ? 'muted'
  : 'muted'
</script>

<template>
  <section class="transcript">
    <header class="head">
      <div class="turns">
        <button
          v-for="choice in choices"
          :key="choice.key"
          type="button"
          class="turn-pick"
          :class="{ active: (shown && choice.turn === shown) || selected === choice.key, live: choice.turn?.phase === 'asking' }"
          @click="choose(choice.key)"
        >
          turn {{ choice.iterations }}
          <span v-if="choice.turn?.phase === 'asking'" class="pulse" />
        </button>
        <span v-if="!choices.length" class="none">no coordinator turns yet</span>
      </div>

      <div v-if="shown" class="totals">
        <UiBadge :tone="phaseTone(shown)" :text="shown.phase === 'asking' ? 'running' : shown.phase === 'unfinished' ? 'unfinished' : shown.reached ? 'reached' : shown.reached === false ? 'not yet' : 'no verdict recorded'" />
        <span class="stat"><b>{{ money(spent?.cost_usd) }}</b> cost</span>
        <span class="stat"><b>{{ count(spent?.input_tokens) }}</b> in</span>
        <span class="stat"><b>{{ count(spent?.output_tokens) }}</b> out</span>
        <span class="stat"><b>{{ count(spent?.cache_read_tokens) }}</b> cache read</span>
        <span class="stat"><b>{{ count(spent?.cache_write_tokens) }}</b> cache write</span>
        <span v-if="spent?.thinking_tokens" class="stat"><b>{{ count(spent.thinking_tokens) }}</b> thinking</span>
        <span class="stat"><b>{{ spent?.requests ?? 0 }}</b> requests</span>
        <span class="stat"><b>{{ spent?.tool_calls ?? 0 }}</b> tools</span>
        <span v-if="spent?.duration_ms != null" class="stat"><b>{{ (spent.duration_ms / 1000).toFixed(1) }}s</b></span>
        <span v-if="spent?.model" class="stat mono">{{ spent.model }}</span>
      </div>
    </header>

    <p v-if="shown?.note || shown?.error" class="verdict" :class="{ fault: shown?.error }">
      {{ shown?.error ?? shown?.note }}
    </p>

    <UiSpinner v-if="loading" size="sm" />

    <ol v-else-if="shown" class="rows">
      <li v-if="!rows.length && shown.phase === 'asking'" class="row muted">
        <UiSpinner size="sm" /> starting the run…
      </li>
      <li v-if="!rows.length && shown.phase !== 'asking'" class="row muted">
        this turn streamed nothing — a coordinator that only answers has nothing to show
      </li>

      <li
        v-for="event in rows"
        :key="event.seq ?? JSON.stringify(event)"
        class="row"
        :class="event.event.replace('.', '-')"
      >
        <template v-if="event.event === 'session.started'">
          <span class="tag">session</span>
          <span class="body muted">
            {{ event.model }} · {{ (event.offered_tools as string[] | undefined)?.length ?? 0 }} tools offered
            · {{ event.harness_version }} · {{ when(event.at) }}
          </span>
        </template>

        <template v-else-if="event.event === 'text'">
          <span class="tag">said</span>
          <div class="body text">{{ event.text }}</div>
        </template>

        <template v-else-if="event.event === 'thinking'">
          <span class="tag muted">thought</span>
          <div class="body muted clickable" @click="toggle(event.seq)">
            {{ expanded.has(event.seq ?? -1) ? event.text : short(event.text, 140) }}
          </div>
        </template>

        <template v-else-if="event.event === 'tool.requested'">
          <span class="tag tool">{{ event.name }}</span>
          <div class="body">
            <div class="clickable" @click="toggle(event.seq)">
              <code>{{ callSummary(event) }}</code>
              <span v-if="results.get(String(event.call_id))" class="result" :class="{ fault: results.get(String(event.call_id))?.is_error }">
                → {{ results.get(String(event.call_id))?.is_error ? 'error' : 'ok' }}
                · {{ count(results.get(String(event.call_id))?.bytes as number | undefined) }} bytes
              </span>
              <span v-else-if="shown.phase === 'asking'" class="result"><UiSpinner size="sm" /></span>
            </div>
            <template v-if="expanded.has(event.seq ?? -1)">
              <pre class="detail">{{ JSON.stringify(event.input, null, 2) }}</pre>
              <pre v-if="results.get(String(event.call_id))" class="detail result-body">{{ short(results.get(String(event.call_id))?.content, 4000) }}</pre>
            </template>
          </div>
        </template>

        <template v-else-if="event.event === 'usage'">
          <span class="tag muted">usage</span>
          <span class="body usage">
            in {{ count(usageOf(event).input_tokens ?? 0) }}
            · out {{ count(usageOf(event).output_tokens ?? 0) }}
            · cache read {{ count(usageOf(event).cache_read_input_tokens ?? 0) }}
            · cache write {{ count(usageOf(event).cache_creation_input_tokens ?? 0) }}
            <template v-if="usageOf(event).thinking_tokens"> · thinking {{ count(usageOf(event).thinking_tokens ?? 0) }}</template>
            <template v-if="event.model"> · {{ event.model }}</template>
          </span>
        </template>

        <template v-else-if="event.event === 'session.ended'">
          <span class="tag">ended</span>
          <span class="body">
            <b>{{ money(event.total_cost_usd as number | null) }}</b>
            · {{ event.num_turns }} turns · {{ event.stop_reason }} · {{ event.terminal_reason }}
            · {{ ((event.duration_ms as number | null) ?? 0) / 1000 }}s
            <span v-if="event.is_error" class="fault"> · error</span>
          </span>
        </template>

        <template v-else>
          <span class="tag fault">{{ event.event }}</span>
          <span class="body">{{ short(event, 300) }}</span>
        </template>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.transcript {
  display: grid;
  grid-template-rows: auto auto 1fr;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.head {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--color-border);
}

.turns {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
  align-items: center;
}

.turn-pick {
  background: var(--color-surface-2);
  border: 1px solid var(--color-border);
  color: var(--color-text-muted);
  border-radius: 999px;
  padding: 0 var(--space-2);
  line-height: 22px;
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

.turn-pick.active {
  color: var(--color-text);
  border-color: var(--color-accent);
}

.turn-pick .pulse {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-accent);
  animation: blink 1s ease-in-out infinite;
}

@keyframes blink {
  50% { opacity: 0.2; }
}

.none {
  font-size: 12px;
  color: var(--color-text-muted);
}

.totals {
  display: flex;
  gap: var(--space-3);
  flex-wrap: wrap;
  align-items: center;
  font-size: 12px;
  color: var(--color-text-muted);
}

.stat b {
  color: var(--color-text);
  font-family: var(--font-mono);
  font-weight: 600;
}

.mono {
  font-family: var(--font-mono);
}

.verdict {
  margin: 0;
  padding: var(--space-2) var(--space-3);
  font-size: 12px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text);
}

.verdict.fault {
  color: var(--color-fault);
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  min-height: 0;
}

.row {
  display: grid;
  grid-template-columns: 6rem 1fr;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  font-size: 13px;
  align-items: start;
}

.row.usage,
.row.thinking,
.row.session-started {
  padding-top: 2px;
  padding-bottom: 2px;
  border-bottom: none;
  font-size: 11px;
}

.tag {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-muted);
  padding-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tag.tool {
  color: var(--color-accent);
  text-transform: none;
  font-family: var(--font-mono);
}

.tag.fault,
.fault {
  color: var(--color-fault);
}

.body {
  min-width: 0;
}

.body.text {
  white-space: pre-wrap;
  color: var(--color-text);
  line-height: 1.45;
}

.muted {
  color: var(--color-text-muted);
}

.usage {
  font-family: var(--font-mono);
  color: var(--color-text-muted);
}

.clickable {
  cursor: pointer;
}

code {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-text);
}

.result {
  margin-left: var(--space-2);
  font-size: 12px;
  color: var(--color-ok);
}

.result.fault {
  color: var(--color-fault);
}

.detail {
  margin: var(--space-1) 0 0;
  padding: var(--space-2);
  background: var(--color-bg);
  border-radius: var(--radius-1);
  font-family: var(--font-mono);
  font-size: 11px;
  overflow-x: auto;
  white-space: pre-wrap;
  max-height: 24rem;
  overflow-y: auto;
}

.result-body {
  color: var(--color-text-muted);
}
</style>

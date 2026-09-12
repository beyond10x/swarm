<script setup lang="ts">
// The swarm's mail: who said what to whom, and a way for a person to join in.
//
// Messages and mailboxes are entities, so the canvas already draws them and the instance table
// already lists them. This is the readable version: threads by subject, unread first, with the
// body, and a compose box — because the point of an inbox a human can reach is that a human can
// reach it. A message sent from here is posted as `you`, which is a sender no agent claims.
//
// Nothing here marks a message read on the recipient's behalf. Reading it in this panel is a
// person reading it, and the recipient's own read mark is the recipient's to make — the same rule
// the prompt follows and the same reason the Operator actor may post and may not ack.
import { computed, ref } from 'vue'
import * as runtime from '@/runtime'
import type { Instance } from '@/runtime'
import { field, useSwarmStore } from '@/stores/swarms'
import { UiBadge, UiButton, UiEmptyState, UiField, UiSelect, UiTextArea, UiTextInput } from '@/components/ui'

const props = defineProps<{ slug: string }>()
const store = useSwarmStore()

const MESSAGE = 'swarm.mailbox.Message'
const MAILBOX = 'swarm.mailbox.Mailbox'
/** The sender a person writes as. No agent holds this role slug, so it is never ambiguous. */
const HUMAN = 'you'

const held = computed(() => store.getSwarm(props.slug))
const messages = computed<Instance[]>(() => held.value?.canvas[MESSAGE] ?? [])
const mailboxes = computed<Instance[]>(() => held.value?.canvas[MAILBOX] ?? [])

/** Everyone who can be written to, as the address a sender types. */
const addresses = computed(() =>
  mailboxes.value
    .filter((mailbox) => mailbox.state === 'Open')
    .map((mailbox) => {
      const agent = String(field(mailbox, 'agent_id') ?? '')
      const name = String(field(mailbox, 'name') ?? 'main')
      return { value: name === 'main' ? agent : `${agent}/${name}`, label: `${agent}/${name}` }
    }),
)

const unread = computed(() => messages.value.filter((message) => message.state === 'Unread').length)

/** Newest first, and unread before read, because unread is what a reader is looking for. */
const sorted = computed(() =>
  [...messages.value].sort((left, right) => {
    if ((left.state === 'Unread') !== (right.state === 'Unread')) {
      return left.state === 'Unread' ? -1 : 1
    }
    return String(field(right, 'sent_at') ?? '').localeCompare(String(field(left, 'sent_at') ?? ''))
  }),
)

const opened = ref(new Set<string>())
function toggle(id: string): void {
  const next = new Set(opened.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  opened.value = next
}

const to = ref('')
const subject = ref('')
const body = ref('')
const busy = ref(false)
const problem = ref<string | undefined>()
const sent = ref<string | undefined>()

const canSend = computed(() => subject.value.trim().length > 0 && body.value.trim().length > 0)

async function send(broadcast: boolean): Promise<void> {
  if (!canSend.value || busy.value) return
  busy.value = true
  problem.value = undefined
  sent.value = undefined
  try {
    const posted = await runtime.sendMail(props.slug, {
      to: broadcast ? undefined : to.value || addresses.value[0]?.value,
      sender: HUMAN,
      subject: subject.value.trim(),
      body: body.value.trim(),
    })
    sent.value = posted.broadcast_id
      ? `sent to ${posted.messages.length} mailboxes`
      : 'sent'
    subject.value = ''
    body.value = ''
    await store.refresh(props.slug)
  } catch (why) {
    problem.value = why instanceof Error ? why.message : 'the message could not be sent'
  } finally {
    busy.value = false
  }
}

const tone = (state: string): 'ok' | 'info' | 'muted' =>
  state === 'Unread' ? 'info' : state === 'Acked' ? 'ok' : 'muted'

function when(value: unknown): string {
  return typeof value === 'string' && value ? new Date(value).toLocaleTimeString() : ''
}
</script>

<template>
  <section class="mail">
    <header class="head">
      <span class="title">mail</span>
      <UiBadge v-if="unread" tone="info" :text="`${unread} unread`" />
      <span class="count">{{ messages.length }} messages · {{ mailboxes.length }} mailboxes</span>
    </header>

    <div class="body">
      <ol class="rows">
        <UiEmptyState
          v-if="!messages.length"
          title="No mail yet"
          text="A swarm's agents write to each other here, and so can you."
        />
        <li
          v-for="message in sorted"
          :key="message.id"
          class="row"
          :class="{ unread: message.state === 'Unread', open: opened.has(message.id) }"
          @click="toggle(message.id)"
        >
          <span class="who">
            <b>{{ field(message, 'sender_id') }}</b>
            <span class="arrow">→</span>
            {{ field(message, 'recipient_id') }}
          </span>
          <span class="subject">{{ field(message, 'subject') }}</span>
          <UiBadge :tone="tone(message.state)" :text="message.state" />
          <span class="at">{{ when(field(message, 'sent_at')) }}</span>
          <UiBadge
            v-if="field(message, 'broadcast_id')"
            tone="muted"
            text="broadcast"
            class="flag"
          />
          <p v-if="opened.has(message.id)" class="text">{{ field(message, 'body') }}</p>
        </li>
      </ol>

      <form class="compose" @submit.prevent="send(false)">
        <UiField label="To">
          <UiSelect
            v-model="to"
            :options="addresses"
          />
        </UiField>
        <UiField label="Subject">
          <UiTextInput v-model="subject" placeholder="What it is about" />
        </UiField>
        <UiField label="Message" hint="Agents read their mail at the start of every turn.">
          <UiTextArea v-model="body" :rows="4" placeholder="What you want to say" />
        </UiField>
        <div class="actions">
          <UiButton :disabled="busy || !canSend" @click="send(false)">Send</UiButton>
          <UiButton variant="secondary" :disabled="busy || !canSend" @click="send(true)">
            Broadcast
          </UiButton>
        </div>
        <p v-if="problem" class="problem">{{ problem }}</p>
        <p v-else-if="sent" class="sent">{{ sent }}</p>
      </form>
    </div>
  </section>
</template>

<style scoped>
.mail {
  display: grid;
  grid-template-rows: auto 1fr;
  height: 100%;
  min-height: 0;
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
  font-size: 11px;
  color: var(--color-text-muted);
  margin-left: auto;
  font-family: var(--font-mono);
}

.body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 20rem;
  min-height: 0;
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  min-height: 0;
  border-right: 1px solid var(--color-border);
}

.row {
  display: grid;
  grid-template-columns: 12rem minmax(0, 1fr) auto auto auto;
  gap: var(--space-2);
  align-items: center;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  font-size: 12px;
  cursor: pointer;
}

.row:hover {
  background: var(--color-surface-2);
}

.row.unread {
  border-left: 2px solid var(--color-accent);
}

.who {
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.who b {
  color: var(--color-text);
}

.arrow {
  opacity: 0.5;
  margin: 0 2px;
}

.subject {
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.at {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-muted);
}

.text {
  grid-column: 1 / -1;
  margin: var(--space-2) 0 0;
  padding: var(--space-2);
  background: var(--color-bg);
  border-radius: var(--radius-1);
  white-space: pre-wrap;
  line-height: 1.5;
  cursor: text;
}

.compose {
  padding: var(--space-3);
  display: grid;
  gap: var(--space-3);
  align-content: start;
  overflow-y: auto;
  min-height: 0;
}

.actions {
  display: flex;
  gap: var(--space-2);
}

.problem {
  margin: 0;
  color: var(--color-fault);
  font-size: 12px;
}

.sent {
  margin: 0;
  color: var(--color-ok);
  font-size: 12px;
}

@media (max-width: 70rem) {
  .body {
    grid-template-columns: 1fr;
  }
}
</style>

<script setup lang="ts">
// One instance of one entity, drawn from what the specification declares about it.
//
// There is nothing entity-specific here on purpose. A node shows its entity, its lifecycle state
// and whichever of its fields hold a value — so an entity added to `src/core` is drawable the moment
// the server serves it, without this file being touched. A node type per entity would be a second
// list to keep in step with the model, and it would fall behind.
//
// Undetermined fields are shown as such rather than hidden. ESS distinguishes "nothing has written
// this" from "this is empty", and a canvas that silently dropped the first would be hiding the one
// thing worth noticing about a half-filled record.
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { Instance } from '@/runtime'
import { UiBadge, UiStateBadge } from '@/components/ui'

export interface InstanceNodeData {
  instance: Instance
  /** Terminal states, read from the specification, so an ended instance can be drawn as ended. */
  terminal?: string[]
}

defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: InstanceNodeData; selected?: boolean }>()

const instance = computed(() => props.data.instance)

/** `swarm.goal.Goal` reads as `goal`, which is what a person calls it. */
const kind = computed(() => instance.value.entity.split('.').pop() ?? instance.value.entity)

const ended = computed(() => props.data.terminal?.includes(instance.value.state) ?? false)

/** The fields something has actually written, in the order the entity declares them. */
const written = computed(() =>
  Object.entries(instance.value.fields)
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([name, value]) => ({ name, value: render(value) })),
)

/** How many fields nothing has written yet. Worth showing; not worth listing. */
const undetermined = computed(
  () => Object.values(instance.value.fields).filter((value) => value === null).length,
)

function render(value: unknown): string {
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}

/** An identity is long and the first segment is enough to tell two apart on a canvas. */
const short = computed(() => instance.value.id.split('-')[0])
</script>

<template>
  <div class="instance-node" :class="{ selected: props.selected, ended }">
    <Handle id="in" type="target" :position="Position.Left" />

    <header class="head">
      <UiBadge tone="info" :text="kind" />
      <UiStateBadge :state="instance.state" />
    </header>

    <div class="identity" :title="instance.id">
      <span class="field">{{ instance.identity_field }}</span>
      <span class="value">{{ short }}</span>
    </div>

    <dl v-if="written.length" class="fields">
      <template v-for="entry in written" :key="entry.name">
        <dt>{{ entry.name }}</dt>
        <dd :title="entry.value">{{ entry.value }}</dd>
      </template>
    </dl>

    <footer v-if="undetermined" class="undetermined" :title="'nothing has written these yet'">
      {{ undetermined }} undetermined
    </footer>

    <Handle id="out" type="source" :position="Position.Right" />
  </div>
</template>

<style scoped>
.instance-node {
  min-width: 13rem;
  max-width: 20rem;
  background: var(--surface-1, #14181f);
  border: 1px solid var(--border, #2a3240);
  border-radius: 0.5rem;
  padding: 0.5rem 0.625rem;
  font-size: 0.8125rem;
  display: grid;
  gap: 0.375rem;
}

.instance-node.selected {
  border-color: var(--accent, #4f8cff);
  box-shadow: 0 0 0 1px var(--accent, #4f8cff);
}

/* A terminal instance is done: it is still on the canvas, and it is no longer live. */
.instance-node.ended {
  opacity: 0.72;
}

.head {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  justify-content: space-between;
}

.identity {
  display: flex;
  gap: 0.375rem;
  align-items: baseline;
  font-family: var(--mono, ui-monospace, monospace);
  font-size: 0.75rem;
  opacity: 0.75;
}

.identity .field {
  opacity: 0.65;
}

.fields {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.125rem 0.5rem;
  margin: 0;
}

.fields dt {
  opacity: 0.6;
  font-size: 0.75rem;
}

.fields dd {
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.undetermined {
  font-size: 0.6875rem;
  opacity: 0.5;
  font-style: italic;
}
</style>

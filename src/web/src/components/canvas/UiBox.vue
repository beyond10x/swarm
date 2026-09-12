<script setup lang="ts">
// A `swarm.blackbox.Box{kind: Ui}`, drawn as the component it names.
//
// Every other entity is drawn by `InstanceNode`, which shows fields and states — right for a Goal,
// and wrong for a panel whose whole point is what it looks like. This is the one node type that
// renders something from the library instead of describing it.
//
// WHAT THE BOX CARRIES, and why it is where it is: `ref_id` is the component name (the
// specification already declares ref_id as "a component name for a Ui") and `props` is a
// `Map<String, String>` added to `swarm.blackbox.Box` for this story. The full argument is in
// `@/lib/uibox`, which owns the decision as a pure function; this file only mounts the result.
//
// A box naming a component the library does not have draws as a refusal rather than as nothing: an
// agent wrote that name, and an empty rectangle would look like a panel that had not loaded.
import { computed, watchEffect } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import { uiComponentNames, uiComponents } from '@/components/ui'
import { columnsFrom, uiBoxSpec } from '@/lib/uibox'
import type { Instance } from '@/runtime'
import { useSwarmStore } from '@/stores/swarms'

export interface UiBoxData {
  instance: Instance
  /** The swarm this box sits on, so a view it is fed from can be read. */
  slug?: string
  /** Whether it changed a moment ago. Lit while true, as every other node is. */
  changed?: boolean
}

defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: UiBoxData; selected?: boolean }>()

const store = useSwarmStore()

const instance = computed(() => props.data.instance)
const spec = computed(() => uiBoxSpec(instance.value, uiComponentNames))
const component = computed(() =>
  spec.value ? (uiComponents as Record<string, unknown>)[spec.value.component] : undefined,
)

/** The name on the box, whether or not the library has it — a refusal has to say what it refused. */
const named = computed(() => {
  const ref_id = instance.value.fields.ref_id
  return typeof ref_id === 'string' ? ref_id : undefined
})

const title = computed(() => {
  const name = instance.value.fields.name
  return typeof name === 'string' ? name : 'panel'
})

// Asking is the side effect; the rows are read back below. Registering the interest here rather
// than in the store means a box that names no view costs no request.
watchEffect(() => {
  const view = spec.value?.view
  const slug = props.data.slug
  if (view && slug) store.followView(slug, view)
})

/** The rows the view holds right now. The store re-reads it whenever the swarm changes. */
const rows = computed(() => {
  const view = spec.value?.view
  const slug = props.data.slug
  return view && slug ? store.viewRows(slug, view) : undefined
})

/**
 * What the component is actually passed.
 *
 * `rows` and `columns` are added only for a box fed from a view, and only where the box did not
 * say: a prop the box wrote always wins, because it is the thing a person or an agent decided.
 */
const bound = computed<Record<string, unknown>>(() => {
  const carried = spec.value?.props ?? {}
  if (!rows.value) return carried
  return {
    rows: rows.value,
    columns: columnsFrom(rows.value),
    ...carried,
  }
})
</script>

<template>
  <div class="ui-box" :class="{ selected: props.selected, changed: props.data.changed }">
    <Handle id="in" type="target" :position="Position.Left" />

    <header class="head">
      <span class="title">{{ title }}</span>
      <span class="named">{{ named ?? '—' }}</span>
    </header>

    <div v-if="component" class="body nodrag nowheel">
      <component :is="component" v-bind="bound" />
    </div>
    <p v-else class="refused">
      No component named <code>{{ named ?? 'nothing' }}</code> in the library.
    </p>

    <Handle id="out" type="source" :position="Position.Right" />
  </div>
</template>

<style scoped>
.ui-box {
  min-width: 22rem;
  max-width: 34rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-2);
  overflow: hidden;
}

.ui-box.selected {
  border-color: var(--color-accent);
}

.ui-box.changed {
  border-color: var(--color-ok);
}

.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--color-border);
}

.title {
  font-weight: 600;
}

.named {
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  color: var(--color-text-muted);
}

/* A panel is scrolled inside its own node; the canvas keeps the wheel for zooming elsewhere. */
.body {
  padding: var(--space-2);
  max-height: 22rem;
  overflow: auto;
}

.refused {
  margin: 0;
  padding: var(--space-3);
  color: var(--color-warn);
  font-size: 0.8125rem;
}
</style>

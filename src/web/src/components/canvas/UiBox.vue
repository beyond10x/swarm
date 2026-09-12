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
// agent wrote that name, and an empty rectangle would look like a panel that had not loaded. That
// refusal is reachable — `uiBoxSpec` tells "not a Ui box" from "a Ui box naming nothing the library
// has", and `SwarmCanvas` mounts this component for both a panel and a refusal.
//
// A component the contract says EMITS is rendered inert, and says so. Nothing writes `Box.props`
// back — no command does, and `blackbox.yaml` records that as UNMAPPED — so a text input a person
// could type into would discard the typing on the next refresh. That looks like it worked, which is
// worse than plainly not offering it.
//
// `inert` is an attribute on THIS subtree, so it can only make that promise for a component that
// stays in it. One does not: `UiModal` teleports to `document.body`, locks the page's scroll and
// takes Escape and Tab from the whole document, and a box naming it covered the application with a
// backdrop nothing was bound to close. Such a component is refused rather than drawn — see
// `uiRefused` in the contract, which is decided by reading the components, not by this file.
import { computed, watchEffect } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import { uiComponents, uiEmitters } from '@/components/ui'
import { columnsFrom, viewProblem, type UiBoxSpec } from '@/lib/uibox'
import type { Instance } from '@/runtime'
import { useSwarmStore } from '@/stores/swarms'

export interface UiBoxData {
  instance: Instance
  /**
   * What the canvas decided this box is: a panel, or a name the library does not have.
   *
   * Carried rather than decided again here, so there is no state this component can be mounted in
   * that it has no drawing for — `SwarmCanvas` mounts it for exactly these two.
   */
  decision: UiBoxSpec
  /** The swarm this box sits on, so a view it is fed from can be read. */
  slug?: string
  /** Whether it changed a moment ago. Lit while true, as every other node is. */
  changed?: boolean
}

defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: UiBoxData; selected?: boolean }>()

const store = useSwarmStore()

const instance = computed(() => props.data.instance)
const decision = computed(() => props.data.decision)

/** The panel, when the box names a component the library has. */
const spec = computed(() => (decision.value.kind === 'panel' ? decision.value : undefined))

const component = computed(() =>
  spec.value ? (uiComponents as Record<string, unknown>)[spec.value.component] : undefined,
)

/** The name on the box — a refusal has to say what it refused, and a panel says what it is. */
const named = computed(() =>
  decision.value.kind === 'panel' ? decision.value.component : decision.value.named,
)

/** Why this box draws no component, when it draws none. */
const refusal = computed(() => {
  if (decision.value.kind === 'unknown-component') {
    return `No component named ${decision.value.named} in the library.`
  }
  if (decision.value.kind === 'unsafe-component') {
    return `${decision.value.named} is not drawn on a canvas: it renders outside this box and takes the whole page.`
  }
  return undefined
})

/**
 * Whether the component this box names emits, per the contract table.
 *
 * Read from `uiEmitters`, which is that table as data and checked against it by
 * `components/ui/index.test.ts`: a component that starts emitting is inert here by having been
 * documented, not by somebody remembering to add it to a list.
 */
const inert = computed(() => (spec.value ? uiEmitters.has(spec.value.component) : false))

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
  return view && slug ? (store.viewState(slug, view)?.rows ?? []) : undefined
})

/**
 * What is wrong with the view this box is fed from, if anything.
 *
 * A name in `view` is an agent's, exactly as `ref_id` is, and it is wrong in the same ways. Shown
 * instead of the component: the rows ARE the panel, and a table reading "No rows" for a view that
 * does not exist is the one answer that is certainly false.
 */
const feed = computed(() => {
  const view = spec.value?.view
  const slug = props.data.slug
  if (!view || !slug) return undefined
  return viewProblem(view, store.shape?.views, store.viewState(slug, view)?.error)
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
      <span class="named">{{ named }}</span>
    </header>

    <p v-if="refusal" class="refused">{{ refusal }}</p>
    <p v-else-if="feed" class="refused">{{ feed }}</p>
    <div v-else-if="component" class="body nodrag nowheel">
      <!-- `inert` makes this subtree take no input, because there is nowhere for what it emits
           to go. It is the whole mechanism only for a component that stays in this subtree, which
           is why one that does not is refused above rather than rendered here. -->
      <div :inert="inert || undefined">
        <component :is="component" v-bind="bound" />
      </div>
    </div>

    <p v-if="inert && !refusal && !feed" class="note">
      Display only: nothing can keep what this component emits. Its contents cannot be selected,
      searched, or read by assistive technology.
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

.note {
  margin: 0;
  padding: 0 var(--space-3) var(--space-2);
  color: var(--color-text-muted);
  font-size: 0.6875rem;
}

.refused {
  margin: 0;
  padding: var(--space-3);
  color: var(--color-warn);
  font-size: 0.8125rem;
}
</style>

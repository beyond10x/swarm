<script setup lang="ts">
import { ref, useId, watch } from 'vue'
import UiIcon from './UiIcon.vue'

const props = withDefaults(defineProps<{ title?: string; collapsible?: boolean; open?: boolean }>(), {
  title: undefined,
  collapsible: false,
  open: true,
})
defineSlots<{ default?(): any; actions?(): any }>()

// `open` seeds and drives the state; the toggle is internal (the contract declares no emit).
const expanded = ref(props.open)
watch(() => props.open, (v) => { expanded.value = v })
const bodyId = useId()

function toggle() {
  if (props.collapsible) expanded.value = !expanded.value
}
</script>

<template>
  <section class="ui-panel" :class="{ collapsed: !expanded }">
    <header v-if="title || collapsible || $slots.actions" class="header">
      <button v-if="collapsible" type="button" class="toggle" :aria-expanded="expanded" :aria-controls="bodyId" @click="toggle">
        <UiIcon name="chevron-right" :size="14" class="chevron" />
        <span class="title">{{ title }}</span>
      </button>
      <h3 v-else class="title">{{ title }}</h3>
      <div v-if="$slots.actions" class="actions"><slot name="actions" /></div>
    </header>
    <div v-show="expanded" :id="bodyId" class="body"><slot /></div>
  </section>
</template>

<style scoped>
.ui-panel { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-2); min-width: 0; }
.header { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--color-border); min-height: var(--space-8); }
.collapsed .header { border-bottom-color: transparent; }
.title { margin: 0; font-size: 13px; font-weight: 600; text-transform: uppercase; letter-spacing: .04em; color: var(--color-text-muted); }
.toggle {
  display: inline-flex; align-items: center; gap: var(--space-2); padding: var(--space-1) var(--space-2); margin-left: calc(-1 * var(--space-2));
  border: 0; border-radius: var(--radius-1); background: transparent; color: inherit; font: inherit; cursor: pointer;
}
.toggle:hover { background: var(--color-surface-2); }
.toggle:hover .title { color: var(--color-text); }
.toggle:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 1px; }
.chevron { color: var(--color-text-muted); transition: transform .12s; }
.toggle[aria-expanded="true"] .chevron { transform: rotate(90deg); }
.actions { display: flex; align-items: center; gap: var(--space-2); margin-left: auto; }
.body { padding: var(--space-3); }
</style>

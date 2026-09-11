<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ modelValue: string; tabs: { value: string; label: string }[] }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const buttons: (HTMLButtonElement | null)[] = []
function setRef(el: unknown, i: number) {
  buttons[i] = (el as HTMLButtonElement | null) ?? null
}

// The roving tabindex lands on the active tab, or the first one when nothing matches.
const focusIndex = computed(() => Math.max(0, props.tabs.findIndex((t) => t.value === props.modelValue)))

function select(i: number) {
  const tab = props.tabs[i]
  if (!tab) return
  emit('update:modelValue', tab.value)
  buttons[i]?.focus()
}

function onKeydown(e: KeyboardEvent, i: number) {
  const n = props.tabs.length
  if (!n) return
  let next: number | undefined
  if (e.key === 'ArrowRight') next = (i + 1) % n
  else if (e.key === 'ArrowLeft') next = (i - 1 + n) % n
  else if (e.key === 'Home') next = 0
  else if (e.key === 'End') next = n - 1
  if (next === undefined) return
  e.preventDefault()
  select(next)
}
</script>

<template>
  <div class="ui-tabs" role="tablist">
    <button
      v-for="(tab, i) in tabs"
      :key="tab.value"
      :ref="(el) => setRef(el, i)"
      type="button"
      role="tab"
      class="tab"
      :class="{ active: tab.value === modelValue }"
      :aria-selected="tab.value === modelValue"
      :tabindex="i === focusIndex ? 0 : -1"
      @click="select(i)"
      @keydown="onKeydown($event, i)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>

<style scoped>
.ui-tabs { display: flex; gap: var(--space-1); border-bottom: 1px solid var(--color-border); overflow-x: auto; }
.tab {
  padding: var(--space-2) var(--space-3); margin-bottom: -1px;
  border: 0; border-bottom: 2px solid transparent; border-radius: var(--radius-1) var(--radius-1) 0 0;
  background: transparent; color: var(--color-text-muted); font: inherit; font-weight: 500; white-space: nowrap; cursor: pointer;
  transition: color .12s, border-color .12s;
}
.tab:hover { color: var(--color-text); }
.tab.active { color: var(--color-text); border-bottom-color: var(--color-accent); }
.tab:focus-visible { outline: 2px solid var(--color-accent); outline-offset: -2px; }
</style>

<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{ icon: string; label: string; size?: 'sm' | 'md' | 'lg' }>(), { size: 'md' })
const emit = defineEmits<{ click: [event: MouseEvent] }>()
</script>

<template>
  <button type="button" class="ui-icon-button" :class="size" :aria-label="label" :title="label" @click="emit('click', $event)">
    <UiIcon :name="icon" :size="size === 'sm' ? 14 : size === 'lg' ? 20 : 16" />
  </button>
</template>

<style scoped>
.ui-icon-button {
  display: inline-flex; align-items: center; justify-content: center; flex: none; padding: 0;
  border: 1px solid transparent; border-radius: var(--radius-1);
  background: transparent; color: var(--color-text-muted); cursor: pointer;
  transition: background-color .12s, color .12s;
}
.sm { width: var(--space-5); height: var(--space-5); }
.md { width: var(--space-6); height: var(--space-6); }
.lg { width: calc(var(--space-6) + var(--space-2)); height: calc(var(--space-6) + var(--space-2)); }
.ui-icon-button:hover:not(:disabled) { background: var(--color-surface-2); color: var(--color-text); }
.ui-icon-button:active:not(:disabled) { border-color: var(--color-border); }
.ui-icon-button:disabled { opacity: .5; cursor: not-allowed; }
.ui-icon-button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
</style>

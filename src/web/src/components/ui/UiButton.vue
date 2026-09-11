<script setup lang="ts">
import './ui.css'
import UiIcon from './UiIcon.vue'
import UiSpinner from './UiSpinner.vue'

withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'sm' | 'md' | 'lg'
    disabled?: boolean
    loading?: boolean
    icon?: string
  }>(),
  { variant: 'secondary', size: 'md', disabled: false, loading: false, icon: undefined },
)

const emit = defineEmits<{ click: [event: MouseEvent] }>()
defineSlots<{ default?(): any }>()
</script>

<template>
  <button
    type="button"
    class="ui-button"
    :class="[variant, size, { loading }]"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
    @click="emit('click', $event)"
  >
    <UiSpinner v-if="loading" size="sm" />
    <UiIcon v-else-if="icon" :name="icon" :size="size === 'sm' ? 14 : 16" />
    <span v-if="$slots.default" class="label"><slot /></span>
  </button>
</template>

<style scoped>
.ui-button {
  display: inline-flex; align-items: center; justify-content: center; gap: var(--space-2);
  border: 1px solid transparent; border-radius: var(--radius-1);
  font: inherit; font-weight: 500; color: var(--color-text);
  background: transparent; cursor: pointer; white-space: nowrap; user-select: none;
  transition: background-color .12s, border-color .12s, color .12s, filter .12s, transform .06s;
}
.sm { padding: var(--space-1) var(--space-2); font-size: 12px; line-height: 18px; }
.md { padding: var(--space-2) var(--space-3); font-size: 14px; line-height: 18px; }
.lg { padding: var(--space-3) var(--space-4); font-size: 15px; line-height: 18px; }

.primary { background: var(--color-accent); border-color: var(--color-accent); color: var(--color-bg); }
.secondary { background: var(--color-surface-2); border-color: var(--color-border); }
.ghost { color: var(--color-text-muted); }
.danger { background: var(--color-fault); border-color: var(--color-fault); color: var(--color-bg); }

.primary:hover:not(:disabled), .danger:hover:not(:disabled) { filter: brightness(1.1); }
.secondary:hover:not(:disabled) { border-color: var(--color-text-muted); }
.ghost:hover:not(:disabled) { background: var(--color-surface-2); color: var(--color-text); }
.ui-button:active:not(:disabled) { transform: translateY(1px); }
.ui-button:disabled { opacity: .5; cursor: not-allowed; }
.ui-button.loading { cursor: progress; }
.ui-button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
</style>

<script setup lang="ts">
// Label + control + hint/error. The default slot receives { id, describedBy, invalid } so the
// control it wraps can bind them; the four input components do exactly that.
import { computed, useId } from 'vue'

const props = withDefaults(defineProps<{ label: string; hint?: string; error?: string }>(), { hint: undefined, error: undefined })
defineSlots<{ default?(props: { id: string; describedBy: string | undefined; invalid: boolean }): any }>()

const id = useId()
const msgId = `${id}-msg`
const describedBy = computed(() => (props.error || props.hint ? msgId : undefined))
</script>

<template>
  <div class="ui-field" :class="{ invalid: !!error }">
    <label v-if="label" :for="id" class="label">{{ label }}</label>
    <slot :id="id" :describedBy="describedBy" :invalid="!!error" />
    <p v-if="error" :id="msgId" class="error" role="alert">{{ error }}</p>
    <p v-else-if="hint" :id="msgId" class="hint">{{ hint }}</p>
  </div>
</template>

<style scoped>
.ui-field { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.label { font-size: 13px; font-weight: 500; color: var(--color-text-muted); }
.invalid .label { color: var(--color-fault); }
.hint, .error { margin: 0; font-size: 12px; line-height: 1.4; }
.hint { color: var(--color-text-muted); }
.error { color: var(--color-fault); }
</style>

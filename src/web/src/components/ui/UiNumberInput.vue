<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import UiField from './UiField.vue'

defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{
    modelValue: number | undefined
    label?: string
    min?: number
    max?: number
    step?: number
    unit?: string
    error?: string
    mono?: boolean
  }>(),
  { label: '', min: undefined, max: undefined, step: undefined, unit: undefined, error: undefined, mono: false },
)
const emit = defineEmits<{ 'update:modelValue': [value: number | undefined] }>()

const attrs = useAttrs()
const rootAttrs = computed(() => ({ class: attrs.class, style: attrs.style }))
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})

// Cleared (or not-a-number) -> undefined, never NaN or ''.
function onInput(e: Event) {
  const el = e.target as HTMLInputElement
  if (el.value === '') return emit('update:modelValue', undefined)
  const n = el.valueAsNumber
  emit('update:modelValue', Number.isNaN(n) ? undefined : n)
}
</script>

<template>
  <UiField :label="label" :error="error" v-bind="rootAttrs" v-slot="{ id, describedBy, invalid }">
    <div class="ui-control number" :class="{ mono, invalid, 'has-unit': !!unit }">
      <input
        :id="id"
        type="number"
        class="raw"
        :value="modelValue ?? ''"
        :min="min"
        :max="max"
        :step="step"
        :aria-describedby="describedBy"
        :aria-invalid="invalid || undefined"
        v-bind="inputAttrs"
        @input="onInput"
      />
      <span v-if="unit" class="unit" aria-hidden="true">{{ unit }}</span>
    </div>
  </UiField>
</template>

<style scoped>
.number { display: flex; align-items: center; gap: var(--space-2); }
.raw {
  flex: 1; min-width: 0; padding: 0; border: 0; outline: none; background: transparent;
  color: inherit; font: inherit; appearance: textfield; -moz-appearance: textfield;
}
.raw::-webkit-inner-spin-button, .raw::-webkit-outer-spin-button { appearance: none; margin: 0; }
.unit { flex: none; color: var(--color-text-muted); font-size: 12px; user-select: none; }
</style>

<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import UiField from './UiField.vue'
import UiIcon from './UiIcon.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{ modelValue: string; options: { value: string; label: string }[]; label?: string; error?: string; mono?: boolean }>(),
  { label: '', error: undefined, mono: false },
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const attrs = useAttrs()
const rootAttrs = computed(() => ({ class: attrs.class, style: attrs.style }))
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})

const model = computed({
  get: () => props.modelValue,
  set: (v: string) => emit('update:modelValue', v),
})
// When the model matches no option, show an empty placeholder row instead of silently showing option 0.
const hasMatch = computed(() => props.options.some((o) => o.value === props.modelValue))
</script>

<template>
  <UiField :label="label" :error="error" v-bind="rootAttrs" v-slot="{ id, describedBy, invalid }">
    <div class="wrap">
      <select
        :id="id"
        v-model="model"
        class="ui-control select"
        :class="{ mono, invalid }"
        :aria-describedby="describedBy"
        :aria-invalid="invalid || undefined"
        v-bind="inputAttrs"
      >
        <option v-if="!hasMatch" value="" disabled hidden></option>
        <option v-for="o in options" :key="o.value" :value="o.value">{{ o.label }}</option>
      </select>
      <UiIcon name="chevron-down" :size="14" class="chevron" />
    </div>
  </UiField>
</template>

<style scoped>
.wrap { position: relative; min-width: 0; }
.select { appearance: none; padding-right: var(--space-6); cursor: pointer; }
.select option { background: var(--color-surface); color: var(--color-text); }
.chevron { position: absolute; right: var(--space-3); top: 50%; transform: translateY(-50%); pointer-events: none; color: var(--color-text-muted); }
</style>

<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import UiField from './UiField.vue'

defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{ modelValue: string; label?: string; placeholder?: string; rows?: number; error?: string; mono?: boolean }>(),
  { label: '', placeholder: undefined, rows: 3, error: undefined, mono: false },
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const attrs = useAttrs()
const rootAttrs = computed(() => ({ class: attrs.class, style: attrs.style }))
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})
</script>

<template>
  <UiField :label="label" :error="error" v-bind="rootAttrs" v-slot="{ id, describedBy, invalid }">
    <textarea
      :id="id"
      class="ui-control textarea"
      :class="{ mono, invalid }"
      :value="modelValue"
      :placeholder="placeholder"
      :rows="rows"
      :aria-describedby="describedBy"
      :aria-invalid="invalid || undefined"
      v-bind="inputAttrs"
      @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    />
  </UiField>
</template>

<style scoped>
.textarea { resize: vertical; line-height: 1.45; }
</style>

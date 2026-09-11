<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import UiField from './UiField.vue'

defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{ modelValue: string; label?: string; placeholder?: string; error?: string; mono?: boolean }>(),
  { label: '', placeholder: undefined, error: undefined, mono: false },
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

// class/style stay on the field wrapper; everything else (type, disabled, autocomplete, ...) reaches the <input>.
const attrs = useAttrs()
const rootAttrs = computed(() => ({ class: attrs.class, style: attrs.style }))
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})
</script>

<template>
  <UiField :label="label" :error="error" v-bind="rootAttrs" v-slot="{ id, describedBy, invalid }">
    <input
      :id="id"
      type="text"
      class="ui-control"
      :class="{ mono, invalid }"
      :value="modelValue"
      :placeholder="placeholder"
      :aria-describedby="describedBy"
      :aria-invalid="invalid || undefined"
      v-bind="inputAttrs"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
  </UiField>
</template>

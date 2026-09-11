<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{ text: string; removable?: boolean }>(), { removable: false })
const emit = defineEmits<{ remove: [] }>()
</script>

<template>
  <span class="ui-tag" :class="{ removable }">
    <span class="text">{{ text }}</span>
    <button v-if="removable" type="button" class="remove" :aria-label="`Remove ${text}`" @click="emit('remove')">
      <UiIcon name="close" :size="12" />
    </button>
  </span>
</template>

<style scoped>
.ui-tag {
  display: inline-flex; align-items: center; gap: var(--space-1);
  padding: 0 var(--space-2); border-radius: var(--radius-1);
  background: var(--color-surface-2); border: 1px solid var(--color-border);
  color: var(--color-text); font-size: 12px; line-height: 22px; white-space: nowrap;
}
.removable { padding-right: var(--space-1); }
.remove {
  display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px;
  padding: 0; border: 0; border-radius: var(--radius-1); background: transparent;
  color: var(--color-text-muted); cursor: pointer;
}
.remove:hover { background: var(--color-border); color: var(--color-text); }
.remove:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 1px; }
</style>

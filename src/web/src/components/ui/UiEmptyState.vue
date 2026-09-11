<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{ title: string; text?: string; icon?: string }>(), { text: undefined, icon: 'info' })
defineSlots<{ default?(): any }>()
</script>

<template>
  <div class="ui-empty">
    <div v-if="icon" class="icon"><UiIcon :name="icon" :size="32" /></div>
    <h3 class="title">{{ title }}</h3>
    <p v-if="text" class="text">{{ text }}</p>
    <div v-if="$slots.default" class="actions"><slot /></div>
  </div>
</template>

<style scoped>
.ui-empty { display: flex; flex-direction: column; align-items: center; text-align: center; padding: var(--space-8) var(--space-5); color: var(--color-text-muted); }
.icon { color: var(--color-text-muted); opacity: .7; margin-bottom: var(--space-3); }
.title { margin: 0; font-size: 16px; font-weight: 600; color: var(--color-text); }
.text { margin: var(--space-2) 0 0; max-width: 42em; line-height: 1.5; }
.actions { display: flex; gap: var(--space-2); margin-top: var(--space-4); }
</style>

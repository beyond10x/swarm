<script setup lang="ts">
withDefaults(defineProps<{ items: { key: string; value: unknown }[]; mono?: boolean }>(), { mono: false })

function fmt(v: unknown): string {
  if (v === null || v === undefined || v === '') return '—'
  if (typeof v === 'boolean') return v ? 'yes' : 'no'
  if (typeof v === 'object') return JSON.stringify(v)
  return String(v)
}
</script>

<template>
  <dl class="ui-kv" :class="{ mono }">
    <template v-for="item in items" :key="item.key">
      <dt>{{ item.key }}</dt>
      <dd>{{ fmt(item.value) }}</dd>
    </template>
  </dl>
</template>

<style scoped>
.ui-kv { display: grid; grid-template-columns: auto 1fr; gap: var(--space-1) var(--space-4); margin: 0; min-width: 0; }
dt { color: var(--color-text-muted); white-space: nowrap; }
dd { margin: 0; min-width: 0; overflow-wrap: anywhere; }
.mono dd { font-family: var(--font-mono); font-size: 13px; }
</style>

<script setup lang="ts">
withDefaults(defineProps<{ tone?: 'ok' | 'warn' | 'fault' | 'info' | 'muted'; text: string }>(), { tone: 'muted' })
</script>

<template>
  <span class="ui-badge" :class="tone">{{ text }}</span>
</template>

<style scoped>
.ui-badge {
  display: inline-flex; align-items: center; gap: var(--space-1);
  padding: 0 var(--space-2); border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--tone) 40%, transparent);
  background: color-mix(in srgb, var(--tone) 14%, transparent);
  color: var(--tone); font-size: 12px; font-weight: 600; line-height: 20px;
  white-space: nowrap; letter-spacing: .01em;
}
.ui-badge::before { content: ''; width: 6px; height: 6px; border-radius: 50%; background: var(--tone); flex: none; }
.ok { --tone: var(--color-ok); }
.warn { --tone: var(--color-warn); }
.fault { --tone: var(--color-fault); }
.info { --tone: var(--color-info); }
.muted { --tone: var(--color-text-muted); }
</style>

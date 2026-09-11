<script setup lang="ts">
withDefaults(defineProps<{ size?: 'sm' | 'md' }>(), { size: 'md' })
</script>

<template>
  <span class="ui-spinner" :class="size" role="status" aria-label="Loading">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" aria-hidden="true">
      <circle class="track" cx="12" cy="12" r="9" />
      <path d="M21 12a9 9 0 0 0-9-9" />
    </svg>
  </span>
</template>

<style scoped>
.ui-spinner { display: inline-flex; flex: none; color: currentColor; }
.ui-spinner svg { display: block; animation: spin .8s linear infinite; }
.sm svg { width: 14px; height: 14px; }
.md svg { width: 20px; height: 20px; }
.track { opacity: .25; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>

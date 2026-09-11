<script setup lang="ts">
defineSlots<{ default?(): any; right?(): any }>()
</script>

<template>
  <div class="ui-toolbar">
    <div class="left"><slot /></div>
    <div v-if="$slots.right" class="right"><slot name="right" /></div>
  </div>
</template>

<style scoped>
.ui-toolbar { display: flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; min-width: 0; }
.left, .right { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
.right { margin-left: auto; }
</style>

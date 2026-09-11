<script setup lang="ts">
withDefaults(defineProps<{ title?: string }>(), { title: undefined })
defineSlots<{ default?(): any; header?(): any; footer?(): any }>()
</script>

<template>
  <section class="ui-card">
    <header v-if="title || $slots.header" class="header">
      <h3 v-if="title" class="title">{{ title }}</h3>
      <div v-if="$slots.header" class="header-extra"><slot name="header" /></div>
    </header>
    <div class="body"><slot /></div>
    <footer v-if="$slots.footer" class="footer"><slot name="footer" /></footer>
  </section>
</template>

<style scoped>
.ui-card {
  display: flex; flex-direction: column; min-width: 0;
  background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-2);
  box-shadow: var(--shadow-1);
}
.header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--color-border); }
.title { margin: 0; font-size: 14px; font-weight: 600; }
.header-extra { display: flex; align-items: center; gap: var(--space-2); margin-left: auto; }
.body { padding: var(--space-4); flex: 1; }
.footer { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-top: 1px solid var(--color-border); }
</style>

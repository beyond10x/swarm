<script setup lang="ts">
// Owned by the integrator. The shell is deliberately thin: a top bar and the routed view.
import { onMounted } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import RuntimeBar from '@/components/runtime/RuntimeBar.vue'
import { useSwarmStore } from '@/stores/swarms'

const store = useSwarmStore()
// The clock and the status poll run for as long as the shell does: every page reads them.
onMounted(() => store.wake())
</script>

<template>
  <div class="shell">
    <header class="topbar">
      <RouterLink to="/" class="brand">swarm</RouterLink>
      <nav>
        <RouterLink to="/">swarms</RouterLink>
        <RouterLink to="/ui">components</RouterLink>
      </nav>
      <RuntimeBar class="runtime" />
    </header>
    <main class="content"><RouterView /></main>
  </div>
</template>

<style scoped>
.shell { height: 100vh; display: flex; flex-direction: column; }
.topbar { display: flex; align-items: center; gap: var(--space-4); padding: var(--space-2) var(--space-4); border-bottom: 1px solid var(--color-border); background: var(--color-surface); }
.brand { font-weight: 700; letter-spacing: 0.02em; color: var(--color-text); text-decoration: none; }
nav { display: flex; gap: var(--space-3); }
nav a { color: var(--color-text-muted); text-decoration: none; }
nav a.router-link-active { color: var(--color-accent); }
.runtime { margin-left: var(--space-4); justify-content: flex-end; }
.content { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: auto; }
</style>

<script setup lang="ts">
// Owned by the integrator. The shell is deliberately thin: a top bar and the routed view.
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <div class="shell">
    <header class="topbar">
      <RouterLink to="/" class="brand">swarm2</RouterLink>
      <nav>
        <RouterLink to="/">swarms</RouterLink>
        <RouterLink to="/ui">components</RouterLink>
      </nav>
    </header>
    <main class="content"><RouterView /></main>
  </div>
</template>

<style scoped>
.shell { min-height: 100vh; display: flex; flex-direction: column; }
.topbar { display: flex; align-items: center; gap: var(--space-4); padding: var(--space-2) var(--space-4); border-bottom: 1px solid var(--color-border); background: var(--color-surface); }
.brand { font-weight: 700; letter-spacing: 0.02em; color: var(--color-text); text-decoration: none; }
nav { display: flex; gap: var(--space-3); }
nav a { color: var(--color-text-muted); text-decoration: none; }
nav a.router-link-active { color: var(--color-accent); }
.content { flex: 1; display: flex; flex-direction: column; min-height: 0; }
</style>

<script setup lang="ts">
// Internal helper (not exported from index.ts): renders one glyph from ./icons.ts by name.
import { computed } from 'vue'
import { resolveIcon } from './icons'

const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 16 })
const def = computed(() => resolveIcon(props.name))
</script>

<template>
  <svg
    v-if="def"
    class="ui-icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    :fill="def.fill ? 'currentColor' : 'none'"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path :d="def.d" />
  </svg>
</template>

<style scoped>
.ui-icon { flex: none; display: inline-block; vertical-align: -0.15em; }
</style>

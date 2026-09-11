<script setup lang="ts">
// A UiBadge whose tone is fixed by the lifecycle state (see the map in index.ts).
import { computed } from 'vue'
import UiBadge from './UiBadge.vue'

type Tone = 'ok' | 'warn' | 'fault' | 'info' | 'muted'

const TONE_BY_STATE: Record<string, Tone> = {
  running: 'ok', working: 'ok', succeeded: 'ok', answered: 'ok',
  blocked: 'warn', parked: 'warn', paused: 'warn', held: 'warn',
  faulted: 'fault', failed: 'fault', escalated: 'fault',
  idle: 'info', created: 'info', draft: 'info', open: 'info',
}

const props = defineProps<{ state: string }>()
const tone = computed<Tone>(() => TONE_BY_STATE[props.state.toLowerCase()] ?? 'muted')
</script>

<template>
  <UiBadge :tone="tone" :text="state" />
</template>

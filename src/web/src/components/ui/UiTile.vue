<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{ title: string; subtitle?: string; accent?: string; big?: boolean; clickable?: boolean }>(),
  { subtitle: undefined, accent: undefined, big: false, clickable: false },
)
const emit = defineEmits<{ click: [event: MouseEvent | KeyboardEvent] }>()
defineSlots<{ default?(): any; footer?(): any }>()

// `accent` is a token colour name (accent, accent-2, ok, warn, fault, info, ...) -> var(--color-<name>).
const style = computed(() => (props.accent ? { '--tile-accent': `var(--color-${props.accent})` } : undefined))

function onKeydown(e: KeyboardEvent) {
  if (!props.clickable) return
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    emit('click', e)
  }
}
</script>

<template>
  <div
    class="ui-tile"
    :class="{ big, clickable }"
    :style="style"
    :role="clickable ? 'button' : undefined"
    :tabindex="clickable ? 0 : undefined"
    @click="emit('click', $event)"
    @keydown="onKeydown"
  >
    <div class="body">
      <h3 class="title">{{ title }}</h3>
      <p v-if="subtitle" class="subtitle">{{ subtitle }}</p>
      <div v-if="$slots.default" class="content"><slot /></div>
    </div>
    <div v-if="$slots.footer" class="footer"><slot name="footer" /></div>
  </div>
</template>

<style scoped>
.ui-tile {
  position: relative; display: flex; flex-direction: column;
  min-width: 280px; min-height: 180px;
  padding: var(--space-4); padding-left: calc(var(--space-4) + 4px);
  background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-2);
  box-shadow: var(--shadow-1); overflow: hidden; color: var(--color-text);
  transition: transform .12s, box-shadow .12s, border-color .12s;
}
.ui-tile::before { content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 4px; background: var(--tile-accent, transparent); }
.big { padding: var(--space-6); padding-left: calc(var(--space-6) + 4px); }
.body { flex: 1; min-width: 0; }
.title { margin: 0; font-size: 16px; font-weight: 600; line-height: 1.25; overflow-wrap: anywhere; }
.big .title { font-size: 28px; font-weight: 700; }
.subtitle {
  margin: var(--space-1) 0 0; color: var(--color-text-muted); font-size: 13px; line-height: 1.4;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
}
.big .subtitle { font-size: 15px; margin-top: var(--space-2); }
.content { margin-top: var(--space-3); }
.footer {
  display: flex; align-items: center; gap: var(--space-2);
  margin-top: var(--space-3); padding-top: var(--space-3); border-top: 1px solid var(--color-border);
}
.clickable { cursor: pointer; }
.clickable:hover { transform: translateY(-2px); box-shadow: var(--shadow-2); border-color: var(--color-text-muted); }
.clickable:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
</style>

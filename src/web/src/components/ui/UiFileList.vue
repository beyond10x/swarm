<script setup lang="ts">
import { ref, useId, watch } from 'vue'
import UiIcon from './UiIcon.vue'

type FileRow = { path: string; size?: number; mtime?: string | number | Date; kind?: 'file' | 'dir' }

const props = withDefaults(defineProps<{ files: FileRow[]; selected?: string }>(), { selected: undefined })
const emit = defineEmits<{ select: [path: string] }>()

const id = useId()
const root = ref<HTMLElement>()
// The keyboard cursor; follows `selected` when the parent changes it.
const cursor = ref(-1)
watch(
  () => [props.selected, props.files] as const,
  ([sel, files]) => {
    const i = files.findIndex((f) => f.path === sel)
    if (i >= 0) cursor.value = i
    else if (cursor.value >= files.length) cursor.value = files.length - 1
  },
  { immediate: true },
)

function moveTo(i: number) {
  const n = props.files.length
  if (!n) return
  cursor.value = Math.min(n - 1, Math.max(0, i))
  root.value?.querySelector<HTMLElement>(`[data-index="${cursor.value}"]`)?.scrollIntoView({ block: 'nearest' })
}

function onKeydown(e: KeyboardEvent) {
  switch (e.key) {
    case 'ArrowDown': e.preventDefault(); moveTo(cursor.value + 1); break
    case 'ArrowUp': e.preventDefault(); moveTo(cursor.value < 0 ? props.files.length - 1 : cursor.value - 1); break
    case 'Home': e.preventDefault(); moveTo(0); break
    case 'End': e.preventDefault(); moveTo(props.files.length - 1); break
    case 'Enter':
    case ' ': {
      e.preventDefault()
      const f = props.files[cursor.value]
      if (f) emit('select', f.path)
      break
    }
  }
}

function pick(i: number) {
  cursor.value = i
  const f = props.files[i]
  if (f) emit('select', f.path)
}

function humanSize(n?: number): string {
  if (n === undefined || n === null) return '—'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = n
  let u = 0
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
  return `${u === 0 ? v : v.toFixed(v < 10 ? 1 : 0)} ${units[u]}`
}

function relTime(t?: string | number | Date): string {
  if (t === undefined || t === null || t === '') return '—'
  const d = new Date(t)
  const ms = d.getTime()
  if (Number.isNaN(ms)) return String(t)
  const s = Math.round((Date.now() - ms) / 1000)
  if (s < 45) return 'just now'
  const m = Math.round(s / 60)
  if (m < 60) return `${m} min ago`
  const h = Math.round(m / 60)
  if (h < 24) return `${h} h ago`
  const days = Math.round(h / 24)
  if (days < 30) return `${days} d ago`
  return d.toISOString().slice(0, 10)
}
</script>

<template>
  <div
    ref="root"
    class="ui-filelist"
    role="listbox"
    tabindex="0"
    :aria-activedescendant="cursor >= 0 ? `${id}-${cursor}` : undefined"
    @keydown="onKeydown"
  >
    <div
      v-for="(f, i) in files"
      :id="`${id}-${i}`"
      :key="f.path"
      :data-index="i"
      role="option"
      class="row"
      :class="{ selected: f.path === selected, cursor: i === cursor, dir: f.kind === 'dir' }"
      :aria-selected="f.path === selected"
      @click="pick(i)"
    >
      <UiIcon :name="f.kind === 'dir' ? 'folder' : 'file'" :size="16" class="kind" />
      <span class="path" :title="f.path">{{ f.path }}</span>
      <span class="size">{{ f.kind === 'dir' ? '' : humanSize(f.size) }}</span>
      <span class="mtime">{{ relTime(f.mtime) }}</span>
    </div>
    <div v-if="!files.length" class="empty">No files</div>
  </div>
</template>

<style scoped>
.ui-filelist { display: flex; flex-direction: column; min-width: 0; overflow: auto; border: 1px solid var(--color-border); border-radius: var(--radius-2); background: var(--color-surface); font-size: 13px; }
.ui-filelist:focus-visible { outline: 2px solid var(--color-accent); outline-offset: -2px; }
.row { display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; align-items: center; gap: var(--space-3); padding: var(--space-1) var(--space-3); cursor: pointer; border-left: 2px solid transparent; }
.row:hover { background: var(--color-surface-2); }
.row.selected { background: color-mix(in srgb, var(--color-accent) 16%, transparent); border-left-color: var(--color-accent); }
.ui-filelist:focus .row.cursor { box-shadow: inset 0 0 0 1px var(--color-accent); }
.kind { color: var(--color-text-muted); }
.dir .kind { color: var(--color-warn); }
.path { font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dir .path { font-weight: 600; }
.size { font-family: var(--font-mono); color: var(--color-text-muted); text-align: right; white-space: nowrap; }
.mtime { color: var(--color-text-muted); white-space: nowrap; }
.empty { padding: var(--space-5); text-align: center; color: var(--color-text-muted); }
</style>

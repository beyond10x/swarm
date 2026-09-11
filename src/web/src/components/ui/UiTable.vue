<script setup lang="ts" generic="Row extends object">
// Generic over the row type so `Swarm[]`-style interfaces pass without a cast; the contract's
// `Record<string, unknown>[]` is a subset of `object[]`.
import { computed, getCurrentInstance } from 'vue'

const props = withDefaults(
  defineProps<{
    columns: { key: string; label: string; width?: string; align?: string }[]
    rows: Row[]
    rowKey?: string
    dense?: boolean
  }>(),
  { rowKey: 'id', dense: false },
)
const emit = defineEmits<{ 'row-click': [row: Row] }>()
defineSlots<{ empty?(): any } & { [K in `cell-${string}`]?: (props: { row: Row; value: unknown }) => any }>()

// Rows only look clickable when somebody listens.
const instance = getCurrentInstance()
const clickable = computed(() => !!instance?.vnode.props?.onRowClick)

function cell(row: Row, key: string): unknown {
  return (row as Record<string, unknown>)[key]
}

function keyOf(row: Row, i: number): string {
  const k = cell(row, props.rowKey)
  return k === undefined || k === null ? String(i) : String(k)
}

function fmt(v: unknown): string {
  if (v === null || v === undefined || v === '') return '—'
  if (typeof v === 'boolean') return v ? 'yes' : 'no'
  if (typeof v === 'object') return JSON.stringify(v)
  return String(v)
}

function onRowKeydown(e: KeyboardEvent, row: Row) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    emit('row-click', row)
  }
}
</script>

<template>
  <div class="ui-table-wrap" :class="{ dense, clickable }">
    <table class="ui-table">
      <colgroup>
        <col v-for="c in columns" :key="c.key" :style="c.width ? { width: c.width } : undefined" />
      </colgroup>
      <thead>
        <tr>
          <th v-for="c in columns" :key="c.key" scope="col" :class="c.align ?? 'left'">{{ c.label }}</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, i) in rows"
          :key="keyOf(row, i)"
          class="row"
          :tabindex="clickable ? 0 : undefined"
          @click="emit('row-click', row)"
          @keydown="clickable && onRowKeydown($event, row)"
        >
          <td v-for="c in columns" :key="c.key" :class="c.align ?? 'left'">
            <slot :name="`cell-${c.key}`" :row="row" :value="cell(row, c.key)">{{ fmt(cell(row, c.key)) }}</slot>
          </td>
        </tr>
        <tr v-if="!rows.length" class="empty-row">
          <td :colspan="columns.length">
            <slot name="empty"><span class="empty">No rows</span></slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.ui-table-wrap { position: relative; overflow: auto; min-width: 0; border: 1px solid var(--color-border); border-radius: var(--radius-2); background: var(--color-surface); }
.ui-table { width: 100%; border-collapse: separate; border-spacing: 0; font-size: 14px; }
th, td { padding: var(--space-2) var(--space-3); text-align: left; vertical-align: middle; border-bottom: 1px solid var(--color-border); }
.dense th, .dense td { padding: var(--space-1) var(--space-2); font-size: 13px; }
th { position: sticky; top: 0; z-index: 1; background: var(--color-surface-2); color: var(--color-text-muted); font-weight: 600; font-size: 12px; text-transform: uppercase; letter-spacing: .04em; white-space: nowrap; }
tbody tr:last-child td { border-bottom: 0; }
.center { text-align: center; }
.right { text-align: right; }
.clickable .row { cursor: pointer; }
.clickable .row:hover { background: var(--color-surface-2); }
.row:focus-visible { outline: 2px solid var(--color-accent); outline-offset: -2px; }
.empty-row td { padding: var(--space-5); text-align: center; color: var(--color-text-muted); }
</style>

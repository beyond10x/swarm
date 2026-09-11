<script setup lang="ts">
// Vue Flow custom node for a Box: a blackbox whose only visible surface is its typed ports.
// Rendered via the `#node-box` slot of <VueFlow>; `data.box` is the Box from the model.
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { Box, BoxKind } from '@/model'
import { UiBadge } from '@/components/ui'

export interface BoxNodeData { box: Box }

defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: BoxNodeData; selected?: boolean }>()

const box = computed(() => props.data.box)

const KIND_TONE: Record<BoxKind, 'ok' | 'warn' | 'fault' | 'info' | 'muted'> = {
  flow: 'info', service: 'ok', tool: 'muted', store: 'warn', agent: 'info', swarm: 'muted',
}
const tone = computed(() => KIND_TONE[box.value.kind] ?? 'muted')
</script>

<template>
  <div class="box-node" :class="{ selected: props.selected, ['kind-' + box.kind]: true }">
    <header class="box-head" :title="box.summary ?? box.name">
      <UiBadge :tone="tone" :text="box.kind" />
      <span class="name">{{ box.name }}</span>
      <span class="version">v{{ box.version }}</span>
      <span v-if="box.ownerAgentId" class="owner">{{ box.ownerAgentId }}</span>
    </header>
    <div class="ports">
      <ul class="col col-in">
        <li v-for="p in box.inputs" :key="'in:' + p.name" class="port" :title="p.schemaId">
          <Handle :id="p.name" type="target" :position="Position.Left" />
          <span class="pname">{{ p.name }}<span v-if="p.required" class="req">*</span></span>
          <span class="schema">{{ p.schemaId }}</span>
        </li>
        <li v-if="box.inputs.length === 0" class="port none">no inputs</li>
      </ul>
      <ul class="col col-out">
        <li v-for="p in box.outputs" :key="'out:' + p.name" class="port" :title="p.schemaId">
          <span class="schema">{{ p.schemaId }}</span>
          <span class="pname">{{ p.name }}</span>
          <Handle :id="p.name" type="source" :position="Position.Right" />
        </li>
        <li v-if="box.outputs.length === 0" class="port none">no outputs</li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.box-node {
  width: 260px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-1);
  color: var(--color-text);
  font-size: 12px;
  overflow: visible;
}
.box-node.selected { border-color: var(--color-accent); box-shadow: 0 0 0 1px var(--color-accent), var(--shadow-2); }
.box-node.kind-flow { border-top: 2px solid var(--color-info); }
.box-node.kind-service { border-top: 2px solid var(--color-ok); }
.box-node.kind-store { border-top: 2px solid var(--color-warn); }
.box-node.kind-tool, .box-node.kind-swarm { border-top: 2px solid var(--color-text-muted); }
.box-node.kind-agent { border-top: 2px solid var(--color-accent-2); }
.box-head {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface-2);
  border-radius: var(--radius-2) var(--radius-2) 0 0;
  min-height: 36px;
}
.name { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; }
.version { color: var(--color-text-muted); font-family: var(--font-mono); font-size: 11px; }
.owner { color: var(--color-accent-2); font-family: var(--font-mono); font-size: 11px; }
.ports { display: flex; justify-content: space-between; gap: var(--space-2); padding: var(--space-2) 0; }
.col { list-style: none; margin: 0; padding: 0; flex: 1; min-width: 0; }
.port {
  position: relative;
  display: flex; align-items: center; gap: var(--space-1);
  height: 22px; line-height: 22px;
  white-space: nowrap; overflow: hidden;
}
.col-in .port { padding-left: var(--space-3); justify-content: flex-start; }
.col-out .port { padding-right: var(--space-3); justify-content: flex-end; }
.port.none { color: var(--color-text-muted); font-style: italic; }
.pname { font-weight: 500; }
.req { color: var(--color-warn); }
.schema { color: var(--color-text-muted); font-family: var(--font-mono); font-size: 10px; overflow: hidden; text-overflow: ellipsis; }
/* Vue Flow's handle is position:absolute and top:50%; the port row is its positioned ancestor. */
.port :deep(.vue-flow__handle-left) { left: -5px; }
.port :deep(.vue-flow__handle-right) { right: -5px; }
</style>

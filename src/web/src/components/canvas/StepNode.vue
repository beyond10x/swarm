<script lang="ts">
/** Handle ids for control-flow edges (an Edge without wires). `$` keeps them clear of port names. */
export const STEP_FLOW_IN = '$in'
export const STEP_FLOW_OUT = '$out'
</script>

<script setup lang="ts">
// Vue Flow custom node for a Flow Step. Control-flow handles `$in` / `$out` sit on the header so a
// plain (unwired) DAG edge has somewhere to land; typed ports are handles on their own rows.
// Rendered via the `#node-step` slot of <VueFlow>.
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { Handle, Position } from '@vue-flow/core'
import type { Step, StepKind } from '@/model'
import { UiBadge, UiButton } from '@/components/ui'

export interface StepNodeData { step: Step; swarmId: string }


defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: StepNodeData; selected?: boolean }>()
const router = useRouter()

const step = computed(() => props.data.step)

const KIND_TONE: Record<StepKind, 'ok' | 'warn' | 'fault' | 'info' | 'muted'> = {
  command: 'muted', tool: 'warn', llm: 'info', subflow: 'ok',
}
const tone = computed(() => KIND_TONE[step.value.kind] ?? 'muted')

/** The one line that says what the step does: run / tool / prompt (first line), whichever exists. */
const gist = computed(() => {
  const s = step.value
  const src = s.run ?? s.tool ?? s.prompt ?? (s.subflowId ? `subflow ${s.subflowId}` : '')
  return src.split('\n')[0] ?? ''
})

function openSubflow() {
  if (!step.value.subflowId) return
  router.push({ name: 'flow', params: { id: props.data.swarmId, flowId: step.value.subflowId } })
}
</script>

<template>
  <div class="step-node" :class="{ selected: props.selected, ['kind-' + step.kind]: true }">
    <header class="head">
      <Handle :id="STEP_FLOW_IN" type="target" :position="Position.Left" class="flow-handle" title="control flow in" />
      <UiBadge :tone="tone" :text="step.kind" />
      <span class="name" :title="step.name">{{ step.name }}</span>
      <UiButton
        v-if="step.kind === 'subflow' && step.subflowId"
        variant="ghost" size="sm" class="nodrag open"
        @click="openSubflow"
      >open →</UiButton>
      <Handle :id="STEP_FLOW_OUT" type="source" :position="Position.Right" class="flow-handle" title="control flow out" />
    </header>
    <div v-if="gist" class="gist" :title="gist">{{ gist }}</div>
    <div class="ports">
      <ul class="col col-in">
        <li v-for="p in step.inputs" :key="'in:' + p.name" class="port" :title="p.schemaId">
          <Handle :id="p.name" type="target" :position="Position.Left" />
          <span class="pname">{{ p.name }}</span>
          <span class="schema">{{ p.schemaId }}</span>
        </li>
      </ul>
      <ul class="col col-out">
        <li v-for="p in step.outputs" :key="'out:' + p.name" class="port" :title="p.schemaId">
          <span class="schema">{{ p.schemaId }}</span>
          <span class="pname">{{ p.name }}</span>
          <Handle :id="p.name" type="source" :position="Position.Right" />
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.step-node {
  width: 240px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-1);
  color: var(--color-text);
  font-size: 12px;
}
.step-node.selected { border-color: var(--color-accent); box-shadow: 0 0 0 1px var(--color-accent), var(--shadow-2); }
.step-node.kind-llm { border-top: 2px solid var(--color-info); }
.step-node.kind-tool { border-top: 2px solid var(--color-warn); }
.step-node.kind-subflow { border-top: 2px solid var(--color-ok); }
.step-node.kind-command { border-top: 2px solid var(--color-text-muted); }
.head {
  position: relative;
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface-2);
  border-bottom: 1px solid var(--color-border);
  border-radius: var(--radius-2) var(--radius-2) 0 0;
  min-height: 36px;
}
.name { font-weight: 600; flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.open { flex: none; }
.gist {
  padding: var(--space-1) var(--space-3);
  font-family: var(--font-mono); font-size: 11px; color: var(--color-text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  border-bottom: 1px solid var(--color-border);
}
.ports { display: flex; justify-content: space-between; gap: var(--space-2); padding: var(--space-1) 0; }
.ports:empty { display: none; }
.col { list-style: none; margin: 0; padding: 0; flex: 1; min-width: 0; }
.port {
  position: relative; display: flex; align-items: center; gap: var(--space-1);
  height: 22px; line-height: 22px; white-space: nowrap; overflow: hidden;
}
.col-in .port { padding-left: var(--space-3); }
.col-out .port { padding-right: var(--space-3); justify-content: flex-end; }
.pname { font-weight: 500; }
.schema { color: var(--color-text-muted); font-family: var(--font-mono); font-size: 10px; overflow: hidden; text-overflow: ellipsis; }
.head :deep(.flow-handle) { width: 8px; height: 8px; border-radius: 2px; }
.port :deep(.vue-flow__handle-left) { left: -5px; }
.port :deep(.vue-flow__handle-right) { right: -5px; }
.head :deep(.vue-flow__handle-left) { left: -5px; }
.head :deep(.vue-flow__handle-right) { right: -5px; }
</style>

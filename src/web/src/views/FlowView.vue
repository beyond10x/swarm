<script setup lang="ts">
// A Flow's DAG: one node per Step, edges from the flow's edges (wired ports or plain control flow).
// Owned by W2. Route: /swarm/:id/flow/:flowId → props { id, flowId }.
import { computed, onMounted, ref, shallowRef, watch } from 'vue'
import { RouterLink } from 'vue-router'
import { VueFlow, useVueFlow, type Node, type Edge } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'
import '@/components/canvas/canvas-theme.css'
import type { Flow, Step } from '@/model'
import { layoutDag } from '@/lib/layout'
import StepNode, { STEP_FLOW_IN, STEP_FLOW_OUT, type StepNodeData } from '@/components/canvas/StepNode.vue'
import { useSwarmStore } from '@/stores/swarms'
import { UiPanel, UiBadge, UiKeyValue, UiEmptyState, UiSpinner } from '@/components/ui'

const props = defineProps<{ id: string; flowId: string }>()

const store = useSwarmStore()
const loading = ref(!store.loaded)
onMounted(async () => {
  if (!store.loaded) await store.load()
  loading.value = false
})

const swarm = computed(() => store.getSwarm(props.id))
const flow = computed<Flow | undefined>(() => swarm.value?.flows.find((f) => f.flowId === props.flowId))

// ---- geometry -----------------------------------------------------------------------------------
const STEP_W = 240
const PORT_ROW = 22
function stepHeight(s: Step): number {
  const gist = s.run ?? s.tool ?? s.prompt ?? s.subflowId
  return 36 + (gist ? 20 : 0) + 8 + Math.max(s.inputs.length, s.outputs.length) * PORT_ROW
}

// ---- graph --------------------------------------------------------------------------------------
const nodes = shallowRef<Node[]>([])
const edges = shallowRef<Edge[]>([])

function rebuild(f: Flow | undefined) {
  if (!f) { nodes.value = []; edges.value = []; return }
  const laid = layoutDag(
    f.steps.map((s) => ({ id: s.stepId, width: STEP_W, height: stepHeight(s) })),
    f.edges.map((e) => ({ from: e.from, to: e.to })),
    'LR',
  )
  nodes.value = f.steps.map<Node>((s) => ({
    id: s.stepId,
    type: 'step',
    position: laid.get(s.stepId) ?? { x: 0, y: 0 },
    data: { step: s, swarmId: props.id } satisfies StepNodeData,
  }))
  const out: Edge[] = []
  for (const e of f.edges) {
    if (e.wires && e.wires.length > 0) {
      for (const w of e.wires) {
        out.push({
          id: `${e.from}:${w.fromOutput}->${e.to}:${w.toInput}`,
          source: e.from, sourceHandle: w.fromOutput,
          target: e.to, targetHandle: w.toInput,
          label: e.when,
        })
      }
    } else {
      out.push({
        id: `${e.from}->${e.to}`,
        source: e.from, sourceHandle: STEP_FLOW_OUT,
        target: e.to, targetHandle: STEP_FLOW_IN,
        label: e.when,
        style: { strokeDasharray: '4 3' },
      })
    }
  }
  edges.value = out
}
watch(flow, rebuild, { immediate: true, deep: true })

// One store per view instance. Not derived from flowId: "open →" on a subflow step pushes a new
// flowId onto the same route, the component is reused, and the hooks must stay bound.
const vfId = `flow-view-${Math.random().toString(36).slice(2, 10)}`
const { onNodesInitialized, fitView, getSelectedNodes } = useVueFlow({ id: vfId })

onNodesInitialized(() => { fitView({ padding: 0.2 }) })

// ---- selection panel ----------------------------------------------------------------------------
const selectedStep = computed<Step | undefined>(() => {
  const n = getSelectedNodes.value[0]
  return n ? (n.data as StepNodeData).step : undefined
})

const selectedItems = computed<{ key: string; value: string }[]>(() => {
  const s = selectedStep.value
  if (!s) return []
  const items: { key: string; value: string }[] = [
    { key: 'name', value: s.name },
    { key: 'kind', value: s.kind },
  ]
  if (s.run) items.push({ key: 'run', value: s.run })
  if (s.tool) items.push({ key: 'tool', value: s.tool })
  if (s.prompt) items.push({ key: 'prompt', value: s.prompt })
  if (s.subflowId) items.push({ key: 'subflow', value: s.subflowId })
  const c = s.context
  if (c) {
    if (c.skills?.length) items.push({ key: 'skills', value: c.skills.join(', ') })
    if (c.tools?.length) items.push({ key: 'tools', value: c.tools.join(', ') })
    if (c.files?.length) items.push({ key: 'files', value: c.files.join(', ') })
    if (c.writeScope?.length) items.push({ key: 'write scope', value: c.writeScope.join(', ') })
    if (c.budgetTokens !== undefined) items.push({ key: 'budget tokens', value: String(c.budgetTokens) })
    if (c.cwd) items.push({ key: 'cwd', value: c.cwd })
  }
  if (s.evidenceKind) items.push({ key: 'evidence', value: s.evidenceKind })
  for (const p of s.inputs) items.push({ key: `in · ${p.name}${p.required ? '*' : ''}`, value: p.schemaId })
  for (const p of s.outputs) items.push({ key: `out · ${p.name}`, value: p.schemaId })
  return items
})
</script>

<template>
  <div class="flow-view b10x-canvas">
    <div v-if="loading" class="center"><UiSpinner size="md" /></div>
    <div v-else-if="!swarm" class="center">
      <UiEmptyState title="Swarm not found" :text="`No swarm with id ${props.id}.`">
        <RouterLink to="/">back to swarms</RouterLink>
      </UiEmptyState>
    </div>
    <div v-else-if="!flow" class="center">
      <UiEmptyState title="Flow not found" :text="`Swarm ${swarm.displayName} has no flow ${props.flowId}.`">
        <RouterLink :to="{ name: 'swarm', params: { id: props.id } }">back to {{ swarm.displayName }}</RouterLink>
      </UiEmptyState>
    </div>
    <template v-else>
      <header class="head">
        <RouterLink class="crumb" :to="{ name: 'swarm', params: { id: props.id } }">{{ swarm.displayName }}</RouterLink>
        <span class="sep">/</span>
        <h1 class="title">{{ flow.name }}</h1>
        <template v-if="flow.binds">
          <UiBadge tone="info" :text="`entity ${flow.binds.entity}`" />
          <UiBadge tone="warn" :text="`state ${flow.binds.state}`" />
        </template>
        <span class="spacer" />
        <span class="io" v-if="flow.inputs.length">
          <span class="io-label">in</span>
          <UiBadge v-for="p in flow.inputs" :key="'in:' + p.name" tone="muted" :text="`${p.name} · ${p.schemaId}`" />
        </span>
        <span class="io" v-if="flow.outputs.length">
          <span class="io-label">out</span>
          <UiBadge v-for="p in flow.outputs" :key="'out:' + p.name" tone="ok" :text="`${p.name} · ${p.schemaId}`" />
        </span>
      </header>
      <div class="body">
        <div class="canvas">
          <UiEmptyState v-if="flow.steps.length === 0" title="Empty flow" text="This flow has no steps." />
          <VueFlow
            v-else
            :id="vfId"
            v-model:nodes="nodes"
            v-model:edges="edges"
            :fit-view-on-init="true"
            :min-zoom="0.2"
            :max-zoom="2"
            :nodes-connectable="false"
          >
            <template #node-step="nodeProps">
              <StepNode v-bind="nodeProps" />
            </template>
            <Background variant="dots" :gap="20" :size="1" pattern-color="#2b303b" />
            <Controls position="bottom-left" />
            <MiniMap position="bottom-right" pannable zoomable node-color="#6ea8fe" mask-color="rgba(15,17,21,0.7)" />
          </VueFlow>
        </div>
        <aside class="side">
          <UiPanel :title="selectedStep ? `Step · ${selectedStep.name}` : 'Selected'">
            <UiKeyValue v-if="selectedStep" :items="selectedItems" mono />
            <UiEmptyState v-else title="Nothing selected" text="Click a step to see what it runs and what it may touch." />
          </UiPanel>
        </aside>
      </div>
    </template>
  </div>
</template>

<style scoped>
.flow-view { display: flex; flex-direction: column; flex: 1; min-height: 0; background: var(--color-bg); }
.center { flex: 1; display: flex; align-items: center; justify-content: center; }
.head {
  display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap;
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border); background: var(--color-surface);
}
.crumb { color: var(--color-text-muted); text-decoration: none; }
.crumb:hover { color: var(--color-accent); }
.sep { color: var(--color-text-muted); }
.title { font-size: 16px; font-weight: 600; margin: 0 var(--space-2) 0 0; }
.spacer { flex: 1; }
.io { display: inline-flex; align-items: center; gap: var(--space-1); }
.io-label { color: var(--color-text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: .04em; margin-right: var(--space-1); }
.body { display: flex; flex: 1; min-height: 0; }
.canvas { flex: 1; min-width: 0; min-height: 480px; position: relative; display: flex; align-items: center; justify-content: center; }
.canvas :deep(.vue-flow) { width: 100%; height: 100%; }
.side { width: 320px; flex: none; border-left: 1px solid var(--color-border); overflow: auto; padding: var(--space-3); }
</style>

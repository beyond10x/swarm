<script setup lang="ts">
// The swarm's canvas: agents on the left, blackboxes laid out as a DAG, connections as edges.
// Owned by W2. Reads the Swarm it is given; never writes the store — it emits and W1 decides.
import { computed, ref, shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import { VueFlow, Panel, useVueFlow, type Node, type Edge, type Connection as VfConnection } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'
import '@/components/canvas/canvas-theme.css'
import type { Agent, Box, Port, Swarm } from '@/model'
import { layoutDag } from '@/lib/layout'
import BoxNode, { type BoxNodeData } from '@/components/canvas/BoxNode.vue'
import AgentNode, { type AgentNodeData } from '@/components/canvas/AgentNode.vue'
import { UiPanel, UiBadge, UiKeyValue, UiEmptyState } from '@/components/ui'

const props = defineProps<{ swarm: Swarm }>()
const emit = defineEmits<{
  'update:position': [payload: { boxId: string; position: { x: number; y: number } }]
  connect: [payload: { from: { boxId: string; output: string }; to: { boxId: string; input: string } }]
}>()

const router = useRouter()

// ---- geometry -----------------------------------------------------------------------------------
const BOX_W = 260
const AGENT_W = 220
const AGENT_H = 64
const PORT_ROW = 22
const BOX_HEAD = 36 + 16 // header + ports padding

function boxHeight(b: Box): number {
  return BOX_HEAD + Math.max(b.inputs.length, b.outputs.length, 1) * PORT_ROW
}

// ---- graph state --------------------------------------------------------------------------------
const nodes = shallowRef<Node[]>([])
const edges = shallowRef<Edge[]>([])

/** Rebuild nodes/edges from the swarm. A box's own `position` wins; otherwise an already-placed
 *  node keeps where the user threw it; otherwise dagre decides. */
function rebuild(swarm: Swarm) {
  const previous = new Map<string, { x: number; y: number }>()
  for (const n of nodes.value) previous.set(n.id, { x: n.position.x, y: n.position.y })
  const laid = layoutDag(
    swarm.boxes.map((b) => ({ id: b.boxId, width: BOX_W, height: boxHeight(b) })),
    swarm.connections.map((c) => ({ from: c.from.boxId, to: c.to.boxId })),
    'LR',
  )
  const boxNodes: Node[] = swarm.boxes.map((b) => ({
    id: b.boxId,
    type: 'box',
    position: b.position ?? previous.get(b.boxId) ?? laid.get(b.boxId) ?? { x: 0, y: 0 },
    data: { box: b } satisfies BoxNodeData,
  }))
  const agentNodes: Node[] = swarm.agents.map((a, i) => ({
    id: a.agentId,
    type: 'agent',
    position: previous.get(a.agentId) ?? { x: -(AGENT_W + 120), y: i * (AGENT_H + 24) },
    data: { agent: a } satisfies AgentNodeData,
  }))
  nodes.value = agentNodes.concat(boxNodes)
  edges.value = swarm.connections.map((c) => ({
    id: c.connectionId,
    source: c.from.boxId,
    sourceHandle: c.from.output,
    target: c.to.boxId,
    targetHandle: c.to.input,
    label: c.delivery,
    animated: c.delivery === 'at_least_once',
  }))
}
watch(() => props.swarm, rebuild, { immediate: true, deep: true })

const isEmpty = computed(() => props.swarm.boxes.length === 0 && props.swarm.agents.length === 0)

// ---- Vue Flow store + hooks ---------------------------------------------------------------------
// One store per canvas instance. Not derived from swarmId: the route may swap the swarm while the
// component is reused, and hooks below are bound to this store.
const flowId = `swarm-canvas-${Math.random().toString(36).slice(2, 10)}`
const { onNodeDragStop, onConnect, onNodeDoubleClick, onNodesInitialized, fitView, getSelectedNodes } = useVueFlow({ id: flowId })

onNodesInitialized(() => { fitView({ padding: 0.2 }) })

onNodeDragStop(({ nodes: dragged }) => {
  for (const n of dragged) {
    if (n.type !== 'box') continue
    emit('update:position', { boxId: n.id, position: { x: Math.round(n.position.x), y: Math.round(n.position.y) } })
  }
})

onNodeDoubleClick(({ node }) => {
  if (node.type !== 'box') return
  const box = (node.data as BoxNodeData).box
  if (box.kind === 'flow' && box.flowId) {
    router.push({ name: 'flow', params: { id: props.swarm.swarmId, flowId: box.flowId } })
  }
})

// ---- connecting: schema known to everyone is what makes a refusal possible ----------------------
const toast = ref<string | null>(null)
let toastTimer: ReturnType<typeof setTimeout> | undefined
function refuse(msg: string) {
  toast.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => { toast.value = null }, 2000)
}

function findPort(boxId: string, side: 'inputs' | 'outputs', name: string | null | undefined): { box: Box; port: Port } | undefined {
  const box = props.swarm.boxes.find((b) => b.boxId === boxId)
  if (!box || !name) return undefined
  const port = box[side].find((p) => p.name === name)
  return port ? { box, port } : undefined
}

onConnect((c: VfConnection) => {
  const from = findPort(c.source, 'outputs', c.sourceHandle)
  const to = findPort(c.target, 'inputs', c.targetHandle)
  if (!from || !to) {
    refuse('only typed box ports connect — agents have no schema on their handles')
    return
  }
  if (from.port.schemaId !== to.port.schemaId) {
    refuse(`schema mismatch: ${from.port.schemaId} ≠ ${to.port.schemaId}`)
    return
  }
  emit('connect', {
    from: { boxId: from.box.boxId, output: from.port.name },
    to: { boxId: to.box.boxId, input: to.port.name },
  })
})

// ---- selection panel ----------------------------------------------------------------------------
const selected = computed(() => getSelectedNodes.value[0])

const selectedTitle = computed(() => {
  const n = selected.value
  if (!n) return 'Selected'
  return n.type === 'agent' ? `Agent · ${n.id}` : `Box · ${(n.data as BoxNodeData).box.name}`
})

const selectedItems = computed<{ key: string; value: string }[]>(() => {
  const n = selected.value
  if (!n) return []
  if (n.type === 'agent') {
    const a = (n.data as AgentNodeData).agent
    return agentItems(a)
  }
  return boxItems((n.data as BoxNodeData).box)
})

function boxItems(b: Box): { key: string; value: string }[] {
  const items = [
    { key: 'name', value: b.name },
    { key: 'kind', value: b.kind },
    { key: 'version', value: `v${b.version}` },
    { key: 'owner', value: b.ownerAgentId ?? '—' },
  ]
  if (b.flowId) items.push({ key: 'flow', value: b.flowId })
  if (b.summary) items.push({ key: 'summary', value: b.summary })
  for (const p of b.inputs) items.push({ key: `in · ${p.name}${p.required ? '*' : ''}`, value: p.schemaId })
  for (const p of b.outputs) items.push({ key: `out · ${p.name}`, value: p.schemaId })
  return items
}

function agentItems(a: Agent): { key: string; value: string }[] {
  const host = a.host
  const hostText = host
    ? [host.tmuxWindowId, host.windowIndex !== undefined ? `#${host.windowIndex}` : undefined, host.cwd].filter(Boolean).join(' ')
    : '—'
  const items = [
    { key: 'role', value: a.role },
    { key: 'harness', value: a.harness },
    { key: 'state', value: a.state },
    { key: 'host', value: hostText || '—' },
    { key: 'assignment', value: a.assignment ?? '—' },
  ]
  if (a.blockedOn) items.push({ key: 'blocked on', value: a.blockedOn })
  if (a.lastEvent) items.push({ key: 'last event', value: `${a.lastEvent.kind} · ${a.lastEvent.msg}` })
  if (a.unread) items.push({ key: 'unread', value: String(a.unread) })
  return items
}

function miniMapColor(n: Node): string {
  return n.type === 'agent' ? '#a78bfa' : '#6ea8fe'
}
</script>

<template>
  <div class="swarm-canvas b10x-canvas">
    <div v-if="isEmpty" class="empty">
      <UiEmptyState title="Nothing on the canvas" text="This swarm has no boxes and no agents yet. Boxes appear here as agents build them." />
    </div>
    <template v-else>
      <div class="canvas">
        <VueFlow
          :id="flowId"
          v-model:nodes="nodes"
          v-model:edges="edges"
          :fit-view-on-init="true"
          :min-zoom="0.2"
          :max-zoom="2"
          :nodes-draggable="true"
          :nodes-connectable="true"
          :auto-connect="false"
        >
          <template #node-box="nodeProps">
            <BoxNode v-bind="nodeProps" />
          </template>
          <template #node-agent="nodeProps">
            <AgentNode v-bind="nodeProps" />
          </template>
          <Background variant="dots" :gap="20" :size="1" pattern-color="#2b303b" />
          <Controls position="bottom-left" />
          <MiniMap position="bottom-right" pannable zoomable :node-color="miniMapColor" mask-color="rgba(15,17,21,0.7)" />
          <Panel v-if="toast" position="top-center" class="toast">
            <UiBadge tone="fault" :text="toast" />
          </Panel>
        </VueFlow>
      </div>
      <aside class="side">
        <UiPanel :title="selectedTitle">
          <UiKeyValue v-if="selected" :items="selectedItems" mono />
          <UiEmptyState v-else title="Nothing selected" text="Click a box or an agent. Double-click a flow box to open its DAG. Drag a handle onto a port to connect." />
        </UiPanel>
      </aside>
    </template>
  </div>
</template>

<style scoped>
.swarm-canvas { display: flex; flex: 1; min-height: 0; height: 100%; background: var(--color-bg); }
.canvas { flex: 1; min-width: 0; min-height: 480px; position: relative; }
.canvas :deep(.vue-flow) { width: 100%; height: 100%; }
.side { width: 320px; flex: none; border-left: 1px solid var(--color-border); overflow: auto; padding: var(--space-3); }
.empty { flex: 1; display: flex; align-items: center; justify-content: center; }
.toast :deep(*) { font-size: 12px; }
</style>

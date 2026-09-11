<script setup lang="ts">
// The canvas: every instance the swarm holds, laid out and drawn.
//
// Node types are registered from data rather than written as template slots, so an entity added to
// `src/core` appears here the moment the server serves it. `markRaw` matters — Vue Flow keeps the
// registry in its store, and a reactive component definition is both a warning and a waste.
//
// Edges come from `swarm.blackbox.Connection` instances, which is the specification's own answer to
// what may reach what: a connection between two boxes is the edge, and drawing one is how a swarm
// grants itself reach.
import { computed, markRaw } from 'vue'
import { VueFlow, type Edge, type Node, type NodeTypesObject } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import InstanceNode, { type InstanceNodeData } from '@/components/canvas/InstanceNode.vue'
import { layoutDag } from '@/lib/layout'
import type { Canvas, Instance, Shape } from '@/runtime'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'
import '@/components/canvas/canvas-theme.css'

const props = defineProps<{ canvas: Canvas; shape?: Shape; flowId?: string }>()

const NODE_W = 220
const NODE_H = 130

/**
 * One component for every entity.
 *
 * A map rather than a single default, because this is the seam where an entity that deserves its
 * own drawing gets one — a Ui box rendering an actual control, say — without the rest changing.
 */
const nodeTypes = markRaw({ instance: InstanceNode }) as unknown as NodeTypesObject

const CONNECTION = 'swarm.blackbox.Connection'

/** Terminal states per entity, read from the specification rather than guessed from a name. */
const terminal = computed(
  () => new Map((props.shape?.entities ?? []).map((entity) => [entity.name, entity.terminal])),
)

/** Every instance on the canvas, whatever entity it belongs to. */
const instances = computed<Instance[]>(() =>
  Object.entries(props.canvas)
    .filter(([entity]) => entity !== CONNECTION)
    .flatMap(([, held]) => held),
)

/** Connections are edges, not nodes: they are the wire between two boxes. */
const connections = computed<Instance[]>(() => props.canvas[CONNECTION] ?? [])

const nodes = computed<Node[]>(() => {
  const laid = layoutDag(
    instances.value.map((instance) => ({
      id: instance.id,
      width: NODE_W,
      height: NODE_H,
    })),
    connections.value.flatMap((connection) => {
      const from = connection.fields.from_box
      const to = connection.fields.to_box
      return typeof from === 'string' && typeof to === 'string' ? [{ from, to }] : []
    }),
    'LR',
  )

  return instances.value.map((instance, index) => ({
    id: instance.id,
    type: 'instance',
    position: laid.get(instance.id) ?? { x: 0, y: index * (NODE_H + 32) },
    data: {
      instance,
      terminal: terminal.value.get(instance.entity),
    } satisfies InstanceNodeData,
  }))
})

const edges = computed<Edge[]>(() =>
  connections.value.flatMap((connection) => {
    const from = connection.fields.from_box
    const to = connection.fields.to_box
    if (typeof from !== 'string' || typeof to !== 'string') return []
    return [
      {
        id: connection.id,
        source: from,
        target: to,
        animated: connection.state === 'Live',
        label: String(connection.fields.schema_id ?? ''),
      },
    ]
  }),
)
</script>

<template>
  <div class="canvas">
    <VueFlow
      :id="props.flowId ?? 'swarm'"
      :nodes="nodes"
      :edges="edges"
      :node-types="nodeTypes"
      fit-view-on-init
      :min-zoom="0.2"
      :max-zoom="2"
    >
      <Background pattern-color="#2a3240" :gap="20" />
      <Controls />
      <MiniMap pannable zoomable />
    </VueFlow>

    <p v-if="!instances.length" class="empty">
      Nothing here yet. A swarm starts bare — give it a goal and start it, and what it becomes it
      builds.
    </p>
  </div>
</template>

<style scoped>
.canvas {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 24rem;
}

.empty {
  position: absolute;
  inset: 0;
  display: grid;
  place-content: center;
  text-align: center;
  max-width: 26rem;
  margin: auto;
  opacity: 0.55;
  pointer-events: none;
}
</style>

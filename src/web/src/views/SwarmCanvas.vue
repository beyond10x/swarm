<script setup lang="ts">
// The canvas: every instance the swarm holds, laid out and drawn.
//
// Node types are registered from data rather than written as template slots, so an entity added to
// `src/core` appears here the moment the server serves it. `markRaw` matters — Vue Flow keeps the
// registry in its store, and a reactive component definition is both a warning and a waste.
//
// Edges are the specification's, from two sources and no others. A declared RELATION is an edge —
// `swarm.goal.Goal references swarm.manager.Swarm via swarm_id` is already written down, so a goal
// is drawn joined to the swarm it belongs to without anybody saying so twice. A
// `swarm.blackbox.Connection` is the other: a wire one box drew to another, which is how a swarm
// grants itself reach.
import { computed, markRaw, ref, watch, type Ref } from 'vue'
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

const props = defineProps<{
  canvas: Canvas
  shape?: Shape
  flowId?: string
  /** Instances that changed a moment ago, drawn lit so a change is seen rather than found. */
  recent?: Set<string>
}>()

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

/**
 * The nodes, held rather than computed.
 *
 * A computed list would be rebuilt on every change the stream carries, and each rebuild would be a
 * new set of node objects — which is a canvas that forgets where a person dragged a node and drops
 * whatever was selected, several times a minute. So the list is kept, `v-model`ed to Vue Flow so
 * drag and selection land in it, and reconciled: an instance already drawn has its data replaced in
 * place, a new one is laid out and added, a gone one is removed. Positions survive everything but
 * removal.
 */
// Cast rather than `ref<Node[]>`: Vue Flow's `Node` is generic enough that TypeScript gives up
// unwrapping it (TS2589), and the cast is the documented way round.
const nodes = ref([]) as Ref<Node[]>

function reconcile(): void {
  const wanted = instances.value
  const ids = new Set(wanted.map((instance) => instance.id))
  const held = new Map(nodes.value.map((node) => [node.id, node]))

  const fresh = wanted.filter((instance) => !held.has(instance.id))
  // Only newcomers are laid out, and among the others so they land near what they relate to.
  const laid = fresh.length
    ? layoutDag(
        wanted.map((instance) => ({ id: instance.id, width: NODE_W, height: NODE_H })),
        edges.value.flatMap((edge) =>
          typeof edge.source === 'string' && typeof edge.target === 'string'
            ? [{ from: edge.source, to: edge.target }]
            : [],
        ),
        'LR',
      )
    : undefined

  const next: Node[] = []
  wanted.forEach((instance, index) => {
    const data: InstanceNodeData = {
      instance,
      terminal: terminal.value.get(instance.entity),
      changed: props.recent?.has(instance.id) ?? false,
    }
    const existing = held.get(instance.id)
    if (existing) {
      existing.data = data
      next.push(existing)
    } else {
      next.push({
        id: instance.id,
        type: 'instance',
        position: laid?.get(instance.id) ?? { x: 0, y: index * (NODE_H + 32) },
        data,
      })
    }
  })
  // Same length and same members means nothing to replace, and replacing anyway would be the
  // rebuild this function exists to avoid.
  if (next.length !== nodes.value.length || next.some((node, at) => node !== nodes.value[at])) {
    nodes.value = next
  }
  void ids
}

/** Every instance by id, so a relation's target can be found. */
const byId = computed(() => new Map(instances.value.map((instance) => [instance.id, instance])))

/** The relations the specification declares, per entity. */
const relations = computed(
  () => new Map((props.shape?.entities ?? []).map((entity) => [entity.name, entity.relations])),
)

/** Edges from declared relations: a goal to its swarm, an agent to its swarm, a config to its own. */
const related = computed<Edge[]>(() =>
  instances.value.flatMap((instance) =>
    (relations.value.get(instance.entity) ?? []).flatMap((relation) => {
      const target = instance.fields[relation.via]
      // A relation whose field nothing has written is not an edge that is missing; it is an edge
      // that does not exist yet, and drawing it would be inventing a connection.
      if (typeof target !== 'string' || !byId.value.has(target)) return []
      return [
        {
          id: `${instance.id}:${relation.name}`,
          source: target,
          target: instance.id,
          label: relation.name,
          // An owned thing is joined more firmly than a referenced one, and the specification is
          // where that difference is declared.
          style: relation.owns ? undefined : { strokeDasharray: '4 4' },
          class: 'relation-edge',
        },
      ]
    }),
  ),
)

/** Edges from connections a swarm drew for itself. */
const wired = computed<Edge[]>(() =>
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

const edges = computed<Edge[]>(() => [...related.value, ...wired.value])

watch([instances, edges, () => props.recent], reconcile, { immediate: true, deep: false })
</script>

<template>
  <div class="canvas b10x-canvas">
    <VueFlow
      :id="props.flowId ?? 'swarm'"
      v-model:nodes="nodes"
      :edges="edges"
      :node-types="nodeTypes"
      fit-view-on-init
      :min-zoom="0.2"
      :max-zoom="2"
    >
      <Background pattern-color="#2a3240" :gap="20" />
      <Controls />
      <MiniMap
        pannable
        zoomable
        mask-color="rgba(15, 17, 21, 0.72)"
        node-color="#2b303b"
        node-stroke-color="#9aa3b2"
        :node-stroke-width="2"
      />
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

// DAG layout over dagre. Pure: takes sizes and edges, returns top-left positions keyed by id.
// Owned by W2. No Vue, no DOM — unit-testable as a plain function.
// Default import on purpose: dagre is CommonJS and its named exports are not statically visible to
// Node's lexer; the default (module.exports) works under Node, Vite dev and Vite build alike.
import dagre from 'dagre'

export interface LayoutNode {
  id: string
  width: number
  height: number
}

export interface LayoutEdge {
  from: string
  to: string
}

export type LayoutDirection = 'LR' | 'TB'

export interface LayoutOptions {
  /** Separation between nodes in the same rank. Default 40. */
  nodesep?: number
  /** Separation between ranks. Default 80. */
  ranksep?: number
}

/**
 * Lay out a DAG. Edges whose endpoints are not in `nodes` are ignored (dagre would otherwise
 * invent an unsized node for them and throw). Returned positions are the node's top-left corner,
 * which is what Vue Flow expects; dagre itself reports centres.
 */
export function layoutDag(
  nodes: LayoutNode[],
  edges: LayoutEdge[],
  direction: LayoutDirection = 'LR',
  options: LayoutOptions = {},
): Map<string, { x: number; y: number }> {
  const out = new Map<string, { x: number; y: number }>()
  if (nodes.length === 0) return out

  const g = new dagre.graphlib.Graph()
  g.setGraph({ rankdir: direction, nodesep: options.nodesep ?? 40, ranksep: options.ranksep ?? 80 })
  g.setDefaultEdgeLabel(() => ({}))

  const known = new Set<string>()
  for (const n of nodes) {
    known.add(n.id)
    g.setNode(n.id, { width: n.width, height: n.height })
  }
  for (const e of edges) {
    if (e.from === e.to) continue
    if (!known.has(e.from) || !known.has(e.to)) continue
    g.setEdge(e.from, e.to)
  }

  dagre.layout(g)

  for (const n of nodes) {
    const placed = g.node(n.id)
    if (!placed) continue
    out.set(n.id, {
      x: Math.round(placed.x - n.width / 2),
      y: Math.round(placed.y - n.height / 2),
    })
  }
  return out
}

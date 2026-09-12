---
format: aep.planning-md/1
id: story:render-a-ui-box
kind: story
status: active
title: Render a UI box
summary: 'A Box{kind: Ui} renders a real component on the canvas.'
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/core/domains/blackbox.yaml
- confidence: inferred
  path: src/web/src/components/canvas/UiBox.vue
- confidence: inferred
  path: src/web/src/components/ui/index.ts
- confidence: inferred
  path: src/web/src/runtime.ts
- confidence: inferred
  path: src/web/src/stores/swarms.ts
- confidence: cited
  path: src/web/src/views/SwarmCanvas.vue
revision: 11
---
## What

`BoxKind` declares `Ui` and nothing renders one. The canvas draws every entity through one generic
node, which is right for a Goal and wrong for a panel that should show a table.

Give a `Box{kind: Ui}` a component and props, and render it on the canvas through the existing
component library.

## Acceptance

A `Box{kind: Ui}` naming a component from `src/web/src/components/ui` renders that component on the
swarm canvas with the props the box carries, and a table box fed from a view shows that view's rows
and updates when the view does.

## Out of scope

A layout language, and generating the whole application from the specification. A panel on the
canvas first.

## Notes

`components/ui/index.ts` is a contract listing 21 components with their props. `SwarmCanvas.vue`
already registers node types from data, which is the seam this needs.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/web` — cited; the acceptance is entirely about what the canvas draws.
- **Files (cited):** `src/web/src/views/SwarmCanvas.vue` — the `nodeTypes` registry and the node-data build, named by the story as the seam.
- **Files (inferred):** a new `src/web/src/components/canvas/UiBox.vue` (collides with nothing), `src/web/src/components/ui/index.ts` (rendering by name needs a name-to-component map beside the named re-exports), `src/web/src/runtime.ts` and `stores/swarms.ts` (a table fed from a view needs a per-view fetch the store does not hold today), `src/core/domains/blackbox.yaml`.
- **Confidence:** medium. The rendering seam and the component contract are read; the props carrier and the view-refresh path are not declared anywhere.
- **Would collide with:** any unit touching the canvas node registry, `components/ui/index.ts`, or — if a props field is needed — `swarm.blackbox.Box`.
- **Not established:** where a UI box's props are stored. `Box` has `ref_id` for the component name and no props field; nothing in `src/core` matches `props`.

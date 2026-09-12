---
format: aep.planning-md/1
id: story:render-a-ui-box
kind: story
status: implemented
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
revision: 13
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

Rewritten 2026-09-12 from the implementor's confirmation, after the work landed.

**Landed on, all confirmed by building:**

- `src/web/src/views/SwarmCanvas.vue` — the node-type registry, as the scope said.
- `src/web/src/components/canvas/UiBox.vue` and `src/web/src/lib/uibox.ts` — new; the node and the decode.
- `src/web/src/components/ui/index.ts` — the contract's comment table is now also data: `uiPropTypes`, `uiEmitters` and `uiRefused` are parsed from it, and a drift test fails when the halves disagree.
- `src/core/domains/blackbox.yaml` — `Box.props` as `Optional<Map<String, String>>`, threaded through `DraftBox`, the emitted event, `sets:` and the `Canvas` view. `ref_id` carries the component name, which it was already documented to do.
- `src/web/src/stores/swarms.ts` — per-view rows held beside the error that produced them.
- Four test files, two of them the adversaries'.

**Guessed and wrong:**

- `src/web/src/runtime.ts` — **not touched.** The scope expected a per-view fetch the store did not hold. `runtime.view(slug, name)` already existed; only the store needed it.

**Beyond the scope list:** `src/web/src/views/SwarmView.vue` gained one attribute, `:slug`. A canvas cannot read a view without knowing which swarm it is, and inferring that from the flow id would have been a guess. `package.json` and `tsconfig.json` gained a test lane on node's own runner, with no new dependency.

**What the work cost beyond the story.** Two adversary passes, six findings. The first correction replaced a JSON-shape heuristic — which turned a badge reading `404` into the number 404 and dropped a required prop whose value was `null` — with a lookup against the type the contract declares. The second pass then found that `:inert` does not contain a `Teleport`: a box naming `UiModal` rendered an unclosable full-screen dialog that scroll-locked the page and swallowed Tab and Escape. Containment is now a refusal rather than an attribute, and which components must be refused is machine-derived by scanning all 21 for markup or listeners that escape their own subtree. `UiModal` is the only one today.

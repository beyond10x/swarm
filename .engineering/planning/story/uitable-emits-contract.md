---
format: aep.planning-md/1
id: story:uitable-emits-contract
kind: story
status: draft
title: UiTable emits what the contract does not declare
summary: The component library's contract table disagrees with UiTable, and correcting it changes canvas behaviour.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
revision: 1
---
## What

`src/web/src/components/ui/index.ts` is a contract: a comment table declaring every component's
props, slots and emits, and the 21 named re-exports beside it. `UiTable` emits `row-click` and the
table does not say so.

Found 2026-09-12 by unit B of that day's wave, which needed the emits column as data: it renders a
component that emits inside `:inert`, because a canvas panel cannot keep what a component emits.
`UiTable` is therefore treated as non-emitting, which happens to be true today only because
`<component :is>` binds no listeners at all.

## Why it was not fixed in that wave

Correcting the table is not a documentation change. The drift test parses the table, and a
`UiTable` row gaining `emits row-click` would put it in the emitter set and make every table panel
inert — a behaviour change nobody asked for, in the middle of a story whose acceptance is that a
table box shows a view's rows.

## Acceptance

The contract table and `UiTable.vue` agree about what it emits, and whatever that agreement implies
for the canvas is decided on purpose rather than inherited: either a table panel is inert like every
other emitter, or the emitter rule distinguishes an emit that a panel could act on from one it
cannot.

## Notes

The drift test that found this is the useful part and should stay: it parses the table and fails
when the lookup and the re-exports disagree. It is the reason this was discovered rather than
assumed.

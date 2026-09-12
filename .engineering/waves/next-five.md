# The next five waves — computed 2026-09-12, after the bug wave closed

`aep plan artifact waves --kind story --status draft --format json`, ten drafts, **0 unassessed,
0 cycles, 32 collisions**. Every draft carries typed scope entries, so this is a computed selection
and not a pairwise reading.

| wave | stories | why it stops there |
|---|---|---|
| 1 | `connectors-as-service-boxes`, `escapes-scan-fails-open` | — |
| 2 | `coordinator-from-config`, `inert-costs-a-table-its-readers` | — |
| 3 | `honour-view-consistency`, `run-a-tool-box`, `uitable-without-rows` | the only wave of three the backlog admits |
| 4 | `slug-is-not-validated` | `state.rs` and `http.rs`, which four other stories touch |
| 5 | `spawn-a-second-agent` | `coordinator.rs` cited against `run-a-tool-box` and `coordinator-from-config` |

Ten stories in five waves, against seven in five before this wave ran, because the bug wave's
adversaries filed five and closed three.

**`connectors-as-service-boxes` is still not implementable in this tree.** It implies `reqwest` and
a tokio client in a workspace that has neither, and no config surface exists for an adapter endpoint
or token. It is wave 1 by surface and it is not wave 1 by readiness. Whoever runs wave 1 should
either pair `escapes-scan-fails-open` with something else or run it alone.

**Three of the ten are the bug wave's own findings** — `slug-is-not-validated`,
`uitable-without-rows`, `escapes-scan-fails-open` — and two more, `inert-costs-a-table-its-readers`
and the `RETRY_DELAY` bound, are decisions rather than defects. A wave that attacks its own work
produces roughly as much backlog as it closes; this one closed 3 and filed 5.

### What this selection rests on

Six files still couple almost everything, unchanged from the 2026-09-12 wave's finding:
`src/web/src/runtime.ts`, `src/runtime/swarm-server/src/state.rs`, `swarm.rs`, `trigger.rs`,
`coordinator.rs`, `src/core/domains/blackbox.yaml`. Two more joined them this wave:
`src/web/src/components/ui/index.ts` and `src/web/src/components/canvas/UiBox.vue`, which three of
the five new stories touch.

The cap on a wave here is that coupling, not the disk and not the model budget.

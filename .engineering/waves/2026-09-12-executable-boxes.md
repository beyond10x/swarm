# Wave proposal — 2026-09-12 — executable boxes

Coordinator: this session. Skill `aep-drive:wave` **0.9.1**. `aep` **0.55.0**.
Base branch `main`, at `7cdace9`.

**Status: proposed. Nothing has been dispatched and no worktree exists.**

---

## The request, and what the store answered

The request was five stories worked at once. **The store says the largest disjoint set is two.**

Seven candidates were drafted, scoped by seven `aep-drive:story-scoper` agents, and their surfaces
written back as typed entries. `aep plan artifact waves --kind story --status draft` then returned
**five waves, thirty-eight collisions, nothing unassessed and no cycles** — five waves meaning five
*rounds*, not one wave of five.

The selection path was the verb, not a pairwise reading.

### The waves, verbatim

| wave | stories |
|---|---|
| 1 | `story:connectors-as-service-boxes` |
| 2 | `story:coordinator-from-config` |
| 3 | `story:honour-view-consistency`, `story:run-a-tool-box` |
| 4 | `story:redelivery-for-at-least-once`, `story:render-a-ui-box` |
| 5 | `story:spawn-a-second-agent` |

### Unassessed

None. All seven carry typed scope entries.

### Cycles

None.

### Collisions — all thirty-eight

```
connectors-as-service-boxes  × coordinator-from-config       src/runtime/swarm-server/src/state.rs    inferred
connectors-as-service-boxes  × coordinator-from-config       src/web/src/runtime.ts                   inferred
connectors-as-service-boxes  × honour-view-consistency       src/runtime/swarm-server/src/state.rs    inferred
connectors-as-service-boxes  × honour-view-consistency       src/web/src/runtime.ts                   inferred
connectors-as-service-boxes  × redelivery-for-at-least-once  src/runtime/swarm-server/src/state.rs    inferred
connectors-as-service-boxes  × render-a-ui-box               src/core/domains/blackbox.yaml           inferred
connectors-as-service-boxes  × render-a-ui-box               src/web/src/runtime.ts                   inferred
connectors-as-service-boxes  × run-a-tool-box                src/core/domains/blackbox.yaml           CITED
connectors-as-service-boxes  × spawn-a-second-agent          src/runtime/swarm-server/src/state.rs    inferred
connectors-as-service-boxes  × spawn-a-second-agent          src/web/src/runtime.ts                   inferred
coordinator-from-config      × honour-view-consistency       src/runtime/swarm-server/src/state.rs    CITED
coordinator-from-config      × honour-view-consistency       src/web/src/runtime.ts                   inferred
coordinator-from-config      × redelivery-for-at-least-once  src/runtime/swarm-server/src/state.rs    inferred
coordinator-from-config      × redelivery-for-at-least-once  src/runtime/swarm-server/src/swarm.rs    inferred
coordinator-from-config      × redelivery-for-at-least-once  src/runtime/swarm-server/src/trigger.rs  inferred
coordinator-from-config      × render-a-ui-box               src/web/src/runtime.ts                   inferred
coordinator-from-config      × run-a-tool-box                src/runtime/swarm-server/src/coordinator.rs  CITED
coordinator-from-config      × run-a-tool-box                src/runtime/swarm-server/src/swarm.rs    inferred
coordinator-from-config      × run-a-tool-box                src/runtime/swarm-server/src/trigger.rs  inferred
coordinator-from-config      × spawn-a-second-agent          src/runtime/swarm-server/src/coordinator.rs  CITED
coordinator-from-config      × spawn-a-second-agent          src/runtime/swarm-server/src/state.rs    inferred
coordinator-from-config      × spawn-a-second-agent          src/runtime/swarm-server/src/swarm.rs    inferred
coordinator-from-config      × spawn-a-second-agent          src/runtime/swarm-server/src/trigger.rs  inferred
coordinator-from-config      × spawn-a-second-agent          src/web/src/runtime.ts                   inferred
honour-view-consistency      × redelivery-for-at-least-once  src/runtime/swarm-server/src/state.rs    inferred
honour-view-consistency      × render-a-ui-box               src/web/src/runtime.ts                   inferred
honour-view-consistency      × spawn-a-second-agent          src/runtime/swarm-server/src/state.rs    inferred
honour-view-consistency      × spawn-a-second-agent          src/web/src/runtime.ts                   inferred
redelivery-for-at-least-once × run-a-tool-box                src/runtime/swarm-server/src/swarm.rs    inferred
redelivery-for-at-least-once × run-a-tool-box                src/runtime/swarm-server/src/trigger.rs  inferred
redelivery-for-at-least-once × spawn-a-second-agent          src/runtime/swarm-server/src/state.rs    inferred
redelivery-for-at-least-once × spawn-a-second-agent          src/runtime/swarm-server/src/swarm.rs    CITED
redelivery-for-at-least-once × spawn-a-second-agent          src/runtime/swarm-server/src/trigger.rs  CITED
render-a-ui-box              × run-a-tool-box                src/core/domains/blackbox.yaml           inferred
render-a-ui-box              × spawn-a-second-agent          src/web/src/runtime.ts                   inferred
run-a-tool-box               × spawn-a-second-agent          src/runtime/swarm-server/src/coordinator.rs  CITED
run-a-tool-box               × spawn-a-second-agent          src/runtime/swarm-server/src/swarm.rs    inferred
run-a-tool-box               × spawn-a-second-agent          src/runtime/swarm-server/src/trigger.rs  inferred
```

### Why five is not available, in one table

Six files are the hub of everything, which is the finding rather than the obstacle:

| file | stories that touch it |
|---|---|
| `src/web/src/runtime.ts` | 5 |
| `src/runtime/swarm-server/src/state.rs` | 5 |
| `src/runtime/swarm-server/src/swarm.rs` | 4 |
| `src/runtime/swarm-server/src/trigger.rs` | 4 |
| `src/runtime/swarm-server/src/coordinator.rs` | 3 |
| `src/core/domains/blackbox.yaml` | 3 |

`state.rs` builds the `/spec` and `/status` payloads, `runtime.ts` is their typed mirror, and
almost any change to the runtime shows up in both. That coupling is real and it is what caps a
wave at two here.

---

## Proposed: wave 4, N = 2

| unit | story | surface | build |
|---|---|---|---|
| A | `story:redelivery-for-at-least-once` | Rust, `swarm-server` + `ess-runtime` | cargo |
| B | `story:render-a-ui-box` | Vue, `src/web` only | npm |

Both serve `vision:swarm-builds-itself` through `epic:executable-boxes`.

**Chosen over wave 3 on the disk numbers below.** Wave 3 is two Rust units and two build
directories; wave 4 is one of each, and one of its units needs no compiler at all.

Unit A's scope is **cited** on its three load-bearing files. Unit B's is **cited** on one file and
inferred on the rest, which is the weaker half of this wave and is stated rather than smoothed
over: where a UI box's props live is not declared anywhere in the specification today.

### Left out, and why

| story | why not in this wave |
|---|---|
| `run-a-tool-box` | collides with A on `swarm.rs` and `trigger.rs`. It is the story the operator asked for first ("OS notification, other CLI commands") and it is wave 3 |
| `honour-view-consistency` | collides with A on `state.rs` |
| `coordinator-from-config` | collides with A on three files |
| `spawn-a-second-agent` | collides with A on four files, two of them cited |
| `connectors-as-service-boxes` | collides on `state.rs` and `runtime.ts`, and it is not implementable tonight: it implies `reqwest` and a tokio runtime in a workspace that deliberately has neither client-side, and no config surface exists for an adapter endpoint or token |

---

## Pre-flight

| check | result |
|---|---|
| working tree | clean on `main`; `.engineering/` is the new store, untracked, and goes in the opening commit |
| worktrees | one — the main checkout. No previous wave's trees left standing |
| **free disk** | **20G free of 848G. The disk is 98% full.** |
| one worktree's build cost | **4.6G** measured on this tree: 4.2G debug, 459M release |
| web unit's cost | 82M of `node_modules` |
| this wave's cost | **~4.7G**, leaving ~15G |
| a wave of five | **~23G of build directories against 20G free. Refused on disk before any other consideration.** |
| build cache | `sccache` is installed and `RUSTC_WRAPPER` is **unset**. 3.1G of cache, 305 hits from 1156 executed compiles. Setting it before dispatch is worth doing |
| repository agent file | **none.** No `AGENTS.md`, no `CLAUDE.md`, no `.agents/` in this repository, so this skill's defaults are unopposed and there is no repository-specific gate to obey |
| model budget | **not established — this is a question for the operator** |

The disk number is the one that matters. Even if the coupling were fixed, five Rust worktrees do
not fit on this machine today.

---

## Units

Filled in by stage 2 as each is created. Nothing exists yet.

| unit | branch | head | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| A | `wave/2026-09-12/redelivery` | — | — | — | — | not started |
| B | `wave/2026-09-12/ui-box` | — | — | — | — | not started |

Integration branch: `wave/2026-09-12/integration`, forked from `main`.

Agent types: `aep-drive:implementor` per unit, then `aep-drive:adversary` against the same
worktree.

---

## The commits approval authorises

Approving this wave authorises exactly these and nothing else:

1. the opening commit — this page, the store, and the two stories moved out of `draft`;
2. one commit per unit, on its own branch;
3. two merges into `wave/2026-09-12/integration`;
4. the closing store commit — evidence, scope rewritten from each implementor's confirmation,
   and each story moved to its terminal status;
5. the merge of the integration branch into `main` once the whole gate is green on it.

**Not** a push, **not** a tag, **not** a release, **not** the next wave. The standing rule that
nothing is committed unasked resumes when this wave closes.

---

## What the scoping already paid for

Two of the seven stories were wrong when drafted, and the scopers disproved both before anybody
implemented anything. Both corrections are in the store.

- `redelivery-for-at-least-once` claimed the work belonged beside the trigger because the pump has
  no clock or store. The trigger never sees those bindings: all three are event-caused and
  `Server::periods()` filters on periodic causes, so the failure arises only on the `issue()` path.
  The defect is one missing `else` in `swarm.rs`.
- `honour-view-consistency` claimed nine `eventual` views. There are six.

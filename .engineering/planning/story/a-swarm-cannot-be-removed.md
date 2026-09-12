---
format: aep.planning-md/1
id: story:a-swarm-cannot-be-removed
kind: story
status: draft
title: Nothing can remove a swarm
summary: There is no DELETE route, and a place with no swarm in it is unreachable by every command.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
- confidence: cited
  path: src/runtime/swarm-server/src/state.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/serves_a_swarm.rs
- confidence: cited
  path: src/web/src/stores/swarms.ts
- confidence: cited
  path: src/web/src/views/SwarmView.vue
revision: 3
---
## What

`GET /swarms` lists a swarm and nothing can remove one. `src/runtime/swarm-server/src/http.rs:112-123`
declares 12 routes with `get` and `post` only — there is no `DELETE` verb anywhere in the server, and
the only `remove_file` in the runtime is a coordinator record (`coordinator.rs:572`). Nothing ever
unlinks `data/swarms/<slug>`.

Two consequences, both observed 2026-09-12:

- **A place with no swarm in it cannot be removed.** `POST /swarms` creates a directory and says so —
  *"This creates a PLACE for a swarm, not a swarm"* (`http.rs:133-136`). If `CreateSwarm` never
  follows, there is no `swarm.manager.Swarm` instance, so the UI's `canAct` reads `record?.state` as
  `undefined` and disables all five actions (`src/web/src/stores/swarms.ts:468-477`) — with no reason
  shown. The operator sees a badge reading `Uncreated`, a word the specification does not contain
  (`SwarmView.vue:126`), and five grey buttons. Swarm `123` was in this state and was removed by hand.
- **`DeleteSwarm` does not do what it says.** `manager.yaml:326` promises the swarm *"no longer
  appears in the swarms list"*. `GET /swarms` is `Server::slugs()`, the keys of the in-memory handle
  map built by scanning the directory at boot (`state.rs:63-77, 156-158`). Six swarms are in state
  `Deleted` right now and all six still appear.

## Acceptance

`DELETE /swarms/{slug}` removes a swarm that is absent from the model or in the terminal `Deleted`
state, and refuses anything else with `SwarmStateConflict`. The list stops showing terminal swarms.
A disabled action in the UI says why it is disabled.

## Out of scope

Whether erasure should be a recorded act in the model. `manager.yaml:87-89` says *"nothing here is
deleted"* on purpose, and re-deciding that is its own piece of work — not least because an erasure
event written into the log being erased is self-refuting, so the record would have to live in a log
that outlives it.

## Notes

Three things the implementation must get right, all of them already documented hazards in this file:

1. Take the **same per-slug `opening` gate** `Server::open` uses (`state.rs:124-130`), so no
   concurrent open reconstructs the handle mid-removal.
2. Remove the slug from the `swarms` map and drop the `Arc` — closing the SQLite handle — **before**
   removing the directory. There is no eviction path today: `.remove()` is never called on `swarms`,
   only on `capped` (`state.rs:197, 225`).
3. Remove the three files together. Deleting `eventlog.sqlite3` while leaving `-wal` behind makes the
   next open attempt WAL recovery against a fresh empty database.

## Scope

- **Files:** `src/runtime/swarm-server/src/http.rs` (the route table and a handler) — cited
- **Files:** `src/runtime/swarm-server/src/state.rs` (eviction, the `opening` gate, `slugs()`) — cited
- **Files:** `src/web/src/stores/swarms.ts` (`canAct`, `visible`) and `src/web/src/views/SwarmView.vue` (the reason) — cited
- **Also likely:** `src/runtime/swarm-server/tests/serves_a_swarm.rs` — inferred
- **Confidence:** high — every path was read and the missing verb confirmed by enumerating the route table
- **Would collide with:** any unit touching `http.rs` or `state.rs`

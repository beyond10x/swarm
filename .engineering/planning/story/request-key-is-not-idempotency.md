---
format: aep.planning-md/1
id: story:request-key-is-not-idempotency
kind: story
status: active
title: A request key does not make a retry idempotent
summary: The documented idempotency key guards the append, not the application.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
scope:
- confidence: inferred
  path: src/runtime/ess-runtime/src/store.rs
- confidence: cited
  path: src/runtime/swarm-server/src/http.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/redelivery_under_attack.rs
- confidence: inferred
  path: src/web/src/runtime.ts
revision: 6
---
## What

`http.rs` documents the request key as "an idempotency key. Retrying with the same one is a retry,
not a second request." It is not. It is an APPEND guard, scoped by `store.rs` to the instance's
stream, and it does nothing about the command being applied a second time.

Measured 2026-09-12 by an adversary against unit A of the 2026-09-12 wave: two `issue()` calls with
the same key and the same `ActivateConfig` input answer `Ok("activated")` then `Ok("wrong-state")`.
A client whose connection dropped and retried is told its command was in the wrong state, which is
the opposite of what the field promises.

Worse for a creating command: the key is scoped to the instance's stream, and a creating command
mints a fresh instance per attempt, so the key is never spent at all. Two same-key `DraftConfig`
calls leave two Configs.

## Acceptance

A command issued twice under one request key produces one effect and answers the same thing both
times, for a moving command and for a creating one alike — or the field's documentation says
plainly that it guards the append and not the application, and the HTTP surface offers whatever a
disconnected client actually needs.

## Out of scope

Distributed exactly-once. This is one process and one SQLite file.

## Notes

Reached through `issue_command`, which takes the key from the client body verbatim. The
`Delivery::request` field added by the redelivery unit rests on the same misreading and is being
corrected there; this story is the surface underneath it.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/runtime/swarm-server` — cited, the story names `http.rs` and reaches the defect through `issue_command`
- **Files:** `src/runtime/swarm-server/src/http.rs:39` — cited, the `Issue::request` doc the story quotes verbatim
- **Files:** `src/runtime/swarm-server/src/http.rs:134` — cited, `issue_command` takes the key from the body verbatim
- **Files:** `src/runtime/swarm-server/src/swarm.rs:653` — cited, `Swarm::issue` applies the command before the append, which is where a retry is re-applied
- **Files:** `src/runtime/swarm-server/tests/redelivery_under_attack.rs` — cited, both tests name this story and assert today's behaviour, so a fix turns them red
- **Symbols:** `Issue::request`, `Swarm::issue`, `Swarm::delivery_key`, `Delivery::request` — cited
- **Also likely:** `src/runtime/ess-runtime/src/store.rs:136` — inferred, `Store::commit`'s doc is the accurate account of the append guard
- **Also likely:** `src/web/src/runtime.ts:271` — inferred, the TS client repeats the same wrong promise
- **Also likely:** `src/runtime/swarm-server/src/swarm.rs:324` — inferred, `Delivery::request`'s doc would be restated by whatever this story decides
- **Documents:** none — cited, no `.md`, `.mdx` or `website/` page mentions the request key
- **Confidence:** high — the story names the file, the field and the two measured failures, and each was read in the tree
- **Would collide with:** any unit touching `swarm-server`'s HTTP surface or `Swarm::issue`/the redelivery queue in `swarm.rs`

Not established: which acceptance branch wins. "Make it idempotent" is code in `Swarm::issue` plus a
request-to-answer record; "say it guards the append" is doc across four files and would not touch
the store. A durable request-to-answer table would widen into `ess-runtime`'s schema, which the
scoper did not read.

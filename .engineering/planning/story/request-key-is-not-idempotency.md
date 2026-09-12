---
format: aep.planning-md/1
id: story:request-key-is-not-idempotency
kind: story
status: implemented
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
- confidence: cited
  path: src/runtime/swarm-server/tests/the_request_key_contract.rs
- confidence: cited
  path: src/runtime/swarm-server/tests/the_request_key_contract_under_attack.rs
- confidence: inferred
  path: src/web/src/runtime.ts
revision: 9
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

Rewritten 2026-09-12 from the implementor's confirmation table after the unit merged. The
corrections are visible rather than deleted, because the next wave selects on overlap by reading
this section.

**What the unit actually touched**, read from `git diff 0c24ac0...wave/2026-09-12b/request-key`:

| path | the mark it carried | what it turned out to be |
|---|---|---|
| `src/runtime/swarm-server/src/http.rs` | cited | touched. `Issue::request`'s doc, the omission clause, the log window and the canvas-record enumeration |
| `src/runtime/swarm-server/tests/redelivery_under_attack.rs` | cited | touched. Three stale present-tense quotations corrected; one case moved out of it |
| `src/web/src/runtime.ts` | **inferred** | **right.** It carried the wrong promise verbatim and was corrected |
| `src/runtime/ess-runtime/src/store.rs` | **inferred** | **wrong as a surface.** Read and found accurate; nothing in it changed. It is the definition the other three restate |
| `src/runtime/swarm-server/src/swarm.rs` | cited | **not touched.** `Delivery::request`'s doc had already been corrected by the redelivery unit on 2026-09-12 and satisfies the new check as it stands |
| `src/runtime/swarm-server/tests/the_request_key_contract.rs` | not scoped | **new file.** The whole check lives here |
| `src/runtime/swarm-server/tests/the_request_key_contract_under_attack.rs` | not scoped | **new file.** The adversary's, kept: an independent reimplementation of the rule that does not name the story, so it is not in the population it measures |

Two of the five scoped paths were wrong, in opposite directions: `store.rs` was named and needed
nothing, `swarm.rs` was cited and needed nothing. The scoper's own note said the doc branch would
not touch the store, and the doc branch is what the operator chose.

- **Confidence:** settled. Every line above is read from the merged diff.
- **Collides with:** any unit touching `swarm-server`'s HTTP surface, `src/web/src/runtime.ts`, or
  the two contract test files. `swarm.rs` and `store.rs` are **not** this story's surface.

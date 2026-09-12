---
format: aep.planning-md/1
id: story:request-key-is-not-idempotency
kind: story
status: draft
title: A request key does not make a retry idempotent
summary: The documented idempotency key guards the append, not the application.
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
revision: 1
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

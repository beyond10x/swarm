---
format: aep.planning-md/1
id: story:connectors-as-service-boxes
kind: story
status: draft
title: Reach a connectors adapter as a Service box
summary: An adapter operation becomes a box with typed ports.
relations:
- decomposes: epic:executable-boxes
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: src/core/domains/blackbox.yaml
- confidence: inferred
  path: src/runtime/swarm-server/src/state.rs
- confidence: inferred
  path: src/web/src/runtime.ts
revision: 7
---
## What

`connectors_v2` gives applications and agents typed, versioned access to integrations through
adapter services. A swarm has no way to reach one, so an agent that needs GitLab or a bounded SQL
read has to shell out and hope.

Expose a connectors adapter as a `Box{kind: Service}` whose ports carry the adapter's operations.

## Acceptance

A connectors adapter operation is reachable as a Service box: its ports cite a published schema
derived from the operation's contract, a Connection to it is live, and invoking it returns the
adapter's result onto the swarm's log. Credentials are the adapter's, never the swarm's.

## Out of scope

The federation host. One adapter, directly.

## Notes

`connectors-client` and `connectors-sdk` exist as crates. Nobody on this side has read them, so the
first task is establishing what the client's surface actually is.

## Scope

Derived 2026-09-12 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** `src/core/domains/blackbox.yaml` — cited; the acceptance is entirely in terms of `Box{kind: Service}`, `Port.schema_id`, `Schema` and `Connection`, and `Service` is already a declared variant.
- **Files (cited):** the workspace `Cargo.toml` — reaching the client means a new pinned dependency and a second HTTP stack beside the deliberate `ureq` choice recorded there.
- **Files (inferred):** a new host-half module in `swarm-server/src`, `state.rs` and `src/web/src/runtime.ts` if the operations must appear in `/spec`.
- **`connectors-client`'s surface, read first-hand.** Two public types, `Endpoint` and `Client`. Discovery is `describe()`, returning a `Descriptor` whose `operations` each carry `input_schema` and `output_schema` — so the port schemas this story wants are literally those fields. Invocation is `invoke(&descriptor, operation, input)`, which validates the operation exists, correlates a request id and returns the outcome. **Both are async, on `reqwest`.** Credentials enter only as a bearer token.
- **Confidence:** medium. The connectors API is read and the box vocabulary is cited; the swarm-side landing file does not exist and was inferred.
- **Not established:** how a swarm obtains an endpoint and token — no config surface for either — and whether the crate is consumable at all, since it implies `reqwest` and a tokio runtime in a workspace that deliberately has neither on the client side. That is a real cost and the first thing this story must settle.

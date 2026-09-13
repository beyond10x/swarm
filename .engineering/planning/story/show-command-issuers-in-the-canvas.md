---
format: aep.planning-md/1
id: story:show-command-issuers-in-the-canvas
kind: story
status: draft
title: Show command issuers in the canvas
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
- confidence: inferred
  path: src/runtime/swarm-server/tests/canvas_command_issuers.rs
- confidence: cited
  path: src/web/src/components/runtime/EventLog.vue
- confidence: inferred
  path: src/web/src/lib/event-rows.test.ts
- confidence: inferred
  path: src/web/src/lib/event-rows.ts
- confidence: cited
  path: src/web/src/runtime.ts
revision: 8
---
## Context

The attribution wave records issuer identity, but `src/web/src/components/runtime/EventLog.vue:48` still renders `event.actor`, and `:81` renders the live actor type. `src/runtime/swarm-server/src/swarm.rs:82` defines What::Applied without an issuer field; fixing only historical rows leaves live rows inconsistent. The saved canvas patch from the previous session is a design sketch: git apply --check reports it corrupt at line 61, so it must not be treated as ready implementation.

## Outcome

Historical and live canvas command rows show the claimed issuer separately from the specification's actor type.

## Acceptance

Two named agents sharing one actor type render distinguishable identities in both historical and live canvas rows, operator and runtime issuers remain distinguishable, and missing or legacy attribution is explicitly unknown rather than inferred to be a named agent.

## Required evidence

Cover historical reload and live SSE through the same user-visible row formatting: two agents of one role, operator, runtime, a legacy event whose issuer duplicates the actor type, an unknown/unnameable claimed label, and older responses lacking an issuer. Include mail so its sender display does not hide who issued the command. Preserve actor type as a separate fact and describe identity as claimed, not authenticated.

Resolve a claimed agent against known spawned agents or label it unresolved while retaining the raw claim; do not classify `agent:?` by prefix, because that prefix can also belong to a legitimate member. Historical label collisions remain unknown. Include a real SSE payload assertion together with visible-row formatting tests, rather than treating enum serialization alone as end-to-end evidence.

## Scope boundaries

Expected edits: `src/web/src/runtime.ts`, `src/web/src/components/runtime/EventLog.vue`, a focused formatting test/helper, and the additive live issuer field in `src/runtime/swarm-server/src/swarm.rs` plus a server test. Reuse Issuer and Recorded from `src/runtime/ess-runtime/src/store.rs`; no changes to their persisted format. No new kernel entity.

This story cannot run alongside `story:pause-and-stop-control-runtime-work` because both change swarm-server/src/swarm.rs. Schedule it in a later wave from the merged server changes; this is a surface collision, not a semantic dependency on cancellation.

## Out of Scope

Authenticating the claimed identity, changing permission rules, fixing unrelated event-list ordering, changing the evidence checker, and redesigning the canvas.

## Scope

Derived 2026-09-13 by `story-scoper`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** canvas event-row formatting and the server's live command announcement — cited.
- **Files:** `src/web/src/runtime.ts` — cited; `Recorded` and the applied `Change` variant currently omit issuer identity.
- **Files:** `src/web/src/components/runtime/EventLog.vue` — cited; `fromRecorded` and `fromChange` render actor types, and mail rows omit command attribution.
- **Files:** `src/runtime/swarm-server/src/swarm.rs` — cited; `What::Applied` at line 82 and its announcement in `Swarm::issue_as` at line 825 omit the issuer already available to persistence.
- **Also likely:** `src/web/src/lib/event-rows.ts` — inferred; extract the existing historical/live row formatters into a shared testable helper.
- **Also likely:** `src/web/src/lib/event-rows.test.ts` — inferred; exercise both visible row formats across named claims, operator, runtime, legacy, missing and unresolved identities, including mail.
- **Also likely:** `src/runtime/swarm-server/tests/canvas_command_issuers.rs` — inferred; verify the historical HTTP log and live SSE carry consistent claimed attribution.
- **Symbols:** `Recorded`, `Change`, `What::Applied`, `Swarm::issue_as`, `fromRecorded`, `fromChange`; reuse `Issuer::label` without changing persistence — cited.
- **Documents:** none required by the acceptance — inferred.
- **Confidence:** high — the story cites the omission and the current source confirms both server and canvas sites — cited.
- **Would collide with:** any unit editing `src/runtime/swarm-server/src/swarm.rs`, the canvas runtime API types, or event-row rendering — cited.

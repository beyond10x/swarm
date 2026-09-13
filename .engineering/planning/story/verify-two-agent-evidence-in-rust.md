---
format: aep.planning-md/1
id: story:verify-two-agent-evidence-in-rust
kind: story
status: draft
title: Verify two-agent evidence with Rust
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: examples/two-agents/
- confidence: cited
  path: src/runtime/swarm-check/
- confidence: cited
  path: website/scripts/spec-facts.mjs
- confidence: cited
  path: website/src/data/spec-facts.json
revision: 11
---
## Context

The operator rejected the newly introduced Python checker. `.engineering/waves/2026-09-13b-an-event-says-who-acted.md:163` records the promised Rust port after the attribution corrections. `examples/two-agents/check-two-agents.py:233` defines the current report; `:1275` defines the CLI. The corrected suite has 83 executed tests, including two adversary rounds. `Cargo.toml:3` still lists only three runtime crates.

## Outcome

A Rust `swarm-check` workspace binary owns the two-agent evidence checker and its regression suite, replacing the Python implementation and fixtures in `examples/two-agents/` in one integrated change.

## Acceptance

Running the Rust replacement produces equivalent evidence verdicts for the demonstration and the mapped regression corpus, with the explicit fail-closed inaccessible-evidence exception below, after the Python checker and its fixtures/tests have been removed from examples/two-agents.

## Required evidence

- Map every one of the 83 existing executed cases to a Rust case with its original behavioral claim; adversary scenarios and negative cases must not disappear behind a lower test count or a renamed happy path.
- Before deleting the old implementation, capture differential results on the existing fixtures and malformed/missing-input cases. Compare parsed JSON meaning and exit status, normalizing only temporary absolute root paths; text output retains clause names, reasons and the final verdict, allowing wrapping differences. The one intentional compatibility change is removal of the Python reader's read-write fallback: inaccessible evidence fails closed, and its error text may change. Record that difference explicitly.
- Read SQLite in read-only mode and prove absent/corrupt evidence yields a failing verdict or documented CLI error, never a newly created database or silently passing default. Accept directory and slug-with-data-root inputs and the existing JSON option.
- Preserve compatibility with legacy pre-issuer logs, malformed/unnameable issuers, unknown claimed agents, assignment identity matching, per-agent transcripts/spend and the last-verdict window. Seven met clauses still exit 0 even when unattended is NOT MET; stronger proof is a separate story.
- Use stable Rust fixtures for the demonstration and all cases; no Python invocation in the replacement binary, its tests, or its test setup. Preserve historical exported evidence; document any command-name change rather than rewriting historical run claims.

## Scope boundaries

New crate `src/runtime/swarm-check/`; remove/replace the checker, fixtures and test modules under `examples/two-agents/`; update that directory's current usage README. Existing domain declarations in `src/core/domains/{manager,agent,goal}.yaml` are read, not changed; this port introduces no kernel noun or event format. The issuer type in `ess-runtime/src/store.rs` is a read-only compatibility input, not an implementation surface.

The coordinator owns root Cargo.toml/Cargo.lock integration (workspace membership and any shared dependency choice), AGENTS.md, README.md, website scripts/facts, and current command references outside examples/two-agents. Record their needed changes in the handoff so the whole gate exercises the new crate and published commands work.

## Out of Scope

Porting the two pre-existing `src/core/bin` Python checkers, changing the evidence criteria, taking a new paid demonstration, runtime cancellation, authentication or containment. The narrow port fulfills the unfinished commitment without silently changing the verification policy.

## Original demonstration input

The committed export omits SQLite and original transcripts; it cannot reproduce a complete run by itself. Differential verification uses the existing `data/swarms/two-agents-proof` in the primary checkout read-only, with its path passed explicitly and the input hashes recorded. Confirm its availability before dispatch. If those originals disappear, report the missing evidence and do not fabricate replacement transcripts or call the demonstration comparison passed. Portable synthetic regression fixtures remain separate from that historical input.

## Scope

Derived 2026-09-13 by `story-scoper`. Every line is **cited** (read from the story or tree) or **inferred** (a reading that could be wrong).

- **Primary implementor surface:** new crate `src/runtime/swarm-check/`, owning the Rust binary, report logic, fixture builders and regression tests — cited by the story.
- **Replacement surface:** `examples/two-agents/`, removing its checker, fixture generator and five Python test modules; update its current usage README while preserving historical exported evidence — cited by the story and tree.
- **Behavioral source:** `Report`, `Records`, `read_events`, `issued_by`, `agent_named_by`, clauses one through seven, `unattended`, `render`, `as_json`, and `main` in the existing checker — cited.
- **Regression inventory:** 83 test methods: 65 checker cases, five first-round adversary cases, five second-round adversary cases, four issuer first-round cases and four issuer second-round cases; preserve their assertions and subcases, including issuer-vocabulary mutation guards — cited by the tree.
- **Coordinator-owned integration surfaces:** `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `README.md`, `website/scripts/spec-facts.mjs`, and `website/src/data/spec-facts.json`; implementor reports required integration changes instead of editing these shared files — cited by the story.
- **Read-only compatibility inputs:** existing kernel domain declarations and the runtime's issuer vocabulary; this story introduces no kernel entity or event format — cited by the story.
- **Confidence:** high — the story names the replacement boundaries and coordinator ownership, and the tree contains the report, CLI and all 83 test methods — cited.
- **Would collide with:** any unit modifying the two-agent checker, its fixtures, regression cases or current example documentation; shared workspace manifests and publication documents require coordinator sequencing — inferred.

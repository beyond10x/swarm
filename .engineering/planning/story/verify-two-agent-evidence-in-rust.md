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
  path: src/runtime/ess-runtime/src/store.rs
- confidence: cited
  path: src/runtime/swarm-check/
- confidence: cited
  path: src/runtime/swarm-cli/tests/adversary_the_shell_default_pass_2.rs
- confidence: cited
  path: website/scripts/spec-facts.mjs
- confidence: cited
  path: website/src/data/spec-facts.json
- confidence: cited
  path: website/src/pages/index.js
revision: 14
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

Final implementation scope, derived from the reviewed diffs on 2026-09-13. All landed surfaces below are **cited**.

- `src/runtime/swarm-check/` owns the binary, seven-clause report, separate unattended verdict, read-only evidence readers, local ordered value/string parser and Rust fixtures/regression tests.
- `examples/two-agents/` removes the seven Python implementation/fixture/test modules and documents the Rust command, original 83-case mapping and compatibility evidence. Historical exported evidence is unchanged.
- Coordinator-owned workspace integration: `Cargo.toml` and `Cargo.lock` register the crate and dependencies. Ordinary runtime JSON ordering and request hashing remain unchanged; evidence ordering stays local to the checker.
- Coordinator-owned command/documentation references: `AGENTS.md`, `README.md`, comments in `src/runtime/ess-runtime/src/store.rs`, and the diagnostic label in `src/runtime/swarm-cli/tests/adversary_the_shell_default_pass_2.rs`. The latter two changes alter no runtime behavior or test assertion.
- Coordinator-owned publication surfaces: `website/scripts/spec-facts.mjs`, `website/src/data/spec-facts.json`, and `website/src/pages/index.js` derive and display the separate checker size and include its integration tests in the published inventory.
- Existing kernel entity/event declarations and issuer vocabulary were read as compatibility inputs. This story introduces no kernel noun or event format. The parallel control story owns its separate manager descriptions and server behavior.
- Confidence: high, from the implementation and integration diffs. The initial scoper's inferred integration needs have been resolved to these cited paths. Future checker changes collide with this crate and example documentation; workspace and publication changes require coordinator sequencing.

## Implementation evidence

Implemented at unit head `7d5c596`, merged into the approved integration at `271242a`. Both adversary passes and the direct final correction review are recorded in the wave page. The final checker package executes 135 passing tests, retaining all original 83 mapped scenarios, both adversary rounds and numeric/Unicode baseline comparisons. The combined full gate at `8a3738c` passed all eleven steps: 311 Rust tests and 71 web tests; final `npm run spec:check` exited 0.

The integrated binary matches parsed original demonstration JSON and text words, allowing wrapping differences. Both CLI invocations exit 0, all seven clauses are met, and unattended remains false. All 17 original input paths and SHA256 hashes are unchanged, including SQLite/WAL/SHM. No historical evidence export changed.

The compatibility limits remain explicit: inaccessible evidence fails closed without SQLite write fallback; surrogate encoding errors use a concise diagnostic; Python's environment-dependent stack-exhaustion threshold is not emulated. Measured accepted nesting through 30,000 and 100,000-level malformed-input handling pass. Ordinary runtime JSON ordering/request hashes remain unchanged.

Evidence: `.engineering/waves/2026-09-13c-control-and-rust-checker.md`, `/home/timo/.cache/swarm-wave-2026-09-13c/checker/correction-2-report.md`, and `/home/timo/.cache/swarm-wave-2026-09-13c/integration/gate/`. Main merge/publication state is recorded in the wave handoff. Story lifecycle remains operator-owned and unchanged.

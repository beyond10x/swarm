---
format: aep.planning-md/1
id: story:refuse-a-value-outside-a-declared-enum
kind: story
status: implemented
title: The interpreter writes a value the specification does not declare
summary: 504 occurrences of "ClaudeCode", which swarm.config.Harness does not name, written into every log.
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
scope:
- confidence: cited
  path: examples/two-agents/evidence/2026-09-13-two-agents-proof/README.md
- confidence: cited
  path: src/runtime/ess-runtime/src/apply.rs
- confidence: cited
  path: src/runtime/ess-runtime/tests/refuses_a_value_outside_an_enum.rs
- confidence: cited
  path: src/runtime/swarm-check/tests/fixtures
- confidence: cited
  path: src/runtime/swarm-check/tests/support/fixtures.rs
- confidence: cited
  path: src/runtime/swarm-server/src/coordinator.rs
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
revision: 5
---
## What

`swarm.config.Harness` declares `[Claude, Codex, B10x]`. `swarm-server` spawned every agent with
`harness: "ClaudeCode"`, and the interpreter wrote it into the event log without a word — **504
occurrences** across this tree, plus **7** of an invalid `"Native"` tool surface the same sweep
found. Nothing could consume either: `coordinator.rs` matches `Some("Claude")` when it reads a
launch line back, so the value was not merely invalid, it was inert.

Until now the specification was authoritative over STRUCTURE — which outcome is taken, what moves,
what is emitted — and over nothing else. A value at an enum position was never compared to the
vocabulary the enum closed.

## Why it belongs in the interpreter and not in `ess`

A diagnostic family in `ess` reads specification TEXT at compile time. `"ClaudeCode"` appears in no
specification file: it is a runtime value in a JSON invocation `ess` never sees. The interpreter is
the only thing that holds the compiled vocabulary and the invocation at the same moment, so it is
the only place the two can be compared at all.

It is also total and decidable straight from the IR — `ResolvedBody::Enum { variants }` is a finite
list of strings — which is what separates it from the general invariant checking ESS declines.

## The stated gaps

A union's tag and an enum behind a newtype are closed vocabularies by the same argument, and are
deliberately not read: this specification declares neither, so a branch for either would be a
refusal nothing in this system can produce. Primitives are not checked either — that is type
checking, and this is membership.

## The sealed evidence is not edited

`examples/two-agents/evidence/2026-09-13-two-agents-proof/events.csv` carries `"ClaudeCode"` in row
2 and **stays as it is**. It is retained evidence behind
`verification-report:two-agents-under-a-frame-2026-09-13`; rewriting a row to a value the run did
not produce would falsify a sealed artifact to flatter a later fix. A note beside it records what
the value is, why it was written and when the writer was corrected. Nothing in the verdict rests on
it — `swarm-check` never reads `harness` — and replay is unaffected, because `store.rs::fold`
replays a log without going through `apply`.

## Acceptance

`apply` refuses an invocation whose input carries a value at an enum position that the enum does
not declare, before it selects an outcome, with `ApplyError::NotAVariant` naming the command, the
path, the type, the value as JSON and every declared variant. Nothing in the tree writes an
undeclared variant any more, the four `swarm-check` fixture corpora are regenerated, and the
refusal has a case of its own that fails if step 0 is removed.

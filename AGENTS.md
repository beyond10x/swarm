# AGENTS.md — swarm

A swarm manager whose kernel is an Executable System Specification. `README.md` is the public
introduction and is written for humans; it owns the pitch and the motivation, and nothing here
repeats them. This file owns the tree, the commands and the rules.

## The one thing to understand before changing anything

The specification in `src/core/` is the system. `swarm-server` compiles it at boot
(`src/runtime/swarm-server/src/main.rs:39`) and refuses to start if it does not resolve. The runtime
**interprets** that compiled IR; it does not generate code from it.

So the ordinary way to add behaviour is to edit YAML, not Rust. A command added to a domain file is
reachable through `POST /swarms/{slug}/commands/{command}`, through `swarm-cli do`, and on the
canvas, without any crate being touched. Reach for Rust only when the thing you need is genuinely
the host's — a process to launch, a clock to read, a socket to bind.

**There is no codegen step.** `src/core/generated/` is gitignored and is not present in a clean
checkout. `ess generate` is an optional manual projection — `--kind schema` feeds
`src/core/bin/check-docs.py`, `--kind openapi` projects a route surface nobody consumes yet. It is
never run by a build and there is no check that its output is current, because there is no committed
output. Do not go looking for generated code, do not add a "generated files are stale" check, and do
not describe this system as code-generated anywhere — it is not, and the distinction is the
interesting part.

## Layout

| path | what it is |
|---|---|
| `src/core/` | **the specification, and the source of truth.** `system.yaml`, `components.yaml` (bindings and wiring), `topology.yaml`, `domains/*.yaml`, plus `bin/` checkers and `docs/ess-authoring-rules.md` |
| `src/runtime/ess-runtime/` | the interpreter. Compiles the spec, folds events into entities, computes views, routes bindings. Knows nothing about swarms |
| `src/runtime/swarm-server/` | HTTP surface, per-swarm event streams, and the trigger that fires periodic bindings |
| `src/runtime/swarm-cli/` | the verbs a coordinator uses from inside a turn — mail, views, and `do` |
| `src/web/` | the Vue canvas. Reads `/spec` at load rather than hard-coding entities |
| `website/` | the public Docusaurus site at `beyond10x.github.io/swarm/` |
| `data/` | runtime state — per-swarm event logs, transcripts and work directories. Not specification input; `ess-inputs.yaml` exists because the legacy layout would otherwise read instance documents as sources |
| `.engineering/` | the planning store and wave notes |

The route table is `src/runtime/swarm-server/src/http.rs`; the CLI verbs are the `Verb` enum in
`src/runtime/swarm-cli/src/main.rs`. Both are hand-written, and `http.rs:7-11` says so about itself.

## Build, run, test

```console
cargo build                                  # the three runtime crates
cargo run -p swarm-server                    # serves on :5000, or $SWARM_PORT
cargo run -p swarm-cli -- --help             # the coordinator's verbs
cargo test                                   # unit and integration tests
cargo test -p swarm-server --test serves_a_swarm    # one integration case

ess specify validate --path src/core         # `swarm v1 — 9 file(s), valid`
ess specify compile --path src/core --format json   # the IR the runtime holds

cd src/web && npm install && npm run dev     # the canvas, against a running server
cd src/web && npm test                       # node --test over src/**/*.test.ts
cd src/web && npx vue-tsc --noEmit           # types
cd src/web && npx vite build

cd website && npm install && npm run build   # the public site
cd website && npm run spec:check             # the site's published numbers are current
```

`ess` and `aep` are separate tools and must be on `PATH`. The workspace pins `ess-*` crates to tag
`0.23.0` and `eventlog-*` to `0.2.1` by git, not by path — the sibling checkouts under `../` are on
different versions, and a path dependency would compile neither the thing we mean.

## The website is a sanctioned exception

Atlas stamps a paragraph into every Beyond10x repository's `AGENTS.md` saying that Website plus Docs
System own rendering and that a repository must **not add a standalone docs deployer**; Pages at
`/<repository>/` is meant to be a generated redirect facade only.

**swarm deploys a real site at `https://beyond10x.github.io/swarm/` anyway, from
`.github/workflows/deploy-website.yml`, and that is a decision the maintainer took on 2026-09-12
with the contract in front of him.** It is written here so the next agent does not read the contract,
conclude the workflow is a mistake, and delete it.

What still holds: `b10x.docs.yaml` declares the four markdown files Atlas collects into
`/docs/swarm/` on the unified site, and `website/**` is excluded from that collection. The two
publications are separate and both are intended.

`swarm.beyond10x.dev` was never real. `dig beyond10x.dev` returns no A record and no NS delegation,
and the name appears in no other repository in the organisation; it arrived with the scaffold import
commit `b7a7b78`. The address is `https://beyond10x.github.io/swarm/`.

## The gate

Every step, in this order. **Read each command's own exit status.** Never a pipeline's, never a
wrapper's, and never the last line of combined output — a step that selected nothing exits 0 and is
indistinguishable from a step that passed.

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets`
3. `cargo test`
4. `ess specify validate --path src/core`
5. `python3 src/core/bin/check-sets-are-emitted.py`
6. `aep plan artifact validate`
7. `npm test` in `src/web`
8. `npx vue-tsc --noEmit` in `src/web`
9. `npx vite build` in `src/web`
10. `npm run build` in `website`
11. `npm run spec:check` in `website`

Step 11 is part of the gate, not an extra. `website`'s build runs `scripts/spec-facts.mjs` first,
which rederives every number the public site states; `--check` compares the committed
`src/data/spec-facts.json`, the README and this file against the specification and fails when any of
them is stale. It is the check that did not exist when the site published four wrong counts for its
entire life, and it also guards the figures in prose, which no build step can import.

`src/core/bin/check-docs.py` is not in the gate because it needs `ess generate --kind schema` run
first. Run it when instance documents under `data/` change.

## The two rules beyond ess/4

`src/core/bin/check-sets-are-emitted.py` enforces two things ess/4 does not, both over the compiled
IR:

1. **Every field an outcome writes must appear as a payload field on an event that outcome emits.**
   ess/4 requires every field *on* an emitted event to declare a source; this requires every field an
   outcome *writes* to reach an event at all. An outcome that sets a field no event carries passes
   ess/4 and fails here, because a replay would never see the write.
2. **Every emitted event must carry its subject's identity field**, or the fact it records is
   unattributable.

Together they are what make entity state a fold over the log rather than a cache with a log beside
it. Neither is retrofittable: once a log exists, its events are already missing whatever they were
allowed to omit. The script exits 1 and names each violation as `command.outcome`.

A related consequence, worth knowing before writing an outcome: **the model does not read a clock.**
The specification decides what an outcome writes, not what time it is, so timestamps arrive as
command inputs and the runtime reads the clock once, at the edge.

## One test target is excluded from `cargo test`

`src/runtime/swarm-server/Cargo.toml` declares:

```toml
[[test]]
name = "open_does_not_queue_behind_an_unrelated_slug"
test = false
```

The case measures a property the runtime does not have — an open of an unrelated slug still waits,
because `Swarm::open`'s replay is CPU-bound work inside an `async fn` and occupies a tokio worker —
and it measures it as a race between two wall-clock durations. It passes on an idle machine and
fails on a loaded one: five of five failures at load average 6.5–10.9.

`test = false` rather than `#[ignore]` is deliberate. The case keeps compiling, so it cannot rot, and
it can still be run on purpose:

```console
cargo test -p swarm-server --test open_does_not_queue_behind_an_unrelated_slug
```

It goes back into the default set when `story:open-blocks-the-runtime` closes. Do not delete it, and
do not weaken it to make it green.

## Planning store

`.engineering/planning/` is written **only** through the `aep plan artifact` CLI. Never open a file
under it in an editor — the CLI owns those files, and a body is changed with `aep plan artifact
body`. `aep plan artifact validate` is step 6 of the gate. Never run `aep plan artifact move`:
whether work is done is a claim about the world that rests on evidence an operator reads, not on an
agent having finished typing.

## What is outside the specification, by design

Naming these matters, because "fully specified" is true of the kernel and not of the host, and the
difference is where the surprises live:

- the HTTP route table (`http.rs:7-11` — `ess generate --kind openapi` could project it; today it is
  hand-written);
- the coordinator process protocol — `metaharness run claude`, the prompt, the verdict line;
- retry bounds. `swarm.rs:48-52`: "Neither number is the specification's." ESS declares `delivery`
  and `on_failure` and says nothing about attempts or spacing;
- the pump depth cap and the budget caps;
- the UI component library;
- every clock read.

## Honesty rules for anything published

The public site and `README.md` were audited against the tree. Three findings stand, and any new
prose must respect them:

- **"fully code-generated" is false.** No `build.rs`, no codegen step, no generated file, no marker,
  no staleness check. Say *interpreted*.
- **Multi-agent operation is demonstrated, at depth one and breadth two.** This finding used to read
  *"a working agent swarm is overstated"*, and it was true until 2026-09-13. It is retired by a
  recorded run, not by a decision. The swarm was `two-agents-proof`; its evidence is exported to
  `examples/two-agents/evidence/2026-09-13-two-agents-proof/`, because `data/swarms/*/` is runtime
  state and gitignored. A coordinator spawned a
  `Worker`, posted it an assignment, and waited while the runtime ran it as its own session —
  `AssignmentTaken` at seq 13, `AssignmentDone` at seq 18, `GoalReached` at seq 23, two agents'
  transcripts and two agents' spend rows. All three of those events had fired **zero times** in this
  repository's history before that run.
  `verification-report:two-agents-under-a-frame-2026-09-13` records it clause by clause, and
  `examples/two-agents/check-two-agents.py` re-derives the verdict from the log at any time.
  **What is still overstated:** one coordinator, one worker, one assignment. Agents spawning agents,
  more than two agents, and a swarm that extends its own specification are all undemonstrated.
- **Nothing confines a coordinator — and a turn is now narrowed.** metaharness gives hermeticity and
  a complete event record, not containment. Its `--substrate` flag is refused by name for this arm,
  because a socket configured there would be accepted, never consulted, and read as containment
  nobody applied; `--write-scope` and `--cgroup-root` are refused for the same arm for the same
  reason. Its own attestation says a hermetic run here is not network-isolated.
  **What did change on 2026-09-13:** every turn launches under a sealed `metaharness.frame/1`
  document with `--decisions frame`, so a tool call outside the admitted set is *refused at the
  decision seam* and a write outside the agent's own work directory is refused by a subject scope the
  runtime derives — never reads from a config, because a config a coordinator writes for itself could
  widen its own scope. That is refusal, not containment, and the distinction is the whole of this
  entry. Measured in the demonstration: 1 of 41 decided calls refused, `decided_by: frame`.
  One weakness is pinned rather than fixed: metaharness judges a call by the first rule any of its
  subjects matches, so an outside path is admitted when the same call also names an admitted one.
  Nothing found emits such a call; `frame::scope`'s doc states the condition, and two cases assert
  today's behaviour with their inversion triggers.

No count in `website/` may be typed by hand. `website/scripts/spec-facts.mjs` derives all of them
from the tree into `website/src/data/spec-facts.json`, and fails loudly rather than emitting a zero
when an anchor it parses has moved. If a number needs to reach the site, teach the script to derive
it.

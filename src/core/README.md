# swarm — a swarm manager, specified

An Executable System Specification (ess/1, validated with `ess 0.23.0`) of a **swarm manager**. A
swarm starts bare — one coordinator, no data, one goal — and builds what it needs. This directory is
the kernel: only what a swarm cannot build for itself.

| check | command | result |
|---|---|---|
| validates | `ess specify validate --path src/core` | `swarm v1 — 7 file(s), valid`, exit 0 |
| compiles | `ess specify compile --path src/core --format json` | exit 0 |
| every written field is replayable | `src/core/bin/check-sets-are-emitted.py` | 0 violations |

Size: 4 domains · 7 entities · 21 types · 44 commands · 75 outcomes · 46 events · 20 views ·
11 errors · 7 actors · 3 components · 1 binding · 3 workloads (`ess specify compile`, counted).

## What is in the kernel, and why nothing else is

Timo, 2026-09-11: *"it usually starts simple: you need to solve tool access, orchestrator needs to
be able to create the agents and they can basically build anything anyway — we do provide inbox +
eventbus like connections + presentation layer for the human to observe. from there on they build
what they need and like."*

| kernel piece | where |
|---|---|
| the swarm record, and its lifecycle | `domains/manager.yaml` |
| how an agent is launched — harness, binary, model, budgets | `domains/config.yaml` `HarnessLaunch` |
| the agent itself, and the coordinator that spawns the rest | `domains/agent.yaml` |
| what anything may reach, and how things connect | `domains/blackbox.yaml` — `Box`, `Port`, `Schema`, `Connection` |

Everything else is a **Box** the swarm draws and a **Connection** it wires. A tool is a box. An MCP
server is a box. A UI panel is a box. The loop is a box and so is the goal. Two boxes connect only
when their ports cite the same published schema, and what a coordinator may reach is the set of
connections drawn to it — granting a tool is drawing an edge, revoking it is removing one.

An earlier draft (2026-09-11) was reverse-engineered from a live 16-agent run and carried 11
domains: `board`, `schedule`, `orchestrator`, `work`, `fault`, `decision` and `flow` alongside these
four. Each described something that run had built for itself. A swarm that starts with them has not
started bare, so they were removed. They are recoverable from the import commit.

## Two rules this system adds to ess/1

ESS is a specification format and says nothing about how state is stored. The runtime is
event-sourced — an entity is a fold over the events its commands emitted — so two rules hold here
that ess/1 does not enforce, checked by `bin/check-sets-are-emitted.py`:

1. **Every field an outcome `sets:` is carried on an event that outcome emits.** A field written and
   never emitted is a fact a replay cannot recover.
2. **Every emitted event carries its subject's identity.** An event that does not say whose it is
   cannot be attributed.

Neither is retrofittable. Once a log exists, the events in it are already missing whatever they were
allowed to omit. The first check found 31 violations in the 2026-09-11 draft.

A related consequence: **the model does not read a clock.** ESS determines what an outcome writes
but not what time it is, so `created_at`, `started_at` and `stopped_at` are handed in as command
inputs and emitted like any other field. The runtime reads the clock at the edge.

## Layout

```
system.yaml            the header: 4 domains
ess-inputs.yaml        the exact input list — the legacy layout otherwise reads EVERY *.yaml here
domains/*.yaml         one domain each
components.yaml        3 subsystems and the bindings between them — the dataflow
topology.yaml          replica floors and ceilings per swarm
bin/check-sets-are-emitted.py   the two rules above, over the compiled IR
docs/ess-authoring-rules.md     what ess/1 accepts and refuses, as probed here
```

Instance data is **not** here. It lives in `../../data/`, one directory per swarm.

## Rough edges of ess/1 met here (0.23.0)

| edge | what was done |
|---|---|
| a field carries exactly one relation (ESS-ENTITY-006) | ownership kept on the owner's side |
| outcome names must be lower-kebab; transition names may carry `_` | renamed outcomes |
| a struct invariant is `field op literal`, never field-vs-field (ESS-TYPE-002) | Boolean inputs; prose |
| `reached_by: command_line` needs a `cli:` block | `in_process` until the block is written from the real grammar |
| "a literal in a binding is text" | the id rides on the event; a binding's command takes only what an event can carry |
| a binding maps one event field per input; no fan-out | prose |
| a `when` reads input only (ESS-COMMAND-003) | prose + a declared error the implementation raises |
| no timer, no DAG, no UI, no JSON construct | actor + commands; struct lists; `json_schema: String` |

## Regenerate

```
ess specify validate --path src/core
ess generate --path src/core --kind schema  --out src/core/generated/schema
ess generate --path src/core --kind openapi --out src/core/generated/openapi
ess generate --path src/core --kind site    --out src/core/generated/site
ess specify graph --path src/core --format dot | dot -Tsvg > src/core/generated/interaction.svg
src/core/bin/check-sets-are-emitted.py
```

`generated/` is a projection and is gitignored. Never edit it.

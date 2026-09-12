# swarm — a swarm manager, specified

An Executable System Specification (ess/4, validated with `ess 0.23.0`) of a **swarm manager**. A
swarm starts bare — one coordinator, no data, one goal — and builds what it needs. This directory is
the kernel: only what a swarm cannot build for itself.

| check | command | result |
|---|---|---|
| validates | `ess specify validate --path src/core` | `swarm v1 — 9 file(s), valid`, exit 0 |
| compiles | `ess specify compile --path src/core --format json` | exit 0 |
| every written field is replayable | `src/core/bin/check-sets-are-emitted.py` | 0 violations |

Size: 6 domains · 10 entities · 24 types · 53 commands · 56 events · 27 views · 16 errors ·
12 actors · 4 components · 2 bindings · 4 workloads (`ess specify compile`, counted).

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
| how one agent says something to another | `domains/mailbox.yaml` — `Mailbox`, `Message` |
| what anything may reach, and how things connect | `domains/blackbox.yaml` — `Box`, `Port`, `Schema`, `Connection` |

Everything else is a **Box** the swarm draws and a **Connection** it wires. A tool is a box. An MCP
server is a box. A UI panel is a box. The loop is a box and so is the goal. Two boxes connect only
when their ports cite the same published schema, and what a coordinator may reach is the set of
connections drawn to it — granting a tool is drawing an edge, revoking it is removing one.

An earlier draft (2026-09-11) was reverse-engineered from a live 16-agent run and carried 11
domains: `board`, `schedule`, `orchestrator`, `work`, `fault`, `decision` and `flow` alongside these
four. Each described something that run had built for itself. A swarm that starts with them has not
started bare, so they were removed. They are recoverable from the import commit.

`board` came back on 2026-09-12 as `mailbox`, and the note it was deleted under — "an inbox needs a
correspondent and there is one agent" — is what makes the case: it is true of a newborn swarm and
false of the swarm it becomes ten minutes later. A coordinator that can spawn agents and cannot
address them has built a roster it cannot use. The recovered file's arguments were kept; one was
corrected, because expressing "unread" as `read_at == null` is a filter this runtime's three-valued
evaluator reads as `Unknown` and drops, which would hide every unread message from its own inbox.

## The loop is declared, not hand-built

`components.yaml` carries the `[loop]` of the birth graph as a **periodic binding** — ess/4's own
timer construct, added in ess/3:

```yaml
  - id: turn-the-loop
    when:
      periodic:
        every: PT30S
        overlap: serial_per_instance
        missed: one_pending_drop_excess
    invoke: {command: swarm.goal.Pursue}
```

Every profile word but `every` admits exactly one value, so the block is mostly a contract being
acknowledged rather than configured. `serial_per_instance` is the one worth reading: one turn of one
goal at a time, which is what stops a slow coordinator being asked to pursue the same goal twice.
`one_pending_drop_excess`: a tick missed while a turn was running is taken once afterwards, not
banked and replayed.

The host — the runtime — supplies which goal to turn, by reading `swarm.goal.GoalsAwaitingATick`,
and answers `eligibility` for whether the swarm is running at all. The cadence is not its business.

## Two rules this system adds to ess/4

ess/4 already requires every field on an emitted event to declare a source — a mapping, or
`{generated: true}` where the implementation mints it, which is how the eleven identity fields in
this kernel are spelled. Two further rules hold here, checked by `bin/check-sets-are-emitted.py`:

1. **Every field an outcome `sets:` is carried on an event that outcome emits.** A field written and
   never emitted is a fact a replay cannot recover.
2. **Every emitted event carries its subject's identity.** An event that does not say whose it is
   cannot be attributed.

The first is the one ess/4 does not cover: it makes events honest about where their values come
from, and this makes the entity rebuildable from them. An outcome that sets a field its event does
not declare passes ess/4 and fails here. Neither is retrofittable — once a log exists, the events in
it are already missing whatever they were allowed to omit. The check found 31 violations in the
2026-09-11 draft.

A related consequence: **the model does not read a clock.** ESS determines what an outcome writes
but not what time it is, so `created_at`, `started_at` and `stopped_at` are handed in as command
inputs and emitted like any other field. The runtime reads the clock at the edge.

## Layout

```
system.yaml            the header: 6 domains
ess-inputs.yaml        the exact input list — the legacy layout otherwise reads EVERY *.yaml here
domains/*.yaml         one domain each
components.yaml        4 subsystems, the bindings between them, and the loop's tick
topology.yaml          replica floors and ceilings per swarm
bin/check-sets-are-emitted.py   the two rules above, over the compiled IR
docs/ess-authoring-rules.md     what ess accepts and refuses, as probed here
```

Instance data is **not** here. It lives in `../../data/`, one directory per swarm.

## Rough edges met here (ess 0.23.0)

| edge | what was done |
|---|---|
| a field carries exactly one relation (ESS-ENTITY-006) | ownership kept on the owner's side |
| outcome names must be lower-kebab; transition names may carry `_` | renamed outcomes |
| a struct invariant is `field op literal`, never field-vs-field (ESS-TYPE-002) | Boolean inputs; prose |
| `reached_by: command_line` needs a `cli:` block | `in_process` until the block is written from the real grammar |
| "a literal in a binding is text" | the id rides on the event; a binding's command takes only what an event can carry |
| a binding maps one event field per input; no fan-out | prose |
| a `when` reads input only (ESS-COMMAND-003) | prose + a declared error the implementation raises |
| no DAG and no UI construct | a flow is a Box a swarm draws; a panel is a `Box{kind: Ui}` |
| no JSON construct | `json_schema: String` |
| a periodic cause needs ess/3, and complete payloads ess/4 | the system declares `format: ess/4`, which is why the tick above is expressible at all |

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

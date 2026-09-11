# swarm2 — a swarm manager, specified

An Executable System Specification (ess/1, validated with `ess 0.23.0`) of a **swarm manager**:
many swarms, each one tmux session with one orchestrator and many agents, composed top-down from
the manager to the single step of a DAG. Drafted 2026-09-11 from `../swarm` (the live tooling of the
2026-09-11 run), `../.agents/`, `../../aep/workflows/` and the run's decisions ledger. Every
entity, field, command and view carries a comment naming what it was read from; what nothing says
is marked `UNMAPPED:` (22 markers, `grep -rn UNMAPPED domains/`).

| check | command | result 2026-09-11 |
|---|---|---|
| validates | `ess specify validate --path swarm2` | `swarm2 v1 — 14 file(s), valid`, exit 0 |
| compiles | `ess specify compile --path swarm2 --format json` | exit 0 |
| instance documents conform | `swarm2/bin/check-docs.py` | 5 documents valid (1 config, 4 flows) |
| suites synthesizable | `ess verify conform synthesize --path swarm2 --format text` | 655 scenarios, 43 refusals (ESS-SYNTH-010, see rough edges) |
| site renders | `ess generate --path swarm2 --kind site --out swarm2/generated/site` | 18 artifacts, 20 state diagrams |

Size: 11 domains · 20 entities · 60 types · 111 commands · 136 events · 63 views · 31 errors ·
29 actors · 8 components · 17 bindings · 8 workloads (`ess specify compile`, counted).

## The kernel, and what is built on it

Timo, 2026-09-11: *"it usually starts simple: you need to solve tool access, orchestrator needs to
be able to create the agents and they can basically build anything anyway - we do provide inbox +
eventbus like connections + presentation layer for the human to observe. from there on they build
what they need and like."*

| kernel piece | where | how |
|---|---|---|
| tool access | `domains/agent.yaml` `Loadout` (skills, tools, mcp_servers, plugin_dirs), `domains/config.yaml` `HarnessLaunch` | an agent is spawned with a loadout; the config says which harness, binary, model and args |
| the orchestrator creates agents | `domains/agent.yaml` `Spawn`, `Assign`; `flows/agent.spawn.yaml` | AGENTS.md §4 as a five-step flow: pick a role, guarantee the loadout, create the inbox, post the assignment, tell the rules |
| inbox | `domains/board.yaml` (ported from `../swarm/domains/board.yaml`, decisions 9y/10y/34 unchanged) | named inboxes per agent, a post addressed to a pair, read marks, ack terminal, broadcast fan-out |
| eventbus-like connections | `domains/blackbox.yaml` | boxes with only inputs and outputs, schemas known to everyone (`PublishedSchemas`), connections other agents make, with ESS's own `delivery`/`on_failure` words |
| presentation layer | `../website/` (Vue 3) and `generated/site/` (ess) | splash tiles, create a swarm, the canvas, DAG render, the component library |

What the agents then build: `domains/flow.yaml` — a Flow is a DAG of steps (`Command | Tool |
Llm | Subflow`) with typed ports and a context; `Subflow` is the recursion. What the first
orchestrator built for itself on 2026-09-11, kept because it is measured: `orchestrator.yaml`
(generations, the two cycles, the four verdicts), `schedule.yaml` (wake gates and crons),
`decision.yaml` (numbered binary decisions), `work.yaml` (the `adp/default/2` work item and waves),
`fault.yaml` (the incident-shaped fault workflow), `config.yaml` (the runtime config as a record).

## Naming

| level | noun | why |
|---|---|---|
| 0 | `SwarmManager` — the system | what he called it |
| 1 | `Swarm` — a member of the manager | one tmux session, one run record, one orchestrator |
| 2 | `Agent` — a member of a swarm | `board.yaml` `agent_id`, `eventlog.sh --agent`, `AEP_ACTOR=agent:<slug>`, `.agents/` all key on it |
| 2 | `Orchestrator` — the one main process | an agent with the reserved role, replica ceiling 1 (`topology.yaml`) |
| 2 | `Role`, `Host`, `Loadout` | the kind, where it runs now, what it may reach |
| 3 | workflow | a lifecycle in the aep shape: states, guarded transitions, back-edges, a terminal `Declined`/`Escalated` |
| 4 | `Flow` | the DAG inside one workflow state (`binds: {entity, state}`) |
| 5 | `Step` | `command`, `tool`, `llm`, `subflow` — input, output, context |

## Layout

```
system.yaml            the header: 11 domains
ess-inputs.yaml        the exact input list — needed because the legacy layout reads EVERY *.yaml
                       under the directory, and config/, flows/, generated/ are not sources
domains/*.yaml         one domain each; every construct carries its provenance comment
components.yaml        8 subsystems, 17 bindings, 4 conversions — the wiring
topology.yaml          replica floors and ceilings per swarm
config/                one Config record: the 2026-09-11 run's values, each with its source
flows/                 four Flow records: check-in, memory-tick, spawn, wake-pass
generated/             projections — never edited: site/, schema/, openapi/, interaction.svg
bin/check-docs.py      validates config/ and flows/ against generated/schema (jsonschema)
docs/ess-authoring-rules.md   what ess/1 accepts and refuses, as probed here
```

## Subsystems (level 2)

| component | owns | reached_by | schedulers |
|---|---|---|---|
| `swarm-manager` | manager, config | network — `generated/openapi/` is its interface | — |
| `agent-runtime` | agent, work, fault | in_process (a `cli:` block is the follow-up) | one wake gate per agent |
| `board` | board | in_process | none — a post wakes nobody (WORKING-RULES §4) |
| `blackbox-bus` | blackbox | network | — |
| `flow-engine` | flow | in_process — driven, later by metaharness | — |
| `orchestrator` | orchestrator | in_process | the `*/1` and `*/15` crons |
| `scheduler` | schedule | in_process — the `Clock` actor lives here | *is* the scheduler |
| `decision-console` | decision | in_process | — |

The 17 bindings are read in `components.yaml`; the picture is `generated/interaction.svg`. Four of
them are the level 3 → 4 edges: a cron tick or a spawn or a wake starts a flow (`flow.StartRun`
with a literal `flow_id`).

## Workflows (level 3)

| entity | states | mirrors |
|---|---|---|
| `manager.Swarm` | Created → Running → Paused → Stopped → Deleted | create / update / delete |
| `work.WorkItem` | Receive → Specify → Decompose → EstablishVerifiers → Implement → Verify → AdversarialVerify → Review → Complete \| Declined | `aep/workflows/development/default.yaml` v2, names verbatim, all five back-edges |
| `fault.Fault` | Detected → Diagnosed → Mitigated → Recovered → Learned \| Escalated | `aep/workflows/incidents/standard.yaml` |
| `agent.Agent` | Spawned → Assigned → Working → Blocked \| Parked \| Idle → Faulted → Retired | WORKING-RULES §8 verdicts, §10 event kinds |
| `orchestrator.Generation` | Seeded → Ticking → Deferred → Restarting → Retired | CHARTER §5 |
| `decision.Decision` | Open → Answered \| Held \| Moot \| Withdrawn \| Taken | WORKING-RULES §9, `decisions.md` |
| `schedule.WakeGate` / `Cron` | Armed → Fired → Armed / Scheduled → Cancelled \| Expired | wake-gate.md, SEED.md §1 |
| `flow.Run` / `StepRun` | Pending → Running → Succeeded \| Failed \| Cancelled / Pending → Ready → Running → Succeeded \| Failed \| Skipped | the driver |
| `blackbox.Box` / `Connection` / `Schema` | Draft → Published → Retired / Proposed → Live → Broken → Removed / Draft → Published → Deprecated | his words for the canvas |
| `config.Config` | Draft → Active → Superseded \| Discarded | one active per swarm |

## Rough edges of ess/1 met here (0.23.0)

| edge | where it bit | what was done |
|---|---|---|
| a field carries exactly one relation (ESS-ENTITY-006) | Swarm owns Decision vs Decision references Swarm | ownership kept on the owner's side |
| outcome names must be lower-kebab; transition names may carry `_` | every domain | renamed outcomes |
| a struct invariant is `field op literal`, never field-vs-field (ESS-TYPE-002) | `Budgets`, `RetryStep` | Boolean input `attempts_left`; prose |
| `reached_by: command_line` needs a `cli:` block | five components | `in_process` until the block is written from the real grammars |
| "a literal in a binding is text" | every Uuid/Integer/Boolean input a binding wanted to fill | the id rides on the event; commands a binding invokes take only what an event can carry |
| a binding maps one event field per input; no fan-out | `ReviseBox` breaking N connections, the poll's five Booleans | prose; no binding from `MessagePosted` to the gate |
| an escalation event must be declared | seven `escalate:` events | declared in the owning domain, emitted by no outcome |
| a `when` reads input only (ESS-COMMAND-003) | "a row exists", "already stamped", "schema equal" | prose + a declared error the implementation raises |
| `terminal:` is not cross-checked against `transitions.from` | `StepRun.Failed` | comment |
| ESS-SYNTH-010: a binding with a failure policy needs an `external:` failing branch on the invoked command | 43 synthesize refusals | recorded; the follow-up is `external:` on `StartRun`, `PostMessage`, `Raise`, `DetectFault`, `Unpark`, `Seed`, `AdoptConfig`, `OpenRun`, `Wake` |
| the legacy layout reads every `*.yaml` recursively | `config/`, `flows/` | `ess-inputs.yaml` |
| no timer, no DAG, no UI, no JSON construct | schedule, flow, blackbox | actor + commands; struct lists; `json_schema: String` |

## Regenerate

```
ess specify validate --path swarm2
ess generate --path swarm2 --kind site   --out swarm2/generated/site
ess generate --path swarm2 --kind schema --out swarm2/generated/schema
ess generate --path swarm2 --kind openapi --out swarm2/generated/openapi
ess specify graph --path swarm2 --format dot | dot -Tsvg > swarm2/generated/interaction.svg
swarm2/bin/check-docs.py
```

## Not done in this pass

- AEP `workflows/*.yaml` documents derived from the lifecycles (state names are kept verbatim so the derivation is 1:1).
- Registering this specification as an `executable-system-specification` artifact in `.engineering/planning` (the store has one writer: the coordinator).
- `cli:` blocks for the five CLI-reached components, from `board.sh`/`eventlog.sh`/`wakegate.sh` `--help`.
- `external:` failing branches so `ess verify conform synthesize` writes the suite; then `ess verify conform run` and evidence.
- A metaharness step map for the flow engine.

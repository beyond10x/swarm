# two-agents — the checker that reads a log and reports each clause by number

The recorded `two-agents-proof` demonstration retired the earlier finding that the repository
had shown only a swarm manager. `story:a-recorded-run-shows-two-agents-working` defines its
**seven acceptance clauses**. The evidence demonstrates one coordinator and one worker; broader
or recursive swarms remain undemonstrated.

The Rust `swarm-check` workspace binary reads a finished swarm's
records and answers each of the seven **independently, by number**, so a run that clears six names
the one it missed and the re-run targets that clause rather than starting again. At two concurrent
`claude-opus-5` sessions, *"it didn't work"* is not an affordable report.

```console
cargo run -p swarm-check -- data/swarms/<slug>
cargo run -p swarm-check -- <slug> --data data/swarms --json
```

Exit **0** when every one of the seven is met; **1** otherwise. The story's *unattended*
condition is reported beside them and decides nothing — see below.

SQLite is opened read-only, with no read-write fallback. A missing or corrupt SQLite database,
or an inaccessible evidence file, produces a failing verdict; the checker never creates a database.
Malformed JSON lines are skipped, preserving the historical reader's behavior. This replaces the
Python command; historical exports retain the command and verdict recorded at the time.

## What it reads

| path | what the checker takes from it |
|---|---|
| `data/swarms/<slug>/eventlog.sqlite3` | `swarm_events` — `global_seq, event_name, actor, subject, occurred_at, data` |
| `data/swarms/<slug>/turns/*.jsonl` | the metaharness event records, one file per turn **attempt** |
| `data/swarms/<slug>/turns/spend.jsonl` | one row per attempt: `agent`, `goal`, `iterations`, `reached`, `spent` |
| `data/swarms/<slug>/turns/capped.jsonl` | if the runtime ever writes one — see clause 7 |

## The seven, and what each one needs to be met

| # | clause | met when |
|---|---|---|
| 1 | a non-Coordinator agent exists | a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator` |
| 2 | work was handed over | a `swarm.agent.AssignmentPosted` naming that agent, a **later** `swarm.agent.AssignmentTaken` by the same agent, and an issuer on that taking that is the runtime or that agent — never an operator, another agent, or a name the log never spawned |
| 3 | two transcripts, neither overwritten | two turn files attributable to two different agents, and a file for every attempt `spend.jsonl` claims |
| 4 | the work was finished | an `AssignmentDone` for the assignment clause 2 matched, or a `GateGreen` **by the agent that took it** — a coordinator finishing its own goal is not this clause |
| 5 | the confinement acted | a `tool.decided` whose decision is a denial and whose `decided_by` is `frame`. A session census is **not** this: `by_decider` counts decisions of every outcome |
| 6 | spend is attributed per agent | rows naming two different agents, both of them **spawned** |
| 7 | the bound held | a recorded refusal by the **agent** ceiling naming one of at least two **spawned** agents |

Two rules the checker obeys, and a third that decides the design:

- **A clause it cannot evaluate is NOT MET, never skipped.** An absent event log, an unreadable turn
  file, a missing spend record — each is a clause that has not been shown.
- **No clause passes by finding nothing.** An empty result is not evidence. Every clause is met only by a positive count.
- **An agent is what an `AgentSpawned` says it is, and nothing else says it.** A name on a spend
  row, a name a turn record gives itself, a name in a file name: each is a claim the log settles.
  One definition, used by attribution and by clauses 6 and 7 alike — there were two for a day, and
  they gave one set of records opposite answers about whether `ghost` was an agent.
- **The unattended condition needs positive evidence, and decides nothing either way.** The story
  asks for "no operator input between the swarm starting and a verdict being recorded", and it is
  met when every event in that window names who issued it and every one of those is the runtime or
  an agent the log spawned. The window runs from the `swarm.manager.SwarmStarted` that opens it to
  the last `GoalReached`/`GoalNotReached` that closes it — **to the verdict, as the story says**,
  because an operator's next command after a run has finished is not a hand on the run. The opener
  itself is excepted: a person starts a swarm by construction, and counting it would make the
  condition unmeetable.

- **An agent is what an `AgentSpawned` says it is — in the issuer column too.** A hand that puts
  any string in its request's `agent` field is not thereby an agent: `agent:ghost` in a log that
  spawned no `ghost` names nobody, and a claim with nothing behind it is not evidence that nobody
  touched the swarm. This is not authentication, which the story excludes — nothing proves the
  claim came from that agent. It is the difference between a record a reader can check and one
  anybody can write about anybody.

  It has been wrong twice, in opposite directions, and the rule is what is left. It **could not
  fail** while `store.rs` wrote `actor.unwrap_or("system")` into the `subject` and `actor` columns
  both and `http.rs` made the actor optional: `dsfsdf`, the swarm the story names as hand-driven,
  has 27 `swarm.manager.*` lifecycle events inside its window, every one of them actor `system`,
  and it passed. Then it **could not pass**, reporting UNDETERMINABLE — honest while the record
  could not carry the fact, and dishonest the moment `story:an-event-cannot-say-which-agent-acted`
  put the issuer on the envelope on 2026-09-13.

- **A log written before that closed is reported as such, not read as though it answered.** Its
  `subject` column holds a second copy of the actor type, so no event in it names an issuer.
  Clause 2 says in its own output that it is not checking the taking's issuer; the unattended
  condition is NOT MET, because the absence of a hand cannot be read off a record that never
  recorded the presence of one. `data/swarms/two-agents-proof` — the demonstration's own log — is
  one of these, and it answers *not unattended* for that reason, with `swarm.goal.Operator` and
  `swarm.manager.Operator` in its actor column as corroboration. Its seven clauses are unchanged.

## How a turn file is attributed to an agent

Four ways, in the order a reader would trust them: an `agent` field on a record in the file; the
agent's name in the file name; a work directory named after the agent in `session.started.cwd`; and
the `(iterations, goal-prefix)` join into `spend.jsonl` — which is the same join `Swarm::turns` uses
to put a verdict against a transcript. The parser accepts historical names without an attempt,
names with an attempt, and current names that also carry an agent segment. The spend join preserves
the original checker's attribution for the recorded demonstration.

**All four check the name against `swarm.agent.AgentSpawned`.** A transcript that names itself
`alice` in a swarm that spawned no `alice` is not attributed, and two such transcripts are not two
agents.

"Neither overwritten" is read the same way, and the accounting starts from the **spend rows'** keys
rather than the files'. An attempt is a row in `spend.jsonl`; a transcript is a file; fewer files
than rows for one turn of one goal means a record was replaced — the defect the attempt suffix was
added to prevent on 2026-09-12. Keying on the files instead asks only "which attempts left a file
that is missing a sibling" and silently passes a turn whose every attempt vanished, which is the
shape `data/swarms/e2e-1789212621/` actually has: `spend.jsonl` claims iterations 1, 2, 3 and 4 and
only 3 and 4 left a transcript.

## What no run can meet yet, as of 2026-09-13

This historical comparison describes the base before the 2026-09-13a wave. The merged runtime
can provide evidence for every clause; whether it does is a question about a recorded run.

Five clauses were unmeetable on the base commit `1da5334`, and every one of them is a thing another
unit of this wave built:

- **Clause 1, clause 2 and clause 4 — a second agent.** `swarm.agent.Role` on the base declares
  one variant, `Coordinator`, so `AgentSpawned.role` cannot say anything else and *"a
  non-Coordinator agent exists"* is unsatisfiable. Measured across all 11 logs under
  `data/swarms/`: the swarm `mail-check-1789197540` spawned `coordinator`, `reviewer`, `builder`
  and `tester`, and **every one of them carries role `Coordinator`**, because there was nothing
  else to give them. With no
  non-Coordinator agent there is no agent to hand work to (clause 2) and nobody to finish it
  (clause 4). The wave's other unit adds `Worker` to that enum, which is why this directory's
  fixtures spawn one.
- **Clause 5 — the frame.** The base coordinator launches `--decisions observe`
  (`coordinator.rs:560`), which allows every tool call and records it: no frame, so no refusal to
  find. The wave replaces it with `--decisions frame --frame <file>`.
- **Clause 7 — a durable ceiling refusal.** The base agent bound fires (`trigger::bounded` reads
  `spend_by_agent`) but publishes only to the watch stream and the tracing log
  (`trigger.rs:288`), both of which are gone with the process, so a finished swarm's records cannot
  show it. The wave writes `turns/capped.jsonl`.

The historical Python `test_adversary_pass2.py` supplied the domain-fixture guard. Its present
Rust replacement succeeds against the integrated domain and compares fixture roles against
`src/core/domains/agent.yaml`, and the documentation guard that requires this section to name
the clauses that an all-Coordinator fixture cannot meet.

The clause 7 reader accepts three shapes, so it reads the merged runtime's rows without another
change: a `turns/capped.jsonl` row whose `bound` is `"agent"` beside an `agent` field — which is
exactly what `Swarm::record_capped` writes — or `bound: {"agent": "<id>"}`, or the `Capped::why()`
sentence on a spend row's `note` or in an event payload.

## Testing it needs no swarm

The Rust suite materializes stable synthetic SQLite and JSON-lines fixtures. It launches no agent,
model or daemon and needs no Python setup.

```console
cargo test -p swarm-check
cargo run -p swarm-check --example fixtures -- <new-directory>
cargo run -p swarm-check --example fixtures -- <new-directory> --unmet 4
cargo run -p swarm-check -- <new-directory> --json
```

The fixture command refuses an existing destination. Its default meets all seven clauses;
`--unmet` removes a selected fact, with clause 4 also failing when clause 2 has no handover.

[The regression mapping](regression-mapping.md) maps all 83 original cases, including all original
subcases and adversary findings, to Rust tests. Behavioral cases compare every parsed JSON field,
CLI exit status, and text clause titles/reasons against captured Python baseline outputs. The Rust
fixture materializer owns setup; the immutable JSON corpus preserves the baseline's evidence.
Vocabulary guards read the runtime's actual `Issuer` enum and label arms and fail by variant name
when a unit, tuple or struct variant cannot be understood. Additional tests cover malformed input,
legacy names, directory/slug invocation and inaccessible evidence.

## Historical demonstration

[The exported evidence](evidence/2026-09-13-two-agents-proof/README.md) preserves the original
run claims and checker output. The export omits SQLite and original transcripts, so it cannot
reproduce the complete verdict by itself. Checking the original demonstration requires its
`data/swarms/two-agents-proof` directory. The portable test corpus is synthetic evidence and is
kept separate from that historical input.

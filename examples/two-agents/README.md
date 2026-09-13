# two-agents — the checker that reads a log and reports each clause by number

`AGENTS.md` § *Honesty rules for anything published* carries a standing finding: *"a working agent
swarm is overstated. What works is a swarm manager: one coordinator per swarm, one goal, one loop."*
`story:a-recorded-run-shows-two-agents-working` is the story that retires that sentence or proves it
should stay, and its acceptance is **seven clauses**.

This directory holds the instrument, not the run. `check-two-agents.py` reads a finished swarm's
records and answers each of the seven **independently, by number**, so a run that clears six names
the one it missed and the re-run targets that clause rather than starting again. At two concurrent
`claude-opus-5` sessions, *"it didn't work"* is not an affordable report.

```console
examples/two-agents/check-two-agents.py data/swarms/<slug>
examples/two-agents/check-two-agents.py <slug> --data data/swarms --json
```

Exit **0** when every one of the seven is met; **1** otherwise. The story's *unattended*
condition is reported beside them and decides nothing — see below.

## What it reads

| path | what the checker takes from it |
|---|---|
| `data/swarms/<slug>/eventlog.sqlite3` | `swarm_events` — `global_seq, event_name, actor, occurred_at, data` |
| `data/swarms/<slug>/turns/*.jsonl` | the metaharness event records, one file per turn **attempt** |
| `data/swarms/<slug>/turns/spend.jsonl` | one row per attempt: `agent`, `goal`, `iterations`, `reached`, `spent` |
| `data/swarms/<slug>/turns/capped.jsonl` | if the runtime ever writes one — see clause 7 |

## The seven, and what each one needs to be met

| # | clause | met when |
|---|---|---|
| 1 | a non-Coordinator agent exists | a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator` |
| 2 | work was handed over | a `swarm.agent.AssignmentPosted` naming that agent, and a **later** `swarm.agent.AssignmentTaken` by the same agent |
| 3 | two transcripts, neither overwritten | two turn files attributable to two different agents, and a file for every attempt `spend.jsonl` claims |
| 4 | the work was finished | an `AssignmentDone` for the assignment clause 2 matched, or a `GateGreen` **by the agent that took it** — a coordinator finishing its own goal is not this clause |
| 5 | the confinement acted | a `tool.decided` whose decision is a denial and whose `decided_by` is `frame`. A session census is **not** this: `by_decider` counts decisions of every outcome |
| 6 | spend is attributed per agent | rows naming two different agents, both of them **spawned** |
| 7 | the bound held | a recorded refusal by the **agent** ceiling naming one of at least two **spawned** agents |

Two rules the checker obeys, and a third that decides the design:

- **A clause it cannot evaluate is NOT MET, never skipped.** An absent event log, an unreadable turn
  file, a missing spend record — each is a clause that has not been shown.
- **No clause passes by finding nothing.** `swarm.agent.AssignmentTaken` has fired zero times in
  this repository's history, so an empty result is the default state of the world. Every clause is
  met only by a positive count.
- **An agent is what an `AgentSpawned` says it is, and nothing else says it.** A name on a spend
  row, a name a turn record gives itself, a name in a file name: each is a claim the log settles.
  One definition, used by attribution and by clauses 6 and 7 alike — there were two for a day, and
  they gave one set of records opposite answers about whether `ghost` was an agent.
- **The unattended condition is UNDETERMINABLE and decides nothing.** The story asks for "no
  operator input between the swarm starting and a verdict being recorded" — and this log cannot
  say. `src/runtime/ess-runtime/src/store.rs:192` writes `actor.unwrap_or("system")` and
  `src/runtime/swarm-server/src/http.rs:38` makes the actor optional, so an operator pausing a
  swarm through the HTTP surface leaves an event indistinguishable from one the loop wrote.
  Measured: `dsfsdf`, the swarm the story itself names as hand-driven, has 27 `swarm.manager.*`
  lifecycle events inside its window, every one of them actor `system`. An earlier revision of
  this checker read that as *unattended met* and let it decide the exit status. It now reports
  the actors as information, says why it cannot answer, and answers nothing. When
  `story:an-event-cannot-say-which-agent-acted` closes, it becomes checkable and comes back.

## How a turn file is attributed to an agent

Four ways, in the order a reader would trust them: an `agent` field on a record in the file; the
agent's name in the file name; a work directory named after the agent in `session.started.cwd`; and
the `(iterations, goal-prefix)` join into `spend.jsonl` — which is the same join `Swarm::turns` uses
to put a verdict against a transcript. **The fourth is the one that works today**: `turn_file`
(`src/runtime/swarm-server/src/coordinator.rs:608`) names a file after the turn and the goal's first
eight characters, and nothing in the name or in the metaharness record says which agent ran it.

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

Read this before spending money on a run. **This section describes the tree after the 2026-09-13a
wave merges**, and says where that differs from the branch this file was written on, because the two
halves of the wave landed on different branches and a reader of either alone gets the wrong answer.

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

**After the merge, nothing here is structurally unmeetable** — every clause becomes a question about
the run rather than about the runtime. Before it, on this branch alone, clause 1, clause 2 and
clause 4 are reported *not met* against any real log, and so are clause 5 and clause 7. That is the
honest verdict rather than a hole in the checker, and it is why `test_adversary_pass2.py`'s first
case is red here and green on the merged tree: it reads `swarm.agent.Role` out of
`src/core/domains/agent.yaml` in whichever tree it is run in. Measured, not assumed — that case and
the other four pass, 5 of 5, against a tree carrying unit A's `agent.yaml`.

The clause 7 reader accepts three shapes, so it reads the merged runtime's rows without another
change: a `turns/capped.jsonl` row whose `bound` is `"agent"` beside an `agent` field — which is
exactly what `Swarm::record_capped` writes — or `bound: {"agent": "<id>"}`, or the `Capped::why()`
sentence on a spend row's `note` or in an event payload.

## Testing it needs no swarm

A real run costs real money: one turn on `claude-opus-5[1m]` cost $0.070 on 2026-09-13, and
`--max-budget-usd` is not a pre-spend bound — a probe capped at $0.01 ended at $0.071. So the
fixtures write the records a run would leave, from nothing, and every clause is exercised **both
ways**.

```console
python3 -m unittest discover -s examples/two-agents          # the suite; exit 0 when green
python3 examples/two-agents/fixtures.py <dir>                # a swarm meeting all seven
python3 examples/two-agents/fixtures.py <dir> --unmet 4      # the same, minus one fact
python3 examples/two-agents/check-two-agents.py <dir>
```

`python3 -m unittest discover -s examples/two-agents` is the command a gate step should run. It
needs no dependency beyond the standard library — `sqlite3` and `json` are in it — which is why this
is Python and not a fourth crate in the workspace.

Three files, and the third is not the author's:

| file | what it holds |
|---|---|
| `fixtures.py` | the records a run would leave, written from nothing. `build()` with no overrides meets all seven; each keyword takes one clause the other way |
| `test_checker.py` | every clause both ways, plus the enumerations: nothing passes by finding nothing, no clause about agents is met by a name the log never spawned, no clause is met by a tally instead of a record |
| `test_adversary.py` | **the adversary's cases, and not the author's to edit.** Five records whose shapes were read off real `data/swarms/*` — the ones that showed this checker reporting *met* on evidence that establishes nothing |

The suite was first run against a deliberately wrong checker that answered "met" to everything: 42
of 45 cases fired. That instrument finds a checker that is too lax about *absence*. It cannot find a
checker that is too lax about *evidence*, which is what `test_adversary.py` is for and what every
fix in it was.

## What is not here

**The scenario.** The story's other half is a swarm, a goal small enough to finish, and an
assignment the coordinator posts to a second agent. That run is evidence and is recorded as a
`verification-report` afterwards; it is not this directory. If the run does not meet the acceptance,
the honest outcome is a report saying so and the sentence in `AGENTS.md` stays.

---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13a-unit-b-pass-1
kind: review-result
status: active
title: Adversary, wave 13a unit B, pass 1
relations:
- reviews: story:a-recorded-run-shows-two-agents-working
revision: 1
---
Adversary, wave 2026-09-13a unit B, pass 1. Against the checker half of
`story:a-recorded-run-shows-two-agents-working`, working tree
`/home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260913a-b`, untracked over base `1da5334`.

## Header, as returned

```
unit: b — examples/two-agents/ (checker + fixtures + suite)
verdict: NEEDS-CHANGE
cases: executed 45→50, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
clauses: 3, 5, 7 report met on evidence that does not establish them; the unattended condition never fails on a log this runtime produces
wrote-outside-worktree: 7 paths under the assigned scratch directory
needs-coordinator: no
```

Five cases added in `examples/two-agents/test_adversary.py`, the adversary's only written path. No
implementation file touched. `<before>` = 45 measured by deselecting that file: `Ran 45 tests … OK`,
exit 0.

## Findings

**F1 — `check-two-agents.py:712` — the unattended condition cannot fail on any log this runtime
produces.** `machines = {"system"} | set(records.spawned)`. But `src/runtime/ess-runtime/src/store.rs:192`
writes `actor.unwrap_or("system")` and `src/runtime/swarm-server/src/http.rs:38` makes `actor` an
**optional** request-body field, so every operator command is recorded as `system`.
*Measured on `data/swarms/dsfsdf/eventlog.sqlite3`*: inside the window from the first `SwarmStarted`
there are 8 `SwarmPaused`, 4 `SwarmResumed`, 7 `SwarmStopped`, 6 further `SwarmStarted` and 1
`SwarmDeleted` — **26 operator commands, every one actor `system`** — and the checker answers
`unattended met: True`. `dsfsdf` is the swarm the story itself names as hand-driven. Every actor in
every real log is `system`, `swarm.goal.Coordinator` or `swarm.mailbox.SwarmAgent`; all three pass.
`Report.ok` (`:147`) makes this verdict the exit status.
NEEDS-CHANGE / introduced, blocker.

**F2 — `:419-424` — clause 3 implements a weaker rule than its own sentence.** `rows_per_key` only
increments for keys already in `files_per_key`, so a spend row whose turn number has **no** file
matches nothing and is dropped. Clause 3's own `looked_for` says "a file for every attempt the spend
record claims"; the code implements "fewer files than rows for one turn that has a file".
*What reaches it*: `data/swarms/e2e-1789212621/turns/` is exactly this shape — `spend.jsonl` claims
iterations 1, 2, 3, 4 and the only files are `0003-01-…` and `0004-01-…`.
NEEDS-CHANGE / introduced, blocker.

**F3 — `:379-382` — `agent_of`'s record path trusts a name the log never spawned.** It returns any
non-empty `agent` string unchecked, unlike its other three paths (`:383`, `:386`) and unlike
clause 6 (`:599`), which all require `in known`. Two files naming `alice` and `bob` meet clause 3 on
a swarm with zero `AgentSpawned`.
*What reaches it*: **nothing found** — all 47 real turn files across 10 swarms carry no top-level
`agent` key. Speculative today, and also the mechanism for the same-agent-under-two-spellings attack,
since clause 3 never intersects its agent set with `records.spawned`.
CONFIRMED / introduced, warning.

**F4 — `:190`, `:668` — the checker contradicts itself about who is an agent, on one set of
records.** `known_agents = spawned | spend_agents`, and clause 7 counts `len(known) >= 2`. On one
swarm it prints, for clause 6, "named by no `swarm.agent.AgentSpawned`: `ghost`" and, for clause 7,
"2 agent(s) to bound: coordinator, ghost; refused: `ghost`".
*What reaches it*: **nothing found** — real spend rows name `null` or `coordinator`. It is the mutant
the suite does not catch: delete the `set(records.spawned)` half of `known_agents` and 45 cases stay
green, because `test_not_met_when_there_was_only_one_agent_to_bound` removes the second agent from
`roles` and `spend` together.
CONFIRMED / introduced, warning.

**F5 — `:536` — the clause 5 census fallback misreads the schema.**
`(census.get("by_decider") or {}).get("frame")` is read as "the frame denied something", but the real
census counts decisions of **any** outcome: `data/swarms/dsfsdf/turns/0035-77fc1fcc.jsonl` shows
`{"allowed":5,"denied":0,…,"by_decider":{"observe":5}}`. `fixtures.py:186` writes
`{"by_decider": {refusal: 1}}` — only the denier — which is the unit's idea of the schema and not the
schema, and it is the only thing keeping `test_not_met_when_the_denial_did_not_come_from_the_frame`
green.
*What reaches it*: **nothing found, and the adversary says so because it built the state** — all 254
real `decided_by` values are `observe`, and the frame probe's census was `by_decider: {"frame": 1}`
alone. Under a single-decider config the predicate coincides with the truth by accident.
INFEASIBLE / introduced, warning. The fixture is wrong against the real schema either way.

**F6 — `:331-338` — clause 2 cannot establish "taken by that agent", and no checker can.**
It rests on `data.agent_id` alone. `apply.rs:174-192` (`permitted`) matches the actor against the
specification's *actor types*, not agent instances, so the `actor` column is
`swarm.agent.SwarmAgent` for any caller, and `agent.yaml:434` makes `TakeAssignment`'s `agent_id`
caller-supplied input.
*What reaches it*: the `/command` HTTP endpoint takes any command with an optional actor. A
coordinator, or an operator's curl, issuing `take-assignment {agent_id: builder}` is
indistinguishable in the record from the builder taking it. **A story/spec gap, not a checker bug** —
but the checker should not claim it does.
CONFIRMED / pre-existing, note.

**F7 — `:468` — clause 4 accepts a coordinator finishing its own goal.** It counts any
`AssignmentDone`/`GateGreen` anywhere in the log, with no link to the assignment clause 2 matched and
no check on the emitting agent. The story's clause 4 text literally permits it; read beside clause 2
it does not, and it is the single-agent status quo the story exists to disprove.
CONFIRMED / introduced, note.

## Named fixes, not applied

1. Machine actors must be the swarm's own actor types plus spawned agents; `system` alone cannot mean
   unattended — or the condition needs a different signal entirely and should stop deciding the exit
   status until it has one.
2. Build `rows_per_key` from the spend rows' own keys, not from `files_per_key`, and report
   `rows > files` including `files == 0`.
3. Gate `agent_of`'s record path on `in records.known_agents` like the other three.
4. Use `set(records.spawned)` for clause 7's "agents to bound", matching clause 6.
5. Drop the census fallback, or read a `denied_by` map if metaharness grows one — `by_decider` is not it.

## The unit's own document, driven against its code

`README.md` asserts the unattended condition "does decide the exit status — a swarm somebody drove by
hand has not demonstrated anything", and caveats only "a person issuing commands through the HTTP
surface **under a specification actor's name**". The measured failure mode is larger by the whole of
it: the HTTP actor field is optional and defaults to `system`. The same README says clause 3 needs "a
file for every attempt `spend.jsonl` claims" one sentence before describing the weaker rule the code
implements. Both halves were written by the unit, and nothing else compared them.

## Attacked, could not break

- **The empty-log guard is real.** `test_nothing_passes_by_finding_nothing` iterates
  `report.clauses + [report.unattended]` dynamically, and `check()`'s `assert [numbers] == [1..7]`
  (`:745`) means an eighth clause trips the assert rather than slipping past.
- Clause 2 rejects a take that precedes its post, a take by the coordinator, and a post to a
  Coordinator-role agent.
- Clause 6 cannot be met by one agent named twice — `spend_agents` is a `Counter`.
- The `(iterations, goal-prefix)` join cannot split one agent across two; a collision under-counts
  and fails clause 3 safe.
- `TURN_FILE`/`LEGACY_TURN_FILE` match all 47 real turn file names across 10 swarms; the fixture
  `SCHEMA` matches the real `swarm_events` DDL; `read_events` opens `mode=ro` first, so it does not
  checkpoint a live WAL.

```findings
- file: examples/two-agents/check-two-agents.py
  line: 712
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The unattended condition counts bare actor `system` as a machine, but this runtime writes `system` for every operator command (store.rs:192 defaults it, http.rs:38 makes it optional), so data/swarms/dsfsdf reports unattended met with 26 operator pause/resume/stop commands inside the window, and that verdict decides the exit status."
- file: examples/two-agents/check-two-agents.py
  line: 419
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Clause 3 promises `a file for every attempt the spend record claims` but counts spend rows only against turn keys that already have a file, so an attempt whose turn left no transcript at all is dropped silently; data/swarms/e2e-1789212621 has exactly that shape with spend rows 1-4 and files only for 3 and 4."
- file: examples/two-agents/check-two-agents.py
  line: 379
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "`agent_of`'s record path returns any non-empty `agent` string without checking it against the log, unlike its other three paths and unlike clause 6, so two turn files naming `alice` and `bob` meet clause 3 on a swarm with zero AgentSpawned events; no real turn record carries that key today, so nothing reaches it yet."
- file: examples/two-agents/check-two-agents.py
  line: 668
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Clause 7 counts spend-named strings as `agents the swarm had to bound` while clause 6 rejects the same string as named by no AgentSpawned, so one set of records gets both verdicts at once; the existing single-agent test removes the second agent from roles and spend together and never exercises the spend half of known_agents."
- file: examples/two-agents/check-two-agents.py
  line: 536
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "The clause 5 census fallback reads `by_decider[frame]` as a frame denial, but the real census counts decisions of any outcome (dsfsdf turn 0035 shows `allowed:5, denied:0, by_decider:{observe:5}`), so a session where the frame only allowed and another decider denied reports a phantom frame refusal; fixtures.py:186 writes only the denier and is what keeps the negative test green, and no recorded run mixes deciders so I could not show anybody reaches the mixed state."
- file: examples/two-agents/check-two-agents.py
  line: 331
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "Clause 2 establishes `taken by that agent` from the caller-supplied `agent_id` alone, and it cannot do better because apply.rs:174 permits actors by spec actor type rather than agent instance, so an AssignmentTaken issued by the coordinator or by an operator's curl is indistinguishable in the record from the agent taking it."
- file: examples/two-agents/check-two-agents.py
  line: 468
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Clause 4 accepts any AssignmentDone or GateGreen anywhere in the log with no link to the assignment clause 2 matched and no check on the emitting agent, so a coordinator finishing its own goal meets `the work was handed over and finished` - which is the single-agent status quo the story exists to disprove."
```

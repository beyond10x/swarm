---
format: aep.planning-md/1
id: story:a-recorded-run-shows-two-agents-working
kind: story
status: implemented
title: A reproducible demonstration, and a checker that reads the log
summary: 'The demonstration AGENTS.md says does not exist: one log, two agents, a refusal, a finished assignment.'
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
- depends_on: story:spawn-a-second-agent
- depends_on: story:a-turn-is-confined-by-a-frame
- verifies: story:spawn-a-second-agent
- verifies: story:a-turn-is-confined-by-a-frame
- depends_on: story:an-event-cannot-say-which-agent-acted
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
revision: 12
---
## What

`AGENTS.md` § *Honesty rules for anything published* carries a standing finding: *"a working agent
swarm is overstated. What works is a swarm manager: one coordinator per swarm, one goal, one loop.
Multi-agent operation — `swarm.agent.Spawn`, `Assign`, the whole mailbox domain — is specified and
not demonstrated."*

This story builds the thing that retires that sentence or proves it should stay: **a demonstration
anybody can re-run, and a checker that reads a log and says which clauses it met.**

## This story builds something; the run is its evidence

The design critic was right that a one-off run is evidence rather than epic-work, and the correction
is not to file the run — it is to stop treating a demonstration as a thing one person does once. A
proof that exists only as a transcript somebody watched is exactly the "specified and not
demonstrated" problem one level up.

So the deliverable is two files, not a memory:

- **a scenario** under `examples/` — a swarm, a goal small enough to finish and checkable enough
  that a verdict means something, and an assignment the coordinator posts to a second agent;
- **a checker** that reads `data/swarms/<slug>/eventlog.sqlite3` and
  `data/swarms/<slug>/turns/spend.jsonl` and reports each acceptance clause below as met or not met,
  **by number**.

The run itself is then evidence: recorded as a `verification-report`, which is what this story
`verifies` against the two it depends on. The report comes after the run, and cannot be written
before it.

## Acceptance

Seven clauses. **The checker reports each one independently**, so a run that clears six names the one
it missed and the re-run targets that clause rather than starting again. A real run costs real money
— two concurrent `claude-opus-5` sessions — and "it didn't work" is not an affordable report.

1. **A non-Coordinator agent exists.** An `AgentSpawned` whose role is not `Coordinator`.
2. **Work was handed over.** An `AssignmentPosted` to that agent, and a later `AssignmentTaken`
   naming it. **What this clause can and cannot establish**, measured 2026-09-13
   (`review-result:adversary-2026-09-13a-unit-b-pass-1`, finding 6): it establishes that the record
   *says* the agent took it, and it **cannot** establish that the agent, rather than the coordinator
   or an operator's curl, issued the command — `apply.rs:174` permits actors by specification actor
   *type*, not agent instance, and `TakeAssignment`'s `agent_id` is caller-supplied input. The
   checker must say so in its own output rather than implying more.
   `story:an-event-cannot-say-which-agent-acted` is the gap and is filed.
3. **Two transcripts, neither overwritten.** A turn file for every attempt `spend.jsonl` claims, and
   those files attributed to two different agents. An attempt with no file beside it is a record that
   was overwritten, and counts as not met.
4. **The work was finished by the agent that took it.** A `FinishAssignment` or `GateGreen` **for the
   assignment clause 2 matched, emitted by the agent that took it.** Tightened 2026-09-13 from "a
   `FinishAssignment` or a `GateGreen`" anywhere in the log, which the adversary showed a coordinator
   finishing its own goal satisfies — the single-agent status quo this story exists to disprove.
5. **The confinement acted.** At least one tool call refused **by the frame**, in the turn record — a
   refusal that happened, not a flag that was set, and not another decider's denial. Read it from the
   run's own refusal record; `census.by_decider` counts decisions of every outcome and is not a
   denial map.
6. **Spend is attributed per agent.** Spend rows exist for two agents the log **spawned**, each
   naming its own agent.
7. **The bound held with more than one agent to bound.** The agent ceiling refused one of two
   **spawned** agents, and the refusal is in a record a reader finds after the run. Separate from
   clause 6 because per-agent recording and per-agent enforcement fail independently. Note
   2026-09-13: `report_capped` currently publishes only to the watch stream and the tracing log, so
   nothing durable records a refusal; that is runtime work, not checker work.

Clauses 1–4, 6 and 7 rest on `story:spawn-a-second-agent`; clause 5 rests on
`story:a-turn-is-confined-by-a-frame`. Clause 7 additionally needs `spawn-a-second-agent`'s clause 4.

**Unattended, operationally.** No operator input between the swarm starting and a verdict being
recorded. **This is not yet checkable and the checker must not pretend it is**: `store.rs:192` writes
`actor.unwrap_or("system")` and `http.rs:38` makes the actor optional, so an operator's command and
the runtime's own record identically as `system`. Measured: `dsfsdf`, the swarm this story names as
hand-driven, contains 26 operator pause/resume/stop commands inside its window and every one is
`system`. Until `story:an-event-cannot-say-which-agent-acted` closes, the checker reports the
condition as **undeterminable** and says why, and it does **not** decide the exit status on it.

**Budget, measured.** `--max-budget-usd` is **not** a pre-spend bound: a probe on 2026-09-13 launched
with `--max-budget-usd 0.01` ended at `total_cost_usd 0.0710`, seven times the cap. One turn on
`claude-opus-5[1m]` cost $0.070. Plan against that ratio, not against the flag. The same probe
reported the operator's seven-day rate-limit window at 76% utilisation.

## What it costs

Real money. Budget before starting: since the 2026-09-12d wave, `SWARM_MAX_TURNS` and
`SWARM_MAX_SPEND_USD` bound one agent across every goal it works. `dsfsdf` spent 38 turns and $10+
discovering its goal string was the placeholder `2342342`; pick the goal with that in mind.

## Then correct what is published

`README.md`, `website/` and `AGENTS.md` describe the swarm in terms this run either earns or refutes.
Whichever way it goes, the prose changes to match and `npm run spec:check` in `website` stays green.
**If the run does not meet the acceptance, the honest outcome is a `verification-report` saying so
and the sentence in `AGENTS.md` stays.** That is a result, not a failure of this story.

`website/src/data/spec-facts.json` is **not** this story's surface and was wrongly recorded as one.
It is regenerated by `website/scripts/spec-facts.mjs` and never hand-edited; what would make it stale
is `src/core/domains/agent.yaml` (`story:spawn-a-second-agent`) and `src/core/domains/config.yaml`
(`story:a-turn-is-confined-by-a-frame`). Step 11 of the gate catches it.

## Out of scope

Agents spawning agents; more than two agents; a swarm that extends its own specification. Depth one,
breadth two.

## Scope

- **Files:** `examples/` — the scenario, new; **cited**, this story creates it.
- **Files:** the checker, new, beside the scenario; **cited**.
- **Files:** `README.md`, `AGENTS.md` — the published claims this run settles; **cited**.
- **Not this story's surface:** `website/src/data/spec-facts.json`, derived. Removed from scope.
- **Would collide with:** nothing in the current backlog. It creates files and edits two prose
  documents no other draft names.

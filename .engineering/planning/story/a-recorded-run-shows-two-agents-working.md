---
format: aep.planning-md/1
id: story:a-recorded-run-shows-two-agents-working
kind: story
status: draft
title: A reproducible demonstration, and a checker that reads the log
summary: 'The demonstration AGENTS.md says does not exist: one log, two agents, a refusal, a finished assignment.'
relations:
- decomposes: epic:executable-boxes
- serves: vision:swarm-builds-itself
- depends_on: story:spawn-a-second-agent
- depends_on: story:a-turn-is-confined-by-a-frame
- verifies: story:spawn-a-second-agent
- verifies: story:a-turn-is-confined-by-a-frame
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
revision: 8
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
2. **Work was handed over.** An `AssignmentPosted` and the matching `AssignmentTaken` by that agent.
   `AssignmentTaken` has fired zero times in this repository's history.
3. **Two transcripts, neither overwritten.** Two turn files under `data/swarms/<slug>/turns/`
   attributed to two different agents.
4. **The work was finished, not merely started.** A `FinishAssignment` or a `GateGreen`.
5. **The confinement acted.** At least one tool call refused by the frame, in the turn record — a
   refusal that happened, not a flag that was set.
6. **Spend is attributed per agent.** Spend rows exist for both agents, each naming its own agent.
7. **The bound held with more than one agent to bound.** The agent ceiling refused one of them.
   Separate from clause 6 because per-agent *recording* and per-agent *enforcement* fail
   independently: the 2026-09-12d wave delivered the fold, and a fold that records correctly while
   nothing refuses is the exact shape of the defect that wave was opened to fix.

Clauses 1–4, 6 and 7 rest on `story:spawn-a-second-agent`; clause 5 rests on
`story:a-turn-is-confined-by-a-frame`, whose own clause 2 is the unproven one. Clause 7 additionally
needs `story:spawn-a-second-agent`'s clause 4 — the in-flight claim keyed on the unit of work —
because two agents on one goal serialise on one claim today.

**Unattended, operationally:** no operator input between the swarm starting and a verdict being
recorded. The checker establishes it from the log — every command in the window carries a
non-human actor — rather than from anybody's recollection.

**Budget, measured.** `--max-budget-usd` is **not** a pre-spend bound: a probe on 2026-09-13 launched
with `--max-budget-usd 0.01` ended at `total_cost_usd 0.0710`, seven times the cap, because the
vendor stops the session once its own estimate crosses the number. One turn on `claude-opus-5[1m]`
cost $0.070. Plan the demonstration's budget against that ratio, not against the flag. The same probe
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

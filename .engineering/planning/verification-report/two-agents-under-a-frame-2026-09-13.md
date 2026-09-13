---
format: aep.planning-md/1
id: verification-report:two-agents-under-a-frame-2026-09-13
kind: verification-report
status: draft
title: 'A swarm ran two agents under a frame: seven of seven'
relations:
- verifies: story:a-recorded-run-shows-two-agents-working
- verifies: story:spawn-a-second-agent
- verifies: story:a-turn-is-confined-by-a-frame
revision: 4
---
## The verdict

**Seven of seven clauses met. `check-two-agents.py` exit 0.** The swarm was `two-agents-proof`, run 2026-09-13 against `main` at `fb482dd` plus one checker fix
described below. Total model spend **$1.49**, against an operator budget of ~$3.

`AGENTS.md` § *Honesty rules for anything published* said: *"a working agent swarm is overstated.
What works is a swarm manager: one coordinator per swarm, one goal, one loop. Multi-agent operation
— `swarm.agent.Spawn`, `Assign`, the whole mailbox domain — is specified and not demonstrated."*
That sentence is now false and the record says why.

Its evidence is exported to `examples/two-agents/evidence/2026-09-13-two-agents-proof/` — the event
log as CSV, the spend rows, the ceiling refusal, the sealed frames and what each agent wrote.
`data/swarms/*/` is runtime state and gitignored, so the swarm itself is on the machine that ran it
and the export is what a reader can check.

## What the log shows

The coordinator was given a goal it could not meet alone: `alpha.txt` written by itself, `beta.txt`
written by a different agent, and a deliberate probe of the frame. It spawned a `Worker`, posted it
an assignment, and waited.

| seq | event | actor |
|---|---|---|
| 6 | `swarm.goal.GoalPursued` | system |
| 7 | `swarm.agent.AgentSpawned` | system |
| 10 | `swarm.agent.AssignmentPosted` | system |
| 13 | **`swarm.agent.AssignmentTaken`** | **`swarm.agent.Worker`** |
| 18 | `swarm.agent.AssignmentDone` | system |
| 20 | `swarm.agent.AgentIdle` | `swarm.agent.Worker` |
| 23 | **`swarm.goal.GoalReached`** | system |

`AssignmentTaken`, `AssignmentDone` and `AgentIdle` had each fired **zero times in this
repository's history** before this run, across eleven swarm logs.

Files on disk, one per agent, in directories that did not exist before this wave:

```
data/swarms/two-agents-proof/work/coordinator/alpha.txt   ALPHA
data/swarms/two-agents-proof/work/worker/beta.txt         BETA
```

## Clause by clause

| # | clause | met by |
|---|---|---|
| 1 | a non-Coordinator agent exists | `worker` (`Worker`), 1 of 2 spawned |
| 2 | work was handed over | posted to `worker` at seq 10, taken by it at seq 13 |
| 3 | two transcripts, neither overwritten | 3 turn files, 2 distinct agents, every attempt has a file |
| 4 | the work was finished | `AssignmentDone` at seq 18 by `worker`, which took it at seq 13 |
| 5 | **the confinement acted** | 1 of 41 decided calls refused **by the frame**: `ToolSearch`, outside the operation vocabulary this step was framed with |
| 6 | spend attributed per agent | 2 rows, `coordinator` ×1, `worker` ×1 |
| 7 | **the bound held** | `capped.jsonl`: `bound: "agent"`, `$1.00 of $0.10`, *"by the agent `coordinator` across every goal it works"* |

Clause 5 is the sandbox acting rather than configured: the coordinator attempted a tool the frame
did not admit and the call did not happen. Clause 7 cost **$0.00** — the ceiling is checked before a
turn launches, so a swarm whose agent has already spent past the cap is refused without a model call.

## What this run does not show

- **The original log cannot establish the unattended condition.** It predates issuer envelopes: its subject column duplicates actor on all 24 events, so the record cannot distinguish an operator from the runtime. The current checker reports unattended NOT MET for that missing attribution. This is not a finding that the initial SetGoal or StartSwarm made the run attended; the checker explicitly permits the opening command.

  **The seven clauses remain seven met, exit 0.** Unattended is reported separately and does not decide that exit status. `5331fe8` enables later runs to carry the necessary attribution but does not retroactively add it to this log.
- **It is not containment.** Clause 5 is refusal at a tool decision seam. The metaharness Claude arm does not apply substrate, write-scope or cgroup containment, and the hermetic-run attestation is not a claim of network isolation.
- **Depth one, breadth two.** One coordinator, one worker, one assignment. Recursive delegation and larger swarms remain undemonstrated.

## One defect this run found, in the checker

The first run reported **clause 3 NOT MET**, with *"turn 1 of goal f409711d has 1 attempt(s) in
spend.jsonl and 0 file(s)"* while the file was plainly there. Cause: unit A's final turn-file name is
`{turn}-{attempt}-{agent}-{unit}.jsonl` and unit B's `TURN_FILE` pattern parsed
`{turn}-{attempt}-{unit}`. The agent segment was added by the same wave, on the branch the checker
could not see.

It is an integration defect between two units that neither could have caught alone, and the checker
failed **closed** — it reported not met rather than passing on a file it could not parse, which is
the property two adversary passes were spent establishing. The pattern now accepts both spellings,
with the older one kept so a pre-wave log stays readable. The checker's own 62 cases stay green.

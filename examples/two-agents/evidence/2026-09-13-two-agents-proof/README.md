# 2026-09-13 — two agents under a frame

The evidence for `verification-report:two-agents-under-a-frame-2026-09-13`, exported here because a
swarm's own runtime state is gitignored: `data/swarms/*/eventlog.sqlite3`, `turns/` and `work/` are
all runtime state and do not belong in the repository. What is here is what a reader needs to check
the claim without having been present.

| file | what it is |
|---|---|
| `events.csv` | every row of `swarm_events`, in order, as the log holds it — including the
`subject` column, which on this log equals `actor` on all 24 rows because the run predates the
issuer landing in `ess-runtime` |
| `spend.jsonl` | one row per turn attempt, carrying the agent that spent it |
| `capped.jsonl` | the ceiling refusal — `bound: "agent"` |
| `*.frame.json` | the sealed frames the three turns actually ran under, digests intact |
| `checker-output.txt` | `check-two-agents.py`'s verdict, seven clauses, exit 0 |
| `alpha.txt`, `beta.txt` | what each agent wrote, from its own work directory |

The transcripts themselves (`turns/*.jsonl`, ~1 MB) are **not** exported: they are the models' full
event streams, they are runtime state, and nothing in the verdict rests on reading them by eye —
the checker reads them and `checker-output.txt` is what it concluded.

**Re-deriving the verdict needs the original swarm**, which is on the machine that ran it and not
here. That is the honest limit of this directory: it lets you read what happened and check the
checker's reasoning, and it does not let you re-run the check. To do that, run a swarm of your own
and point `check-two-agents.py` at it.

## Re-exported 2026-09-13, after the wave that made an event say who acted

`checker-output.txt` originally recorded `unattended  UNDETERMINABLE`. That line is one the checker
can no longer print for any input: `story:an-event-cannot-say-which-agent-acted` gave commands an
issuer, so the condition stopped abstaining and started deciding. On this log it decides **NOT MET**,
and it is right to — the run was started by an operator's `SetGoal` and `StartSwarm`, which are
`swarm.goal.Operator` and `swarm.manager.Operator` in the `actor` column.

**The seven clauses are unchanged: seven met, exit 0.** The unattended condition is reported beside
them and decides nothing, so the verdict this directory was created to evidence has not moved.

Two things worth saying plainly rather than leaving for a reader to notice:

- **This log cannot show an issuer**, because it was written before the runtime recorded one. Its
  `subject` column is a copy of `actor` on every row, which is exactly the signature the checker uses
  to tell a pre-issuer log from a silent one. The export now carries that column so the signature is
  visible off this machine.
- **The demonstration that proved v0.2.0 was recorded by a runtime that could not say who acted.**
  That is not a flaw in the demonstration — every clause it claims is carried by events whose meaning
  does not depend on an issuer — but it is the reason the next demonstration will be better evidence
  than this one.

## `events.csv` carries a harness value the specification does not declare, and it stays

Row 2 of `events.csv` is `swarm.agent.AgentSpawned` with `"harness":"ClaudeCode"`. That is not a
variant of `swarm.config.Harness`, which declares `[Claude, Codex, B10x]`
(`src/core/domains/config.yaml`). It was what `swarm-server` wrote for every agent it spawned, on
every log in this repository's history, and on 2026-09-18 the interpreter was taught to refuse it:
`ess-runtime/src/apply.rs::require_declared_variants` rejects a command input sitting at an enum
position that the enum does not name.

**This file is not edited, and must not be.** It is retained evidence behind
`verification-report:two-agents-under-a-frame-2026-09-13`, and rewriting a row to a value the run
did not produce would make the artifact say something the run did not — a sealed artifact
falsified to flatter a later fix. The value is part of what happened. What this note does instead
is the whole of the correct handling: it records, beside the file, that the value is invalid, why
it was written, and when the writer was corrected.

Nothing in the verdict rests on it. `swarm-check` never reads `harness`; the seven clauses are
carried by event names, sequence and the `actor`/`subject` columns. And the refusal cannot break
this file's replay either: `ess-runtime/src/store.rs::fold` replays a log without going through
`apply`, so a recorded event carrying `"ClaudeCode"` still folds. The refusal is at the writing
seam, where a value enters the system, not at the reading seam, where a value already happened.

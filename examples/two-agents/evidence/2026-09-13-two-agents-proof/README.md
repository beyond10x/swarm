# 2026-09-13 — two agents under a frame

The evidence for `verification-report:two-agents-under-a-frame-2026-09-13`, exported here because a
swarm's own runtime state is gitignored: `data/swarms/*/eventlog.sqlite3`, `turns/` and `work/` are
all runtime state and do not belong in the repository. What is here is what a reader needs to check
the claim without having been present.

| file | what it is |
|---|---|
| `events.csv` | every row of `swarm_events`, in order, as the log holds it |
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

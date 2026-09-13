# Wave proposal — 2026-09-13a — two agents, under a frame

Coordinator: this session. Skill `aep-drive:wave` **0.9.1**. `aep` **0.55.0**, `ess` **0.23.0**,
`metaharness` **0.6.4** installed (source checkout is 0.7.0).
Base `main` at `1da5334`. **Authorised by**: the operator's standing goal, "autonomous, sandboxed
multi-agent swarm proven", set 2026-09-13, and their instruction to use `aep-plan:planning` and
`aep-drive:wave`.

## Why these units, and why one of them is large

`aep plan artifact waves --kind story --status draft` puts `a-turn-is-confined-by-a-frame` in wave 1
and `spawn-a-second-agent` in wave 6, because both cite `coordinator.rs` and the deriver will not
co-schedule them. Both stories' bodies say the same thing in prose and a `depends_on` edge now
encodes it.

**Running them as two waves costs a whole wave of latency to avoid a collision that disappears if one
agent owns the file.** So unit A takes both stories and owns `coordinator.rs` outright. That is
larger than this skill's usual unit and it is the deliberate call: the collision is the binding
constraint, and the goal is the demonstration, not the parallelism.

Unit B is disjoint from A by construction — it writes only under `examples/` and creates a checker.

| unit | stories | surface | worktree |
|---|---|---|---|
| A | `story:a-turn-is-confined-by-a-frame` + `story:spawn-a-second-agent` | `src/core/domains/{agent,config}.yaml`; `swarm-server/src/{coordinator,trigger,swarm,state,budget}.rs` | `wave-20260913a-a` |
| B | the checker half of `story:a-recorded-run-shows-two-agents-working` | `examples/two-agents/**` — new files only | `wave-20260913a-b` |

Integration branch `wave/2026-09-13a/integration`, forked from `main` at `1da5334`.
Agents: `aep-drive:implementor` per unit, then `aep-drive:adversary`.

## What is already measured, so no unit rediscovers it

Two probes, 2026-09-13, $0.224 total, recorded in `story:a-turn-is-confined-by-a-frame`:

- a frame with a wrong digest is refused before any process starts (exit 2, nothing billed);
- a frame admitting only `file.read`, asked to run a shell command, produced
  `census: {denied: 1, by_decider: frame, by_seam: hook}` and the command never ran;
- **the digest cannot be computed by hashing your own JSON** — metaharness hashes its own
  serialisation of the parsed frame. The refusal prints the digest it computed, which is a usable
  fallback and not a shippable mechanism;
- **the frame refuses what is attempted, not what is offered.** All 26 tools stay listed and
  `withheld` is `null`, so the admitted set must be stated in the prompt or every turn pays a billed
  turn to rediscover it. metaharness's own flag for this, `--scope-announce`, is `b10x` only;
- `--max-budget-usd` is not a pre-spend bound: a run capped at $0.01 ended at $0.071.

## Pre-flight

| check | result |
|---|---|
| working tree | clean on `main` at `1da5334` |
| worktrees | three from wave 2026-09-12d, all `finished`, 3.7–3.9M each, retained on `no-remote-recovery-proof` because nothing is pushed. **Assessed, not leftover** — that wave's cleanup ran and was refused for a recorded reason |
| free disk | **90G** |
| build cache | `sccache` present, `RUSTC_WRAPPER` unset, left unset |
| model budget | the probe's `rate_limit` event reported the seven-day window at **76%** utilisation. Two units, no adversary re-runs beyond the two-pass budget |
| `AGENTS.md` | read. Its 11-step gate governs; `aep plan artifact move` is not run by this wave |

## The commits this authorises

The opening commit, one per unit, two merges into the integration branch, the closing store commit,
and the merge into `main` once the gate is green. Not a push, not a tag, not a release.

## Stage

| unit | stage | branch head |
|---|---|---|
| A | not started | — |
| B | not started | — |

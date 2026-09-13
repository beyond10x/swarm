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

---

## The close

**Status: closed 2026-09-13. Both units merged; the 11-step gate plus the checker's own lane are
green on the integration branch, each step's own exit status read separately.**

| step | exit | what it said |
|---|---|---|
| `cargo fmt --all --check` | 0 | — |
| `cargo clippy --all-targets` | 0 | — |
| `cargo test --no-fail-fast` | 0 | 27 result lines, **145 cases passed** |
| `ess specify validate --path src/core` | 0 | `swarm v1 — 9 file(s), valid` |
| `check-sets-are-emitted.py` | 0 | — |
| `aep plan artifact validate` | 0 | valid |
| `python3 -m unittest discover -s examples/two-agents` | 0 | **62 cases** |
| `npm test` in `src/web` | 0 | `# tests 71 # pass 71 # fail 0` |
| `npx vue-tsc --noEmit` · `npx vite build` | 0 · 0 | — |
| `npm run build` in `website` | 0 | — |
| `npm run spec:check` | 0 | README.md and AGENTS.md agree |

Rust cases 108 → **145**. Web 71 → 71. Checker 0 → **62**.

### What this wave produced

**`swarm.agent.AssignmentTaken` fired**, for the first time in this repository's history. Across
eleven swarm logs it had fired zero times, and the working half of `agent.yaml` had never executed.

**Every turn is now framed.** `--decisions observe` — "allow every call and record every call" —
appears nowhere, and a turn with no frame does not launch.

### The ledger

| unit | pass 1 | pass 2 | rounds | carried |
|---|---|---|---|---|
| A | 7 | 9 | 2 corrections + 1 instructed re-pin | 0 |
| B | 7 | 5 | 2 corrections | 0 |

Twenty-eight findings across four passes, zero carried. Four were not the units': two were defects in
this coordinator's own story text, one was an artefact of the branch split, one was `src/web` that no
unit owned.

### What the attacks bought, beyond the fixes

- **2,517 inputs through a hand-written SHA-256 against `sha2`, 0 mismatches.** Declining the
  dependency was measured, not argued.
- **Frame clause 3 moved from "partly met" to measured** — 17 subject probes through metaharness's
  own `SubjectScope::verdict`, no paid run.
- **A coordinator could widen its own sealed frame** with a config it drafts and activates itself,
  while `config.yaml` claimed in the same commit that the worst a config can do is admit less. That
  is the defect the whole story exists to deny, and it survived one full attack pass.
- **The checker could not fail on the thing it existed to check.** Its unattended condition counted
  bare actor `system` as a machine while the runtime writes `system` for every operator command, so
  the swarm the story names as hand-driven reported unattended met — and that verdict decided the
  exit status.

### Three things this wave did not do, in writing

1. **The demonstration has not been run.** Every clause is now *meetable*; none is *met*. Clause 5
   and 7 became possible only with this merge.
2. **This is not containment.** `--substrate`, `--write-scope` and `--cgroup-root` are `b10x` only
   and metaharness refuses them for the claude arm. `AGENTS.md`'s rule stands unchanged.
3. **One defect is pinned, not fixed.** metaharness judges a call by the first rule any subject
   matches, so an outside path is admitted when the same call also names an admitted one. Refusing
   that needs 14,848 glob patterns per swarm. Nothing found reaches it; two cases assert today's
   behaviour with their inversion triggers.

### Filed rather than folded in

`story:an-event-cannot-say-which-agent-acted` — `apply.rs` permits actors by specification actor
*type*, not instance, and `TakeAssignment`'s `agent_id` is caller-supplied, so an `AssignmentTaken`
naming an agent is indistinguishable from anyone issuing it on that agent's behalf. Until it closes,
a single-agent swarm can write a log a reader cannot tell from a two-agent one, and the
demonstration's unattended condition is undeterminable.

### Stage

| unit | stage | branch head |
|---|---|---|
| A | **merged** at `88605fa` | `4ab977d` |
| B | **merged** at `b86e3a1` | `4335170` |

# Wave proposal — 2026-09-12c — the measured defects

Coordinator: this session. Skill `aep-drive:wave` **0.9.1**. `aep` **0.55.0**.
Base branch `main`, at `f776565`. **Approved**: the operator approved the plan at
`~/.claude/plans/encapsulated-swimming-puffin.md` and asked for multiple agents.

## The verb overruled the plan, and that is recorded here

The plan proposed Wave 1 as three parallel units — coordinator, turn cap, deletion — on the reading
that `coordinator.rs`, `budget.rs`+`trigger.rs` and `http.rs`+`state.rs` are disjoint surfaces.

`aep plan artifact waves --kind story --status draft` disagrees, and the verb wins:

```
COLLIDE a-swarm-cannot-be-removed × coordinator-from-config   src/runtime/swarm-server/src/state.rs    cited
COLLIDE a-swarm-cannot-be-removed × the-turn-cap-did-not-hold src/runtime/swarm-server/src/state.rs    inferred
COLLIDE coordinator-from-config   × the-turn-cap-did-not-hold src/runtime/swarm-server/src/state.rs    inferred
COLLIDE coordinator-from-config   × the-turn-cap-did-not-hold src/runtime/swarm-server/src/swarm.rs    inferred
COLLIDE coordinator-from-config   × the-turn-cap-did-not-hold src/runtime/swarm-server/src/trigger.rs  inferred
```

All three land on `state.rs`. The plan's reading was wrong, not the store's.

**The resolution is a narrowing, not an override.** `the-turn-cap-did-not-hold`'s `state.rs` entry is
`inferred` and covers only the *global in-flight ceiling*, which belongs to the multi-agent wave
anyway. Unit A is therefore scoped to exclude `state.rs` entirely, which makes it genuinely disjoint
from unit B. The narrowing is stated in unit A's brief.

`coordinator-from-config` collides with unit A on `swarm.rs` and `trigger.rs` as well, so it **leaves
this wave** and runs next. The operator is not blocked by that: the server was restarted at
`SWARM_COORDINATOR=examples/coordinator-manual.sh` and now reports `configured: true`.

## Pre-flight

| check | result |
|---|---|
| working tree | clean on `main` at `f776565`, pushed |
| worktrees | none — the previous wave's five were removed and `worktree repo list` shows the primary only |
| **free disk** | **14G of 848G, 99% full.** One Rust worktree cost 4.6G measured, so **two is the ceiling** and three was refused |
| `worktree gc` across the whole profile | **frees nothing.** Every record is retained: `no-remote-recovery-proof` on the published ones, `worktree-dirty` on the rest, and one `worktree-not-found` that must stay refused. The tool is doing its job; nothing here is reclaimable |
| what does occupy the disk | `~/.cache/claude-tmp` 28G, `~/.cache/org-brain` 22G, `e21-tmp` 9.3G, `babelforce-specs` 9.0G, `ess-evolution-20260910` 7.2G — **none of them mine to delete.** 160M of my own finished scratch was removed |
| build cache | `sccache` installed, 4.0G, `RUSTC_WRAPPER` unset |
| model budget | no cap; the operator asked for multiple agents |

## Units

| unit | story | surface | worktree |
|---|---|---|---|
| A | `story:the-turn-cap-did-not-hold` | `budget.rs`, `trigger.rs`, `swarm.rs` — **not `state.rs`** | `wave-20260912c-a` |
| B | `story:a-swarm-cannot-be-removed` | `http.rs`, `state.rs`, `stores/swarms.ts`, `SwarmView.vue` | `wave-20260912c-b` |

Both serve `vision:swarm-builds-itself` through `epic:executable-boxes`.

Integration branch `wave/2026-09-12c/integration`, forked from `main` at `f776565`.
Agents: `aep-drive:implementor` per unit, then `aep-drive:adversary` against the same worktree.

## Already done, before any unit

Swarm `123` was removed by hand, in the order the research established: the server was stopped and
the stop verified with `kill -0`; the log was read one last time and held **0 events and 0 commands**;
all three files went together; the server was restarted and `GET /swarms` no longer lists it.

## The commits this authorises

The opening commit, one commit per unit on its own branch, two merges into the integration branch,
the closing store commit, and the merge into `main` once the whole gate is green. Not a tag, not a
release, not the next wave.

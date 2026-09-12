# Wave proposal — 2026-09-12d — the spend ceiling, and a guard that fails open

Coordinator: this session. Skill `aep-drive:wave` **0.9.1**. `aep` **0.55.0** (`protocol 0.55.0`),
`ess` **0.23.0**, `metaharness` **0.6.4**.
Base branch `main`, at `6b3bd87`. **Approved**: the operator approved the plan at
`~/.claude/plans/sprightly-twirling-gosling.md`, which names these two units.

## Selection

Computed, not read pairwise: `aep plan artifact waves --kind story --status draft`.

The verb's waves, verbatim:

```
wave 1  story:connectors-as-service-boxes  story:escapes-scan-fails-open
wave 2  story:coordinator-from-config      story:inert-costs-a-table-its-readers
wave 3  story:honour-view-consistency  story:run-a-tool-box  story:uitable-without-rows
wave 4  story:open-blocks-the-runtime
wave 5  story:slug-is-not-validated
wave 6  story:spawn-a-second-agent
wave 7  story:spend-is-bounded-per-goal-only
```

Unassessed: **none**. Every draft carries typed scope entries. Cycles: none (exit 0).

The verb reports **no collision between `spend-is-bounded-per-goal-only` and
`escapes-scan-fails-open`** — A is Rust-only, B is web-only. They sit in different derived waves
because the deriver groups first-fit, not by readiness. The full collision list the verb printed is
32 pairs; the ones bearing on this selection are that `spend-is-bounded-per-goal-only` collides with
every other Rust story on `state.rs`, and that `escapes-scan-fails-open` collides with
`connectors-as-service-boxes` on nothing — it is paired there only by first fit.

### Why these two and not the verb's wave 1

- `connectors-as-service-boxes` is **not implementable in this tree**: it implies `reqwest` and a
  tokio client in a workspace that has neither, and there is no config surface for an adapter
  endpoint or token. `next-five.md:19` already said so.
- `uitable-without-rows` and `inert-costs-a-table-its-readers` both also land on
  `src/web/src/components/canvas/uibox.inert.test.ts`, so only one web story can run per wave.
- Unit A is first because the coordinator's default became `Launch::Metaharness` on 2026-09-12
  (`src/runtime/swarm-server/src/coordinator.rs:20-30`): a server started with no environment now
  spends real money on every `Pursuing` goal every thirty seconds, and spend is bounded per goal
  only.

## Units

| unit | story | scope | worktree | build dir | scratch |
|---|---|---|---|---|---|
| A | `story:spend-is-bounded-per-goal-only` | **cited**: `budget.rs`, `state.rs`, `swarm.rs`, `trigger.rs`; inferred: `tests/the_turn_cap_under_attack.rs` | `wave-20260912d-a` | `~/.cache/swarm-wave-20260912d-a-target` | `~/.cache/swarm-wave-2026-09-12d/a/` |
| B | `story:escapes-scan-fails-open` | **cited**: `components/ui/index.ts`, `index.test.ts`, `canvas/uibox.inert.test.ts`; inferred: `src/web/package.json` | `wave-20260912d-b` | n/a (node) | `~/.cache/swarm-wave-2026-09-12d/b/` |

Both `serve vision:swarm-builds-itself` through `epic:executable-boxes`.
Integration branch `wave/2026-09-12d/integration`, forked from `main` at `6b3bd87`.
Agents: `aep-drive:implementor` per unit, then `aep-drive:adversary` against the same worktree.

## Pre-flight

| check | result |
|---|---|
| branch | `main` |
| working tree | **dirty** — see below |
| worktrees | none beyond the primary |
| free disk | **17G of 848G, 98% full.** One Rust worktree measured 4.6G last wave; unit B is node and costs ~0.5G. Two units is the ceiling |
| build cache | `sccache` at `/usr/bin/sccache`, 1.3G, `RUSTC_WRAPPER` **unset**. Left unset — it is unset for every other session on this machine too, and setting it for one worktree is not this wave's decision |
| stale build dirs | none. The `~/.cache/swarm-*` entries are scratch from earlier waves, 4K–900K each |
| model budget | operator stated none; N=2, under the skill's default of 4 |
| `git config user.email` | `316511680+b10x-bot[bot]@users.noreply.github.com` (repo-local) |
| `AGENTS.md` | read. Its 11-step gate and its planning-store rule govern this wave |

### The dirty tree, and the override

`git status --porcelain`:

```
 M .engineering/planning/journal.jsonl
```

Four identical `aep.evidence.record/v1` lines for
`review-result:adversary-2026-09-12c-unit-a-pass-2` on `story:coordinator-from-config`, from a
command that ran four times at the end of wave c. **No unit's surface touches this path** — it is
the planning store, which only this coordinator writes.

The operator approved opening on a dirty tree (plan § Decisions 2). The lines are left exactly where
they are: not stashed, not branched away. The closing store commit will carry them, and the closing
report will say so.

## Two deviations from the skill, both on `AGENTS.md`'s authority

1. **No `aep plan artifact move`, at open or at close.** `AGENTS.md` § Planning store: *"Never run
   `aep plan artifact move`: whether work is done is a claim about the world that rests on evidence
   an operator reads, not on an agent having finished typing."* The repository's file overrides the
   skill. Evidence is recorded against the merge commit; the status moves are the operator's.
   Wave c's own close records the cost of getting this wrong in the other direction —
   `story:the-turn-cap-did-not-hold` was moved to `implemented` with a clause still open, and the
   lifecycle admits no way back.
2. **The stories stay `draft` while the wave runs**, which follows from 1.

## The commits this authorises

The opening commit, one commit per unit on its own branch, two merges into the integration branch,
the closing store commit, and the merge into `main` once the whole 11-step gate is green. Not a
push, not a tag, not a release, not the next wave.

## Stage

| unit | stage | branch head |
|---|---|---|
| A | not started | — |
| B | not started | — |

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

---

## The close

**Status: closed 2026-09-12. Both units merged; the whole 11-step gate is green on the integration
branch, each step's own exit status read.**

| step | exit | what it said |
|---|---|---|
| `cargo fmt --all --check` | 0 | — |
| `cargo clippy --all-targets` | 0 | — |
| `cargo test` | 0 | 22 result lines, **103 cases passed** |
| `ess specify validate --path src/core` | 0 | — |
| `check-sets-are-emitted.py` | 0 | — |
| `aep plan artifact validate` | 0 | — |
| `npm test` in `src/web` | 0 | `# tests 59 # pass 59 # fail 0` |
| `npx vue-tsc --noEmit` | 0 | — |
| `npx vite build` | 0 | — |
| `website` build | 0 | — |
| `npm run spec:check` | **1, then 0** | see below |

Rust cases 65 → **103**. Web cases 51 → **59**.

**Step 11 earned its place on its first wave.** It failed: the README said the runtime is "roughly
6,100 lines" and this wave grew it to 7,500. That is precisely the drift the step was added to catch,
on prose no build step can import, one wave after it was made canonical. Corrected, then green.

### The ledger

| unit | pass 1 | pass 2 | carried | resolved |
|---|---|---|---|---|
| A | 6 | 3 | **0** | 6 |
| B | 6 | 3 | **0** | 6 |

Zero carried in both units in both rounds. The attack budget is two passes and both were spent; the
correction answering each second pass was verified by this coordinator reading the diff — no
assertion dropped, no adversary case weakened, the one adversary edit in each unit made on explicit
instruction and recorded in the case's own doc comment.

### What the wave was actually for, and what it found instead

It was dispatched to fix three measured defects. Two of the three premises turned out to be wrong,
and finding that out was worth more than the fixes.

- **The $11.35 overrun happened once, not twice, and the cap did not fail — it did not exist.** The
  44 spend rows sum to $11.345391 and the first 43 to $11.098908: the two figures in `budget.rs` are
  one file read one turn apart. `993731e` introduced the module and both call sites at 07:15:35Z; the
  run ended at 01:29:33Z.
- **The cap that did exist was off by one.** `SWARM_MAX_TURNS=3` ran two turns, because `fire` checks
  turns taken and `ask_the_coordinator` checks an `iterations` field `fire` has already advanced.
  Invisible to every reading and to the first version of the tests, which called `capped` directly —
  found only by driving the real guards.
- **Which coordinator a swarm launched was a coin flip.** `ActivateConfig` never supersedes the
  previous row, so a replaced config leaves two Active and `.find()` over a UUID-keyed map answered
  the old one about half the time — measured four-and-four across eight swarms.
- **`activated_at` is never written**, so one of the two fixes the coordinator offered for that was
  not implementable. The unit said so rather than pretending, and refused instead.
- **The first destructive verb this server ever had shipped a path traversal.**
  `DELETE /swarms/..%2Fescaped` answered 204 and erased a directory outside `swarms/`, from one
  unauthenticated request. Found by the adversary before the merge; `main` never had a delete route,
  so nothing was ever exposed.

### Three classes closed by construction rather than by a list

- an unattributed spend row is **unrepresentable** — `record_spend` takes a mandatory agent and the second door is gone;
- a handle can only live where one lookup looks — a test reads the fields of `Server` out of the source, keeps those typed `Arc<Swarm>`, and fails if any is not named in `handle_of`;
- no route that takes a slug reaches outside `swarms/` — a test reads the route table out of `http.rs` and drives every `{slug}` route with a traversal, so a route added without validation fails on the day it is added.

### Left open, on purpose and in writing

| what | where |
|---|---|
| an agent is bounded per goal, not at all — $6.00 of a $5.00 cap over two goals | `story:spend-is-bounded-per-goal-only`, **pinned red-when-fixed** in the suite |
| `/status` still answers `coordinator.configured: false` for a server that will run metaharness | `story:coordinator-from-config`, kept open; patch written, unapplied |
| `config.yaml` still claims a supersede nothing performs | patch written, unapplied — it is the specification, and no adversary budget remains |
| `SplashView.vue`'s second `Uncreated` | patch written, unapplied |

**A coordinator error, recorded rather than hidden:** `story:the-turn-cap-did-not-hold` was moved to
`implemented` while its fourth acceptance clause was still open. The lifecycle admits only
`implemented → archived`, so the move cannot be undone; the clause is carried forward whole into
`story:spend-is-bounded-per-goal-only`, which exists because of that mistake.

### Two things about running this wave

**Disk capped it at two units, not the three the plan named**, and `aep plan artifact waves`
overruled the plan's grouping besides — all three stories land on `state.rs`. The verb was right and
the plan was wrong.

**`worktree gc` across the whole profile freed nothing**, correctly: every record is retained on
recovery proof or dirtiness. The disk pressure was not the wave's to solve, and one thing this
coordinator did about it was a mistake — `~/.cache/sccache` was deleted after checking
`RUSTC_WRAPPER` in one shell, while 21 sccache processes were running for other sessions. Nothing was
lost, because a cache is not a source of truth, but the check was the wrong check and other
sessions paid for it in cold builds.

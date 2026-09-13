# Changelog

Notable changes to this repository. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Every number in this file is read from a command's output. Where a claim is inferred rather than
measured it says so.

## [0.2.0] — 2026-09-13

A swarm ran two agents. `swarm.agent.AssignmentTaken`, `AssignmentDone` and `AgentIdle` had each
fired **zero times** in this repository's history, across eleven swarm logs; they fire now, in a run
kept at `data/swarms/two-agents-proof`. Every turn also launches under a sealed frame that refuses
what it does not admit. 30 commits since `v0.1.0`.

### Added

- **A second agent that the runtime runs.** `swarm.agent.Role` gained `Worker`. A coordinator spawns
  one, posts it an assignment, and the runtime runs it as its own `metaharness` session with its own
  work directory, transcript, mailbox and launch knobs. `ensure_coordinator` became `ensure_agent`,
  so one door cannot express the wrong role. In-flight claims are keyed on a `Unit` — a goal or an
  assignment — so two agents on one goal do not serialise on one claim.
- **A sealed frame around every turn.** `src/runtime/swarm-server/src/frame.rs` builds and seals a
  `metaharness.frame/1` document; turns launch `--decisions frame --frame <file>` and a turn with no
  frame does not launch. `--decisions observe` — "allow every call and record every call" — appears
  nowhere in the tree. Measured in the demonstration: **1 of 41 decided calls refused, `decided_by:
  frame`**.
- **A subject scope the runtime derives**, never reads from configuration: an agent's own work
  directory in full, the swarm's shared root read-only, everything else refused. Derived rather than
  configured because a config a coordinator writes for itself could widen its own scope. Verified
  against metaharness's own `SubjectScope::verdict` by 17 subject probes.
- **`turns/capped.jsonl`** — a fired ceiling is now recorded where a reader finds it after the run.
  Before this, a refusal reached only the watch stream and the tracing log, both gone with the
  process.
- **`SWARM_MEMBER_MAX_TURNS`, `_BUDGET_USD`, `_MODEL`, `_EFFORT`**, with no fallback to the
  `SWARM_COORDINATOR_*` knobs. A fallback would mean setting a coordinator's model silently bought it
  for every member at every member's per-turn price.
- **`examples/two-agents/`** — a checker that reads a finished swarm's `eventlog.sqlite3`, turn files
  and `spend.jsonl` and reports seven acceptance clauses independently, by number, met or not met,
  with what it looked for and what it found. 1,038 lines, stdlib only, **62 cases**.

### Changed

- `CappedGoal` and the watch stream carry `agent` and `unit`, so two agents publishing at once can be
  told apart. `src/web/src/runtime.ts` was updated to match; the publisher and the only consumer had
  drifted.
- The frame states its admitted operations and its work directory **in the prompt**, because the
  vendor's tool list is not narrowed by the frame — a model that reads the list and not the prompt
  spends a billed turn discovering a refusal.
- `README.md`, `AGENTS.md` and the website's honesty table: the finding that *"a working agent swarm
  is overstated"* is retired by a recorded run and replaced with what is still overstated — one
  coordinator, one worker, one assignment. The finding that *nothing confines a coordinator* **stands
  unchanged**, with what did change stated beside it.

### Fixed

- A coordinator could draft and activate a config that **widened its own sealed frame** to
  `subagent.spawn`, `task.todo` and `web.read`, while `config.yaml` claimed in the same commit that
  the worst a config can do is admit less. `from_config` intersects with the admitted set now, and
  refuses a config whose every operation is outside it.
- Every agent of a swarm shared one work directory while both prompts told each of them to keep a
  `NOTES.md` in it.
- A retry's frame told the model it was a first attempt.
- Agent ids differing only in punctuation shared one directory; slugs now carry a digest suffix.
- A retasked member was refused at its first turn at a new assignment, under the wrong bound.

### Known limits, in writing

- **Depth one, breadth two.** One coordinator, one worker, one assignment. Agents spawning agents and
  a swarm extending its own specification are undemonstrated.
- **This is not containment.** `--substrate`, `--write-scope` and `--cgroup-root` are `b10x` only and
  metaharness refuses them for this arm; its own attestation says a hermetic run here is not
  network-isolated.
- **One pinned weakness.** metaharness judges a call by the first rule any of its subjects matches,
  so an outside path is admitted when the same call also names an admitted one. Refusing it needs
  14,848 glob patterns per swarm. Nothing found emits such a call; two cases assert today's behaviour
  with their inversion triggers.
- **An event records an actor *type*, not an agent instance**, so a log cannot prove which agent
  issued a command, and the demonstration's "unattended" condition is reported UNDETERMINABLE rather
  than guessed. Filed as `story:an-event-cannot-say-which-agent-acted`.

### The shape of it, in numbers

- Rust: **10,372 lines** across three crates — `ess-runtime` 1,927, `swarm-server` 7,807,
  `swarm-cli` 638 — against **4,808** lines of YAML specification.
- Tests: **145 Rust cases** across 27 lanes, **71** web cases, **62** checker cases. Integration test
  files 18 → 22, 7,210 lines.
- Specification counts unchanged: 6 domains, 10 entities, 24 types, 53 commands, 56 events,
  17 errors, 27 views, 12 actors, 4 bindings.

## [0.1.0] — 2026-09-12

First release, and the first time this repository has been published at all. Everything below
already existed in the tree; `0.1.0` is the point at which it acquired a remote, a tag and a
changelog.

### The system

- A swarm manager whose kernel is an Executable System Specification in `src/core/` — `ess/4`,
  9 files, **6 domains, 10 entities, 24 types, 53 commands, 56 events, 17 errors, 27 views,
  12 actors, 4 bindings**, counted by `ess specify compile`. `ess specify validate --path src/core`
  reports `swarm v1 — 9 file(s), valid`.
- A runtime that **executes that specification rather than implementing it**. `ess-runtime` compiles
  the spec at boot and interprets the resolved IR; a command added to a domain file is reachable
  over HTTP, in the CLI and in the UI without code being written. Nothing in this repository is
  code-generated from the specification — `src/core/generated/` is gitignored and `ess generate` is
  an optional manual projection, not a build step.
- 6,103 lines of hand-written Rust across three crates — `ess-runtime` 1,927 (the interpreter),
  `swarm-server` 3,538, `swarm-cli` 638 — against 4,759 lines of YAML specification.
- A Vue 3 canvas UI in `src/web/` (7,815 lines) that reads `GET /spec` rather than hard-coding what
  a swarm contains.
- 12 HTTP routes, a CLI with a mailbox and a generic `issue`, and a periodic `turn-the-loop` binding
  that fires every 30 seconds and runs one coordinator per swarm.

### Added in the run-up to this release

- **Redelivery for an at-least-once binding.** A delivery that fails is owed, retried up to
  `MAX_ATTEMPTS`, and announced as `What::Undelivered` when it is finally given up on. Neither the
  attempt count nor the delay is the specification's, and both say so where they are declared.
- **A UI box that renders.** A `Box{kind: Ui}` on the canvas mounts the component it names, with the
  props it carries, decoded as the type the component library's contract declares.
- **A component-library drift guard.** Every component's `defineEmits` is compared with its row in
  the contract table, in five spellings, failing closed: a component that calls `defineEmits` and
  yields no readable event name is drift by definition.

### Fixed

- **`Server::open` could replace a live `Swarm` handle.** It read the map, dropped the guard, awaited
  `Swarm::open`, then inserted unconditionally — so two concurrent opens of one slug both built a
  handle and the second replaced the first, which could hold the only copy of an owed delivery.
  Construction is now serialised per slug, and 1,600 contended opens produce no refusal.
- **The request key was documented as an idempotency key.** It guards the append on one instance's
  stream and does nothing about a command being applied twice. Three reader-facing surfaces said
  otherwise; all three are corrected, and a test derives the set of documents that mention the key
  and fails if any presents a refuted promise as current.
- **`UiTable` emitted `row-click` and the contract did not say so**, so a table panel on the canvas
  rendered interactive while discarding every event.

### Known, recorded, not fixed

Each of these is an open story in `.engineering/planning/`, with the measurement that found it:

- `open-blocks-the-runtime` — an open of an unrelated slug still waits, because the replay inside
  `Swarm::open` is CPU-bound work in an `async fn` that occupies a tokio worker. The case that
  measures it is compiled but not run by default; the reason is at its declaration in
  `src/runtime/swarm-server/Cargo.toml`.
- `slug-is-not-validated` — `open("../escaped")` returns `Ok` and creates a swarm outside the swarms
  directory; `alias` and `./alias` are two handles over one event log.
- `escapes-scan-fails-open` — the scan deciding which components may not be drawn on a canvas
  matches four literal spellings.
- `uitable-without-rows`, `inert-costs-a-table-its-readers`, `request-key-is-not-idempotency`'s
  remaining half, and four more. `aep plan artifact list --kind story --status draft` is the list.

### Infrastructure

- Published to `github.com/beyond10x/swarm`.
- **All 59 commits were rewritten** so that every author and committer is `b10x-bot[bot]`. Every
  hash changed; `.engineering/waves/rewritten-history.md` explains it and
  `.engineering/waves/2026-09-12b-evidence/rewrite-commit-map.txt` maps old to new.
- The Beyond10x Gates security and privacy check is wired in `.github/workflows/shared-gates.yml`,
  pinned to an exact Gates commit. **It is not yet enforceable**: this repository is not enrolled in
  the protected policy and is not in the `B10X_GATES_POLICY` secret's selected repositories.
- Atlas documentation integration declared in `b10x.docs.yaml`, validated against `b10x-docs/v4`.
  **Not yet live**: the Atlas catalog has no entry for this repository.
- The website deploys to GitHub Pages from `.github/workflows/deploy-website.yml`, every action
  pinned to a commit.

[0.1.0]: https://github.com/beyond10x/swarm/releases/tag/v0.1.0

# Changelog

Notable changes to this repository. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Every number in this file is read from a command's output. Where a claim is inferred rather than
measured it says so.

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

# Next wave proposal — 2026-09-13c

## Planning verification

The original proposal commit (4d9e07b) edits planning and wave notes only; this paragraph records its checks. Implementation and dependency changes began after approval. Whitespace validation, cargo fmt --all --check, ESS validation, AEP validation and the website facts check each exited 0. The full compilation/test/build gate belongs to the implementation wave and was not rerun for this proposal.

AEP output, verbatim:

```text
67 file(s) in /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-next-wave-plan-20260913/.engineering/planning: 67 artifact(s)
11 review(s) recorded no findings block:
  - review-result:demonstration-evidence-note-2026-09-13 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-acceptance-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-design-round-1 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-design-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-parallel-round-1 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-parallel-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-scope-round-1 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:next-wave-20260913-scope-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:plan-critic-parallel-safety-goal-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:plan-critic-scope-goal-round-1 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
  - review-result:plan-critic-scope-goal-round-2 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
valid
```

Status: **approved by the operator's `ok`; both units integrated; full gate pending**. Coordinator: Codex leader.
Skill `aep-drive:wave 0.8.1`; planning skill 0.8.1; installed `aep --version` reports `protocol 0.55.0`.
Interactive run: the operator asked to see the next wave after cleanup. This proposal is the review boundary.
The operator subsequently approved this exact proposal. The active integration branch is now `wave/2026-09-13c/integration`; owning session `wave-20260913c-leader`.

## Execution ledger

Bootstrap registers a buildable `swarm-check` shell with a deliberately failing exit code, its checker dependencies, and Unix libc for process-group control. No behavior is implemented by the scaffold. Cargo metadata for the Linux host resolves the shared lockfile; unrestricted offline metadata attempted to fetch an irrelevant Fuchsia dependency, so the host-specific metadata check is used instead.

| unit | stage | source | target | scratch | branch/head |
|---|---|---|---|---|---|
| A | integrated bb44d5b | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control | same source /target | /home/timo/.cache/swarm-wave-2026-09-13c/control | wave/2026-09-13c/control; 6d84959 (base c935d85) |
| B | integrated 271242a | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker | same source /target | /home/timo/.cache/swarm-wave-2026-09-13c/checker | wave/2026-09-13c/checker; 7d5c596 (base c935d85) |

Integration target is inside `swarm-next-wave-plan-20260913/target`; integration scratch is `/home/timo/.cache/swarm-wave-2026-09-13c/integration`. All implementation builds set RUSTC_WRAPPER to `/usr/bin/sccache`, CARGO_BUILD_JOBS=2 and unit-owned TMPDIR. The primary checkout's target remains untouched.
Base: published, clean `main` at `5331fe8e3c338d12f8fde2371f1218f6c3a551a9`.

### Implementation handoff

A: 4098ad1; package tests 117 → 129 executed, all green. Formatter, strict Clippy, ESS validation and emitted-field check each exited 0. The new private control module and focused integration target confirm inferred scope. Two existing cases gained Running lifecycle setup without weakening their assertions. Linux process-group cleanup is verified; intentionally escaped groups and other platforms are not claimed as verified. The host retry route is POST /swarms/{slug}/quiesce; the integration documentation names its success/refusal/error contract.

B: d0eac57; Rust package 0 → 98 executed, all green: 74 behavioral cases covering 83 baseline report snapshots, nine metadata guards, and 15 compatibility tests. The original Python suite ran all 83 tests before removal. Original demonstration JSON matches exactly; text words match with wrapping differences; all 17 input hashes remain unchanged. The historical run meets seven clauses and does not establish unattended operation. Seven Python source/fixture/test modules were removed; historical exported evidence remains untouched. Inaccessible evidence fails closed without a SQLite write fallback; malformed JSON-lines retain historical skip behavior.

Full implementor reports, original red outputs and per-lane runner counts are retained at:
- /home/timo/.cache/swarm-wave-2026-09-13c/control/report.md
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/report.md

At this initial handoff, each implementor released its own lease before coordinator commits; adversaries then acquired separate pass-1 leases. The current integration state is in the execution ledger above.

Integration dependencies: src/web npm ci --offline exited 0. Website npm ci --offline exited 1 (EALLOWGIT: the environment disables Git dependency fetches). Package.json and lockfile compare exactly with primary; its existing website/node_modules was copied into integration for the isolated build. No dependency or npm policy was changed. These are installation observations, not a substitute for the final build gate.

### Adversary pass 1 and correction routing

Both reviews are recorded verbatim in review-result:adversary-2026-09-13c-unit-{a,b}-pass-1, with machine-readable findings. Their leases were released before implementors resumed.

A: 129 → 130 executed, one new red case. Pause can invalidate a worker after FinishAssignment commits and before GoIdle, leaving Done work with a Working agent after resume. The correction must make the logical multi-command result indivisible with respect to lifecycle cancellation, while releasing publication exclusion before waiting for quiescence. Existing tests remain unchanged.

B: 98 → 103 executed, five new red cases, two compatibility findings. Python integer-instance and numeric equality behavior differ from the port for boolean, integral-float and large unsigned iterations; non-string denial reasons differ in truthiness and rendering. Most malformed shapes have no runtime emitter; the measured failure is the explicit readable-file CLI parity contract. All 83 original names/subcases and baseline cases passed the mapping audit.

The checker correction must preserve object ordering locally. Enabling serde_json preserve_order was declined because Cargo feature unification could change server JSON maps and pinned eventlog request_hash hashes raw serde_json serialization. No global map-order feature or event-log hash behavior change is authorized by this port.

The original two-pass review bound remains: correction 1 goes to adversary pass 2, with any remaining blockers reported explicitly. No unit has merged, and no review outcome is marked fixed before the correction is verified.

### Control correction 1

A correction at 1239327 preserves the adversary's case and serializes FinishAssignment plus GoIdle against lifecycle application. Both lifecycle-first orderings (Pause/Resume and Stop/Start) are also covered. Publication exclusion is released before quiescence waits, so old claims can reject their generation and retire. This is host serialization, not an atomic database transaction across separate ESS commands.

The unchanged red case reran red before correction and then green; the package grows 130 → 132 executed, all passing. Formatter and strict Clippy exit 0. The first finding has a fixed review_outcome record. Full report: /home/timo/.cache/swarm-wave-2026-09-13c/control/correction-1-report.md. Pass 2 is now running against 1239327; no merge yet.

### Control final attack and checker correction

A pass 2: no findings; 132 → 134 executed, all passed. New cases cover two real worker turns contending with pause/resume and stop/start. Pass-1 regression is unchanged and green. Tests are retained in aa372c3; review-result:adversary-2026-09-13c-unit-a-pass-2 records the complete report. The coordinator then noticed the pass-2 fake process used newly authored Python, contrary to the unit brief. A fixture-only correction is replacing that subprocess with a Rust test child, preserving all assertions and barrier order; it receives direct coordinator verification, not a third attack.

B correction 0292605 + coordinator lock reconciliation 8ac92f7: 103 → 118 executed, all passed. The five red cases are unchanged; fourteen additional groups compare 81 saved baseline reports, and a fifteenth guard checks unchanged ordinary JSON map serialization. Source-ordered evidence values and arbitrary integer semantics stay local to the checker. New dependencies are num-bigint0.4.8, num-integer0.1.47, num-traits0.2.19, autocfg1.5.1; serde_json enables raw_value only. Root reviewed and committed the generated lock change separately. Both first-pass findings have fixed outcomes. The original demonstration differential and 17 input hashes still pass. Full correction report: /home/timo/.cache/swarm-wave-2026-09-13c/checker/correction-1-report.md. Pass 2 now attacks 8ac92f7.

A findings trend, verbatim from aep plan artifact findings (exit 0):

```json
{
  "artifact": "story:pause-and-stop-control-runtime-work",
  "reviews": 10,
  "from": "review-result:adversary-2026-09-13c-unit-a-pass-1",
  "from_reviewer": "unattributed",
  "to": "review-result:adversary-2026-09-13c-unit-a-pass-2",
  "to_reviewer": "unattributed",
  "carried": [],
  "new": [],
  "resolved": [
    {
      "file": "src/runtime/swarm-server/src/swarm.rs",
      "line": 829,
      "category": "concurrency",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "Pause can invalidate the worker between committed FinishAssignment and its separate GoIdle, leaving a Done assignment with a Working agent that normal resume and dispatch cannot repair."
    }
  ]
}
```

Counts: carried 0, new 0, resolved 1; findings fell from 1 to 0.

### Control integration and checker final correction

The coordinator reviewed the Rust fixture-only replacement: exact assertions/barrier/timeout body unchanged, both behavioral cases rerun green, package 134 → 135 passing because one child-entrypoint test was added. Formatter and strict Clippy passed. Commit 6d84959 is merged into integration at bb44d5b. No new Python remains in the control tests. The pre-merge merge-tree dry run for both unit heads exited0 and produced c3d1278efcea14c154b0cabad8cb13ac50c6f760 with no conflict. Typed and prose scopes now reflect actual control.rs/lib/test/manifest changes; state.rs was inspected but did not change.

B final attack against 8ac92f7: 118 → 123 executed, 120 green and three red. All pass-1 cases and81 correction snapshots remain green. Two reported findings: printable combining marks are over-escaped in recursive repr; legacy-readable NaN and escaped lone surrogate reasons are rejected by the local parser. Ordinary runtime map/wide-number behavior and duplicate-object member ordering probes passed. No actual runtime emitter was established for the unusual reason shapes; the failure is the explicit legacy-reader compatibility contract.

Both findings return to the same implementor: no prior case failed again. Correction2 will be reviewed directly by the coordinator, including assertion preservation, as wave SKILL.md lines580–583 require. The two-attack budget is exhausted; there is no third attack. Only a green corrected unit may merge.

B findings trend, verbatim from the CLI (exit0):

```json
{
  "artifact": "story:verify-two-agent-evidence-in-rust",
  "reviews": 10,
  "from": "review-result:adversary-2026-09-13c-unit-b-pass-1",
  "from_reviewer": "unattributed",
  "to": "review-result:adversary-2026-09-13c-unit-b-pass-2",
  "to_reviewer": "unattributed",
  "carried": [],
  "new": [
    {
      "file": "src/runtime/swarm-check/src/evidence.rs",
      "line": 270,
      "category": "contract-drift",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "Recursive evidence representation escapes printable combining characters that Python preserves, changing parsed report reason strings."
    },
    {
      "file": "src/runtime/swarm-check/src/evidence.rs",
      "line": 26,
      "category": "acceptance",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "Legacy-readable NaN and escaped lone-surrogate reason values are rejected and their decision rows dropped, changing clause-five and CLI verdicts."
    }
  ],
  "resolved": [
    {
      "file": "src/runtime/swarm-check/src/clauses.rs",
      "line": 273,
      "category": "acceptance",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "Numeric spend compatibility is narrower than the legacy reader, reversing clause-three and CLI verdicts for boolean, integral-float, and large unsigned iteration inputs."
    },
    {
      "file": "src/runtime/swarm-check/src/clauses.rs",
      "line": 529,
      "category": "contract-drift",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "Non-string denial reasons change the clause-five found field through different truthiness and composite-value rendering."
    }
  ]
}
```

Signature counts: carried0/new2/resolved2, findings2 → 2. Semantically one new signature continues the rendering class at a new file/line; the parser-acceptance finding is new ground. The signature ledger is preserved without relabeling it by judgement.

## Recommended wave

| unit | story | visible result | owner surface |
|---|---|---|---|
| A | `story:pause-and-stop-control-runtime-work` | Pause/stop quiesce runtime-owned coordinator and worker turns; no late result or new launch escapes the acknowledged cancellation | swarm-server dispatch/process/lifecycle handling; manager host descriptions |
| B | `story:verify-two-agent-evidence-in-rust` | A Rust checker replaces the introduced Python checker and preserves the mapped regression claims | new swarm-check crate and examples/two-agents |

Both serve `vision:swarm-builds-itself` through `epic:trustworthy-swarm-control`.
This addresses the failed-control behavior found in the review and the explicit unfinished commitment to remove the newly introduced Python.
Two implementors can work independently. One coordinator owns shared manifests, lockfile, documentation and derived facts.
The intended pause semantics are explicit in the proposed story: active turn processes are cancelled, evidence remains, and resume permits eligible work to continue. This is stronger than merely suspending the next scheduled tick.

## Selected scope and ordering

Three independent `aep-drive:story-scoper` readings confirmed the surfaces and were written into the story bodies and typed scope entries.
A is high-confidence, with cited implementation files and inferred test/module/dependency paths.
B is high-confidence and cited, including coordinator-owned integration paths.
The pair has no declared dependency edge and no collision in the CLI output below.

The CLI greedily groups the whole draft backlog and has no include-ID filter. It places B in group 2 and A in group 4 because other draft stories occupy earlier conflicting slots. The proposed pair is drawn from those groups using the returned collision list; it does not override a reported collision.
One input is known stale as scheduling state: the already-published attribution story is still draft under the repository's operator-only move rule. Its body now says not to implement it again; no lifecycle move was made to manipulate the planner.
The complete CLI output, including all groups, collisions and unassessed entries, is preserved verbatim below.

## Deliberately deferred

- `story:show-command-issuers-in-the-canvas`: real collision with A at `src/runtime/swarm-server/src/swarm.rs`. The live What::Applied payload lacks issuer; a frontend-only fix would be incomplete. Its story is now recorded, including historical/live/mail coverage. The old patch is corrupt and is retained only as a sketch.
- Tool/Service execution, replay responsiveness, slug validation and remaining UI defects: not selected ahead of restoring control and fulfilling the checker commitment. Their existing scopes remain in the computed inventory.
- A fresh unattended demonstration, authentication, aggregate spend/accounting hardening and OS containment: this wave makes no claim to deliver them. The old demonstration is preserved with its actual limitations.
- A new release: a separate operator decision after a green integration.

## Coordinator-owned sequence

1. Before any implementor dispatch, recheck clean base/head, disk, leases, input availability and the selected scopes. Reuse this planning checkout as integration; do not create a competing store writer.
2. Bootstrap workspace membership and the minimal buildable swarm-check package plus agreed shared dependency entries on integration before forking units. This is scaffolding only; no checker behavior or regression implementation is assigned to the coordinator. Resolve any server-local signal dependency before unit gates. This allows real package-scoped cargo checks while keeping the root manifest and lockfile single-writer.
3. Each implementor edits only its assigned source/tests. A owns server-local manifests if necessary; B owns the new crate manifest. Cargo.lock changes are returned as integration requirements and reconciled by the coordinator; no unit commits a competing root lockfile.
4. Route each completed unit through `aep-drive:adversary`, at most two attack passes, retaining red evidence. Bootstrap and shared-file changes are visible to the adversary and the final integration gate.
5. Coordinator merges green units, updates current checker commands and relevant host-control explanations, derives site facts, records evidence, and runs all 11 AGENTS.md gate steps in order, with each step's exit status read separately. The Rust port's mapped cases run under cargo test; existing Python core-spec checks remain.
6. Merge the green integration into main under wave approval. No `aep plan artifact move`. Publication, release and retirement reuse only authorization applicable at that point.

Approving this wave authorizes the opening/bootstrap commit, two unit commits, required correction commits, the unit merges, the closing planning/evidence commit, and the integration merge into main. It does not authorize a push, tag or version bump.

## Pre-flight and resource limits

| check | observed / planned |
|---|---|
| primary | clean and synchronized at 5331fe8 |
| previous wave | both unit/integration trees and merged branches retired; their published commits remain on main |
| old wave build outputs | removed; no retained old-wave build directory |
| retained evidence | 1.8 MB wave scratch plus 92 KB gate logs; retain logs and patch, not garbage |
| other repository records | reconciliation reported three unresolved metaharness/substrate records; no cross-repository retirement attempted |
| free space | 29 GB reported by df at planning time; filesystem 97% used |
| measured prior tree footprint | unit about 3.0 GB; integration about 3.2 GB including dependencies, before cleanup |
| planned additional build allowance | 12 GB maximum before re-evaluating concurrency; keep at least 10 GiB free and check before each large build |
| compiler cache | /usr/bin/sccache 0.17.0 available, stats command succeeds; shared cache bounded at 10 GiB; RUSTC_WRAPPER currently unset |
| build environment | set RUSTC_WRAPPER=/usr/bin/sccache for unit/integration cargo commands; two build jobs per checkout; separate target directories; cache growth counts against the allowance |
| model budget | operator said no limit; N=2 is chosen for non-overlap and disk, not model budget |
| paid execution | none required; control tests use fake processes and the checker reads existing evidence |
| gate baseline | previous wave recorded 158 Rust / 83 checker / 71 web cases; this proposal does not claim another runtime gate run |

A pre-flight that finds less than the disk floor or an unconfigured cache refuses dispatch until corrected. Remove only completed units' exact reproducible build directories after retaining their evidence; do not purge the primary checkout's shared 7 GB target or another session's caches to make space.

## Checkout and scratch ownership

| purpose | managed id | branch | source path | build path | scratch root | stage |
|---|---|---|---|---|---|---|
| integration | swarm-next-wave-plan-20260913 | wave/2026-09-13c/integration | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-next-wave-plan-20260913 | same path /target | /home/timo/.cache/swarm-wave-2026-09-13c/integration | full gate pending |
| A | swarm-wave-20260913c-control | wave/2026-09-13c/control | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control | same path /target | /home/timo/.cache/swarm-wave-2026-09-13c/control | integrated; retained unpublished |
| B | swarm-wave-20260913c-checker | wave/2026-09-13c/checker | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker | same path /target | /home/timo/.cache/swarm-wave-2026-09-13c/checker | integrated; retained unpublished |

The coordinator updates each actual path, branch head and stage on transition.
Current owning session: wave-20260913c-leader. Each implementor owns a distinct lease.
Both units forked bootstrap c935d85; their assigned scratch roots contain brief.md.
Planning evidence remains at /home/timo/.cache/swarm-next-wave-plan-20260913.
All new commits are local until a publication decision; source trees remain retained.

## Original demonstration input

The primary checkout still contains `data/swarms/two-agents-proof/eventlog.sqlite3`, three transcript JSONL files, their sealed frames, spend.jsonl and capped.jsonl. The checker implementor receives that root explicitly as read-only.
The SQLite SHA-256 observed at planning is `4c22f000aaa451b9270bc1cfdf1f05f3ea2a342d570afdaa85663fb11a9731ca`.
Recheck all inputs and record hashes before differential comparison; do not reconstruct missing historical transcripts from the committed export.

## Review and tool limitations

Three scopers and the four named critics reviewed the decomposition. Round 1 found two acceptance defects; both were revised and received review_outcome=fixed records. Round 2: all four approve; no carried findings.
Records: `review-result:next-wave-20260913-{acceptance,design,scope,parallel}-round-{1,2}`.

Critic roles: `aep-plan:plan-critic-acceptance`, `aep-plan:plan-critic-design`, `aep-plan:plan-critic-scope`, `aep-plan:plan-critic-parallel-safety`.
Implementation roles on approval: `aep-drive:implementor` and `aep-drive:adversary`.
The harness exposes general collaboration agents, not plugin subagent_type or the critics' Sonnet model pin. Agents followed the installed charters on the inherited model. Only three child slots are available, so the fourth critic ran after a slot freed, independently and without reading the other reviews. These are disclosed execution deviations, not a claim of native plugin dispatch.
The harness did not supply per-agent token/tool-use totals; none are invented.

AEP validation succeeds. The binary warns about the four old prose-only reviews and also warns on approval records containing the rubric-mandated empty findings list. Preserve the empty lists and immutable reviews; do not invent findings to silence that advisory.
Connectors doctor still reports its local daemon unavailable; no integration write or daemon change is needed for this local proposal.

## Raw scheduling output

Command: `aep plan artifact waves --kind story --status draft --format json`. Exit 0. All returned groups, collisions, unassessed entries and cycles follow unchanged.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:an-event-cannot-say-which-agent-acted",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "examples/two-agents/check-two-agents.py"
            },
            {
              "confidence": "cited",
              "path": "src/core/domains/agent.yaml"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/ess-runtime/src/apply.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/ess-runtime/src/store.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/http.rs"
            }
          ]
        },
        {
          "id": "story:connectors-as-service-boxes",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "src/core/domains/blackbox.yaml"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/state.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/runtime.ts"
            }
          ]
        },
        {
          "id": "story:inert-costs-a-table-its-readers",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/web/src/components/canvas/UiBox.vue"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/canvas/uibox.inert.test.ts"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/components/ui/index.ts"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/styles"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:honour-view-consistency",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/runtime/ess-runtime/src/views.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/ess-runtime/tests/computes_views.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/state.rs"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/runtime.ts"
            }
          ]
        },
        {
          "id": "story:run-a-tool-box",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "src/core/components.yaml"
            },
            {
              "confidence": "cited",
              "path": "src/core/domains/blackbox.yaml"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/box_runner.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/coordinator.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/swarm.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/trigger.rs"
            }
          ]
        },
        {
          "id": "story:the-guard-reads-no-template-or-style",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "src/web/src/components/canvas/uibox.inert.test.ts"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/ui/escapes.guard.adversary2.test.ts"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/ui/escapes.guard.ts"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/components/ui/index.test.ts"
            }
          ]
        },
        {
          "id": "story:verify-two-agent-evidence-in-rust",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            },
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "README.md"
            },
            {
              "confidence": "cited",
              "path": "examples/two-agents/"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-check/"
            },
            {
              "confidence": "cited",
              "path": "website/scripts/spec-facts.mjs"
            },
            {
              "confidence": "cited",
              "path": "website/src/data/spec-facts.json"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:open-blocks-the-runtime",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/runtime/ess-runtime/src/store.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/state.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/swarm.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/tests/open_does_not_queue_behind_an_unrelated_slug.rs"
            }
          ]
        },
        {
          "id": "story:uitable-without-rows",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "src/web/package.json"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/canvas/UiBox.vue"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/components/canvas/uibox.inert.test.ts"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/ui/UiTable.vue"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:pause-and-stop-control-runtime-work",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/core/domains/manager.yaml"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/coordinator.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/http.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/state.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/swarm.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/trigger.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/tests/pause_and_stop_control_runtime_work.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:show-command-issuers-in-the-canvas",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/swarm.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/tests/canvas_command_issuers.rs"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/components/runtime/EventLog.vue"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/lib/event-rows.test.ts"
            },
            {
              "confidence": "inferred",
              "path": "src/web/src/lib/event-rows.ts"
            },
            {
              "confidence": "cited",
              "path": "src/web/src/runtime.ts"
            }
          ]
        },
        {
          "id": "story:slug-is-not-validated",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "src/runtime/ess-runtime/src/store.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/http.rs"
            },
            {
              "confidence": "cited",
              "path": "src/runtime/swarm-server/src/state.rs"
            },
            {
              "confidence": "inferred",
              "path": "src/runtime/swarm-server/tests/open_is_one_swarm_under_contention.rs"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:an-event-cannot-say-which-agent-acted",
      "b": "story:open-blocks-the-runtime",
      "path": "src/runtime/ess-runtime/src/store.rs",
      "confidence": "cited"
    },
    {
      "a": "story:an-event-cannot-say-which-agent-acted",
      "b": "story:pause-and-stop-control-runtime-work",
      "path": "src/runtime/swarm-server/src/http.rs",
      "confidence": "cited"
    },
    {
      "a": "story:an-event-cannot-say-which-agent-acted",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/ess-runtime/src/store.rs",
      "confidence": "cited"
    },
    {
      "a": "story:an-event-cannot-say-which-agent-acted",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/http.rs",
      "confidence": "cited"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:honour-view-consistency",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:honour-view-consistency",
      "path": "src/web/src/runtime.ts",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:open-blocks-the-runtime",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:pause-and-stop-control-runtime-work",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:run-a-tool-box",
      "path": "src/core/domains/blackbox.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:show-command-issuers-in-the-canvas",
      "path": "src/web/src/runtime.ts",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connectors-as-service-boxes",
      "b": "story:verify-two-agent-evidence-in-rust",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:honour-view-consistency",
      "b": "story:open-blocks-the-runtime",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:honour-view-consistency",
      "b": "story:pause-and-stop-control-runtime-work",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:honour-view-consistency",
      "b": "story:show-command-issuers-in-the-canvas",
      "path": "src/web/src/runtime.ts",
      "confidence": "cited"
    },
    {
      "a": "story:honour-view-consistency",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:inert-costs-a-table-its-readers",
      "b": "story:the-guard-reads-no-template-or-style",
      "path": "src/web/src/components/canvas/uibox.inert.test.ts",
      "confidence": "inferred"
    },
    {
      "a": "story:inert-costs-a-table-its-readers",
      "b": "story:uitable-without-rows",
      "path": "src/web/src/components/canvas/UiBox.vue",
      "confidence": "cited"
    },
    {
      "a": "story:inert-costs-a-table-its-readers",
      "b": "story:uitable-without-rows",
      "path": "src/web/src/components/canvas/uibox.inert.test.ts",
      "confidence": "inferred"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:pause-and-stop-control-runtime-work",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:pause-and-stop-control-runtime-work",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:run-a-tool-box",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:show-command-issuers-in-the-canvas",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/ess-runtime/src/store.rs",
      "confidence": "cited"
    },
    {
      "a": "story:open-blocks-the-runtime",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:run-a-tool-box",
      "path": "src/runtime/swarm-server/src/coordinator.rs",
      "confidence": "cited"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:run-a-tool-box",
      "path": "src/runtime/swarm-server/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:run-a-tool-box",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:run-a-tool-box",
      "path": "src/runtime/swarm-server/src/trigger.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:show-command-issuers-in-the-canvas",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "cited"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/http.rs",
      "confidence": "cited"
    },
    {
      "a": "story:pause-and-stop-control-runtime-work",
      "b": "story:slug-is-not-validated",
      "path": "src/runtime/swarm-server/src/state.rs",
      "confidence": "cited"
    },
    {
      "a": "story:run-a-tool-box",
      "b": "story:show-command-issuers-in-the-canvas",
      "path": "src/runtime/swarm-server/src/swarm.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:the-guard-reads-no-template-or-style",
      "b": "story:uitable-without-rows",
      "path": "src/web/src/components/canvas/uibox.inert.test.ts",
      "confidence": "inferred"
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Checker final correction and integration

Correction 7d5c596 is merged at 271242a after direct coordinator review under the two-attack limit. The package grows 123 → 135 executed, all passing; formatter, strict Clippy and original demonstration comparisons exit 0. Pass-1 file hash is unchanged; pass-2 behavioral assertions and expectations are preserved (no byte-preservation claim after formatting). Both final findings have fixed outcomes in the store. Report: /home/timo/.cache/swarm-wave-2026-09-13c/checker/correction-2-report.md.

The local parser preserves Unicode category quoting, nonfinite constants, escaped lone surrogates, duplicate keys and ordinary malformed-grammar rejection. Eleven added groups compare 88 baseline JSON/text/exit scenarios; a unit test rejects 100,000-level incomplete structures without recursive stack growth. Measured accepted nesting through 30,000 is preserved. Python's environment-dependent stack-exhaustion threshold is not emulated; surrogate text errors use a concise diagnostic rather than its traceback. No global serde map-order/precision feature changed. The original 83 mapped scenarios and all earlier review regressions remain.

All 17 historical input hashes remain unchanged; all seven clauses met and unattended false remain the original verdict. Website facts were rederived from the combined tree before the gate. Both source trees are retained unpublished; A's reproducible target was removed after its verified handoff. Final gate, main merge and remaining output cleanup follow.

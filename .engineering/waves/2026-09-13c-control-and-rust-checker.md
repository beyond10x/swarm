# Next wave proposal — 2026-09-13c

## Planning verification

This change edits planning and wave notes only. No runtime implementation or dependency change is included. Whitespace validation, cargo fmt --all --check, ESS validation, AEP validation and the website facts check each exited 0. The full compilation/test/build gate belongs to the implementation wave and was not rerun for this proposal.

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

Status: **proposed; implementation has not started**. Coordinator: Codex leader.
Skill `aep-drive:wave 0.8.1`; planning skill 0.8.1; installed `aep --version` reports `protocol 0.55.0`.
Interactive run: the operator asked to see the next wave after cleanup. This proposal is the review boundary.
Base: published, clean `main` at `5331fe8e3c338d12f8fde2371f1218f6c3a551a9`.

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
| planning / future integration | swarm-next-wave-plan-20260913 | plan/2026-09-13c; rename to wave/2026-09-13c/integration after approval | /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-next-wave-plan-20260913 | same path /target (not created) | /home/timo/.cache/swarm-next-wave-plan-20260913 | proposal only |
| A | swarm-wave-20260913c-control (planned) | wave/2026-09-13c/control | manager-returned path, expected /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control | that checkout /target | /home/timo/.cache/swarm-wave-2026-09-13c/control | not created |
| B | swarm-wave-20260913c-checker (planned) | wave/2026-09-13c/checker | manager-returned path, expected /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker | that checkout /target | /home/timo/.cache/swarm-wave-2026-09-13c/checker | not created |

The coordinator updates each actual path, branch head and stage on creation/transition.
Current owning session: next-wave-leader; release its planning lease on handoff and reacquire before edits.
The planning branch is local and deliberately retained for the operator to review; the next owner is this leader after approval. No source implementation is stranded elsewhere.

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

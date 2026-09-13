---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13a-unit-a-pass-1
kind: review-result
status: active
title: Adversary, wave 13a unit A, pass 1
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:spawn-a-second-agent
revision: 1
---
Adversary, wave 2026-09-13a unit A, pass 1. Against `story:a-turn-is-confined-by-a-frame` and
`story:spawn-a-second-agent`, working tree
`/home/timo/.local/state/worktree/trees/b10x/swarm/wave-20260913a-a`, uncommitted, base `1da5334`.

## Header, as returned

```
unit: A of wave 2026-09-13a
verdict: NEEDS-CHANGE
cases: executed 93→94, red 1
origin: introduced 7 / pre-existing 0 / undecided 0
clauses: frame 1-5 met, spawn 1-5 met (all ten hold; findings are outside the clause statements)
wrote-outside-worktree: 12 paths, all under the assigned scratch directory
needs-coordinator: yes — who regenerates website/src/data/spec-facts.json
```

One case added, `tests/the_frame_states_the_attempt_it_frames.rs`, the adversary's only written path.
It puts a shell stand-in named `metaharness` on `PATH` — no money, no vendor — and drives
`coordinator::take_a_turn` through the real `Launch::Metaharness` arm, **the arm no existing case
reaches**, then reads the document the runtime actually passed to `--frame`.

## The gate, each exit status read separately

```
cargo fmt -p swarm-server --check                 0
cargo clippy -p swarm-server --all-targets        0
cargo test -p swarm-server --no-fail-fast         101   executed 94, failed 1
ess specify validate --path src/core              0     swarm v1 — 9 file(s), valid
python3 src/core/bin/check-sets-are-emitted.py    0
```

94 − the adversary's 1 = 93, exactly the unit's reported count. Without `--no-fail-fast` `cargo test`
stops after the failing target and reports 81; 94 is the honest number.

## Findings

**F1 — `website/src/data/spec-facts.json` is stale, and it is gate step 11.**
`node website/scripts/spec-facts.mjs --check` exits **1** in the worktree and **0** against a
`git archive` of base `1da5334`. The five moved facts are exactly this unit's diff:
`integrationTestFiles` 18→20, `integrationTests` 5379→6039 (the 660 lines of its two new test files
exactly), `swarmServer` 5377→7118, `runtimeTotal` 7942→9683, `specYaml` 4759→4799.
The file is outside unit A's ownership, so it is the coordinator's to run.
NEEDS-CHANGE / introduced, blocker.

**F2 — `coordinator.rs:930` — a retry's frame tells the model it is a first attempt.**
The sealed frame hardcodes `step.attempt: 1` for a turn the runtime itself numbered attempt `02`.
metaharness renders this into the instruction the model is shown (`Frame::render_instruction`:
"step {index} attempt {attempt}").
*What reaches it*: a retry — `turn_file`'s own doc records one on 2026-09-12.
*Fix named, not applied*: `run_turn` already holds the attempt, because `turn_file` computed it.
CONFIRMED / introduced, warning.

**F3 — `tests/a_turn_is_confined_by_a_frame.rs:207` — the case named for `frame_file` never calls
it.** It calls `with_extension("frame.json")` and `frame::write` itself. `frame_file`
(`coordinator.rs:858`) has exactly one caller, `coordinator.rs:937`, and **no test caller** — point
it anywhere and the case stays green. This is one of the two the unit declared not-red-first; the
declaration was honest, and this one is not a live guard.
CONFIRMED / introduced, warning.

**F4 — `swarm.rs:177` — `CappedRecord`'s two doc comments are swapped against the only call site.**
`.bound` is documented "the unit of work it was refused at" and filled with `"unit"`/`"agent"`;
`.unit` is documented "which fold the caps were applied to" and filled with the goal id
(`trigger.rs:532-537`).
*What reaches it*: **nothing found today** — one caller, which happens to produce the row the JSON
keys expect. The trap is the next caller, and both fields are `pub`.
CONFIRMED / introduced, warning.

**F5 — `coordinator.rs:884` — every agent of a swarm shares one work directory and one
`NOTES.md`.** `work` is `swarm.dir().join("work")` for every agent, and `prompt_for_member` tells
each member "Keep a `NOTES.md` in it: read it first, and update it before you finish".
*What reaches it*: **this unit's own reason for existing** — `story:a-recorded-run-shows-two-agents-working`
runs two members in one swarm. Clause 5 separated the transcripts; nothing separated the work
directories.
CONFIRMED / introduced, warning.

**F6 — `state.rs:701` — the claim moved to `Unit{kind,id}` and the capped map did not.**
`report_capped`/`forget_capped` key on `{swarm}/{bare id}` while the in-flight claim moved to
`Unit{kind,id}` in the same diff, for the reason `state.rs:118` gives — that "they will never be
equal" is enforced by nothing. Assignment ids now flow into a map called `capped_goals`.
*What reaches it*: two UUIDs colliding, which nothing reaches; and a `/status` reader who sees an
assignment id under `goal`.
CONFIRMED / introduced, note.

**F7 — `coordinator.rs:786-802` — a member runs on the coordinator's knobs.** A member's turn is
bounded by `SWARM_COORDINATOR_MAX_TURNS`, priced by `SWARM_COORDINATOR_BUDGET_USD` and run on
`SWARM_COORDINATOR_MODEL`. `story:spawn-a-second-agent`'s Scope calls those four "not reusable…
because a second agent needs its own".
CONFIRMED / introduced, note.

## Attacked, could not break — and this is the load-bearing half

- **The hand-written SHA-256.** **2,517 inputs against `sha2::Sha256`: 0 mismatches.** Every length
  0–300 (so 55/56/63/64/119/120/127/128 all crossed), `0xff` blocks at ten lengths, 1 KB / 4 KB /
  100 KB / 1,000,003 B, 2,000 pseudo-random at a fixed seed, and the empty input.
- **The seal for documents other than the fixture.** 11 varied frames — one operation, the whole
  vocabulary, no operations, unicode slug and obligations, a quote and a backslash in a path,
  `index = attempt = u32::MAX`, a relative work dir, an empty work dir, duplicated operations — all
  pass `Frame::parse_document` with the digest intact and re-serialise byte-equal.
  **`metaharness-protocol/src/frame.rs` is byte-identical at tag 0.6.4 and in the 0.7.0 checkout**,
  so this covers the installed binary and the version gap is not one.
- **Frame clause 3 against metaharness's own matcher**, not the unit's JSON assertion: 17 subject
  probes through `SubjectScope::verdict`. All five documented rules hold — the work directory and
  everything under it admitted; `/etc/passwd` and the repository itself refused; `..`, `../x`,
  `a/../../secret` and `…/work/../../other` refused; relative non-climbing paths admitted; `proc:`
  and `host:` refused; and a shell call with no subject `Silent`, which is why `swarm do` still
  works. The doc comment's claim that a shell call carries no subject on the claude arm is true:
  `subjects_of_vendor_call` reads only `file_path`/`notebook_path`/`path` and deliberately never
  `command`.
- **`Spent::absorb`'s event shape** — a real `Event::ToolDecided{Decision::Deny}` serialised from
  metaharness-protocol matches the hand-written fixture exactly, nested `decision.decision` included.
  An allow and an abstain are not counted.
- **`--decisions observe` is absent from every code path in the tree.** The only metaharness launch
  is `metaharness_argv(asked, Some(&frame_at))` guarded by `frame::write(...)?`; no fallback, no
  default, no error path around it.
- **`AssignmentTaken` is issued by `trigger::take_it` inside `one_assignment`, reached from
  `trigger::work_the_assignments`** — the runtime's own loop function, which `run()` calls each
  period. The test calls that function, not `issue`.
- **The claim on `Unit`.** A goal and two assignments claim independently; the same unit twice is
  refused; `Assign` runs from `Spawned`/`Idle` only, so one agent cannot hold two open assignments
  and cannot be launched twice in a period.
- **`capped.jsonl`.** One row per fired bound per unit per process; both `report_capped` call sites
  refuse the turn before writing, so no row exists without a refusal; `forget_capped` has no caller.

```findings
- file: website/src/data/spec-facts.json
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    gate step 11 (node website/scripts/spec-facts.mjs --check) exits 1 on this tree and 0 on a
    git-archive of base 1da5334; the five stale counts are exactly this unit's two new test files
    and its Rust and YAML line growth, and the fix is `cd website && npm run spec:facts`.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 930
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the sealed frame hardcodes step.attempt=1 while turn_file has already numbered the turn attempt
    02, so on the retry its own doc records happening on 2026-09-12 metaharness renders "attempt 1"
    into the instruction the model is shown.
- file: src/runtime/swarm-server/tests/a_turn_is_confined_by_a_frame.rs
  line: 207
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the case named for keeping the frame beside the transcript takes no turn and reimplements
    frame_file itself, so coordinator.rs:858 has no test caller and could name any path with the
    suite still green.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 177
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    CappedRecord's `bound` and `unit` doc comments are swapped against the only call site
    (trigger.rs:532-537), so a second caller following the documentation would write a capped.jsonl
    row with the unit id and the fold name in each other's keys.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 884
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    every agent of a swarm is given the same work directory and the member prompt tells each of them
    to keep a NOTES.md in it, so the two-agent demonstration this unit exists to enable has two
    members overwriting one file.
- file: src/runtime/swarm-server/src/state.rs
  line: 701
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the in-flight claim moved to Unit{kind,id} because equal ids are enforced by nothing, while
    report_capped, forget_capped and CappedGoal in the same diff still key assignment ids on the
    bare goal key.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 786
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    a member's turn is bounded, priced and modelled by the four SWARM_COORDINATOR_* knobs that
    story:spawn-a-second-agent's Scope calls not reusable because a second agent needs its own.
```

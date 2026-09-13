---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13a-unit-a-pass-2
kind: review-result
status: active
title: Adversary, wave 13a unit A, pass 2
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:spawn-a-second-agent
revision: 1
---
Adversary, wave 2026-09-13a unit A, **pass 2**. Against `story:a-turn-is-confined-by-a-frame` and
`story:spawn-a-second-agent`, worktree `…/wave-20260913a-a`, uncommitted, base `1da5334`.

## Header, as returned

```
unit: wave 2026-09-13a unit A, correction round 1
verdict: NEEDS-CHANGE
cases: executed 135→141, red 4
origin: introduced 9 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: none
```

Pre-flight verified: the pass-1 file is byte-identical (`0920d12c…`), `frame_file` is private, and the
two existing test files the unit touched changed 2 lines each — mechanical API updates
(`ensure_coordinator`→`ensure_agent`), not weakenings. Six cases added in
`tests/the_second_agent_under_attack_pass_2.rs`, the adversary's only written path. `<before>` = 135,
measured by deselecting them by exact name after a first attempt with shorter patterns over-matched.

## Findings

**A — `coordinator.rs:543` — a config the coordinator drafts for itself widens its own frame.**
`from_config` takes `admitted_operations` verbatim; the only check is `frame::document`'s, against the
ten-name `VOCABULARY` rather than the seven-name `frame::ADMITTED`. A config naming all ten resolves
to all ten, gaining `subagent.spawn`, `task.todo` and `web.read` — two of which `ADMITTED`'s own doc
says are deliberately absent.
*What reaches it*: `prompt_for` tells every coordinator that `swarm do` reaches every command, and
`config.yaml`'s own new comment names `DraftConfig` and `ActivateConfig` as reachable that way — it is
the unit's argument for keeping the **subject scope** out of the config. So `config.yaml`'s claim
that "the worst a config can do to it is admit less" is false in the same commit that added it.
NEEDS-CHANGE / introduced, **blocker**.

**B — `trigger.rs:248` — the unit bound is fed the agent's figure and misreports which record was
capped.** `work_the_assignments` passes `spend_by_agent(agent).1` as `bounded`'s per-unit
`iterations`. With three rows for `builder` on one assignment and none on a second,
`bounded(caps{3}, …, "assignment-the-second", 3)` fires as `Bound::Goal` with `turns=3`.
*What reaches it*: every period, for any member with prior work. A retasked member is refused at its
first turn at the new assignment, and `record_capped` writes `bound: "unit"` — the shape `Capped`'s
own doc records as the 2026-09-12 defect it was created to end.
CONFIRMED / introduced, warning.

**C — `coordinator.rs:361` — `Launch::describe` reads the coordinator's model for a member's turn.**
`run_turn` logs `describe()` for every turn, and it is `/status`'s `coordinator.program`. The argv is
correct — the unit's own case proves no `SWARM_COORDINATOR_*` knob reaches a member — so the knob
leaks into the only account an operator sees.
CONFIRMED / introduced, warning.

**D — `frame.rs:228` — rule 4's refusal is switched off by any admitted subject in the same call.**
`file.write` on `[file:<own>/a, file:/etc/passwd]` → `Admitted`; `/etc/passwd` alone → `Refused`.
Judged by a **verbatim port of metaharness 0.7.0's own `glob_matches`/`SubjectScope::verdict`**,
validated green first on 13 single-subject claims from `frame::scope`'s doc.
*What reaches it*: **nothing found.** `subjects_of_vendor_call` builds the list from `file_path`,
`notebook_path` and `path`; the adversary could not establish that Claude Code emits a tool input
carrying two, and would not make a paid run to find out. It built the two-subject call itself.
INFEASIBLE / introduced, warning. The defect is in the document: rule 4 is stated as "the rule that
makes the scope a confinement rather than a list of verbs" with no mention of the condition it holds
under.

**E — `swarm.rs:144` — `What::Capped` is the one variant that did not gain `agent`.** `CappedGoal`
gained `unit` and `agent`, so `/status` is right; the watch stream publishes an assignment UUID in a
field named `goal`, with no agent and no kind, to the one surface a person watches live.
CONFIRMED / introduced, warning.

**F — `src/web/src/runtime.ts:112` — the only consumer was not told.** The unit kept
`CappedGoal.goal`'s name expressly because `runtime.ts` reads it, then did not add the new fields
there. The TS `turn` event has no `agent`, the `agent` event has no `agent`, and `interface
CappedGoal` has neither `unit` nor `agent`. The publisher can now distinguish two members and the
only consumer cannot, and nothing in the gate compares the two files.
CONFIRMED / introduced, warning. **Not unit A's surface** — taken by the coordinator at integration.

**G — `coordinator.rs:901` — `agent_directory` collides distinct slugs.** Every non-alphanumeric
becomes `-`, so `qa_1` and `qa-1` share one directory and one transcript-name space — F5's defect for
those names.
*What reaches it*: **nothing found** — all eleven historical logs use plain alphanumeric names.
INFEASIBLE / introduced, note.

**H — `coordinator.rs:964` — one live swarm is harmed by the unmigrated `work/NOTES.md`.** Measured
in the primary checkout: four swarms hold one, and **`e2e-1789212621` is live** — last lifecycle event
`SwarmResumed`, `work/NOTES.md` 3,812 bytes. The coordinator now starts in `work/coordinator/` and is
told to read a `NOTES.md` that is not there. The file stays readable under rule 3, but the prompt
describes the shared directory as one directory per agent, which does not name a loose file at its
root. The loss is the pointer, not the bytes, and it is not visible.
CONFIRMED / introduced, note.

**I — `coordinator.rs:337` — `resolve(swarm)` swarm-wide: the reason is sound, the type name is not.**
`Launch::Metaharness { admitted }` carries a **permission** inside a type named for a program, and
swarm-wide resolution is how one coordinator's self-written config sets every member's frame. That is
finding A's blast radius and the argument for fixing A rather than renaming this.
CONFIRMED / introduced, note.

## Attacked and could not break

- **`frame::scope`, rules 1–6, single subject**: 13 assertions through the verbatim port of
  metaharness's matcher. A member cannot write anywhere under the shared root, cannot reach a peer's
  directory, cannot climb out by `..` absolute or relative, cannot read the swarm's `turns/` or
  `eventlog.sqlite3`; `proc:` is refused.
- **F2 on a third attempt**: `step.attempt` is 3, beside `0001-03-coordinator-…`.
- **F3's coupling is genuine**: the case asserts the `--frame` path against a literal derived from the
  transcript's name, not from `frame_file()`. Pointing `frame_file` elsewhere moves one side only.
- **F7's argv half**: no `SWARM_COORDINATOR_*` knob reaches a member's argv; a member with nothing set
  gets `--max-turns 30`, `--max-budget-usd 1.00`, no `--model`, no `--effort`. Only `describe` leaks.
- **F6's collision**: `Unit::key` puts the kind in the key, so a goal and an assignment sharing a UUID
  are two claims and two cap reports.
- **F5's directories**: `write_settings`, `turn_file`, `frame_file` and the mailbox are all per-agent.
- `--cwd` is the agent's own directory and `repository_root()` canonicalises, so the scope's
  absolute-path assumption holds in the shipped `main`.

## How it was routed

A, B, C, E fixed. D is a doc line — INFEASIBLE, nothing found reaches it, and the unit then measured
that no affordable fix exists: refusing the two-subject pair needs a rule matching every path outside
the work tree and not inside it, which in a matcher with only `*`, `**` and literals is **14,848
patterns per swarm**, sealed into every frame, against a type whose stated purpose is a boundary
somebody can read at a glance. The adversary's case is re-pinned to assert today's state on the
coordinator's instruction, recorded in the case's own doc comment — not `#[ignore]`d, which
pre-excuses its own red, and not merged red, which would take the suite's exit status for everything
after it. G closed with a digest-suffixed slug rather than documented. H answered with one sentence in
the prompt. F taken by the coordinator. I needs no work once A is fixed.

```findings
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 543
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    from_config takes admitted_operations verbatim without intersecting frame::ADMITTED, so a
    coordinator that drafts and activates its own config widens its own sealed frame to
    subagent.spawn, task.todo and web.read, contradicting config.yaml's "the worst a config can do
    to it is admit less" added in the same commit.
- file: src/runtime/swarm-server/src/trigger.rs
  line: 248
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    work_the_assignments passes the agent-wide spend_by_agent count as bounded's per-unit
    iterations, so a retasked member is refused at its first turn under Bound::Goal and
    record_capped writes bound="unit" with turns from a record the unit has none of.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 361
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Launch::describe reads SWARM_COORDINATOR_MODEL unconditionally and run_turn logs it for every
    turn including a member's, so the operator is told a member runs on the coordinator's model
    while metaharness_argv passes it no --model at all.
- file: src/runtime/swarm-server/src/frame.rs
  line: 228
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    rule 4 is documented as "every other absolute path is refused" but metaharness judges a call by
    the first rule any of its subjects matches, so an outside absolute path is admitted whenever the
    same call also names the agent's own file; I built the two-subject call and found no vendor tool
    that emits one, and no rule ordering can fix it here.
- file: src/runtime/swarm-server/src/swarm.rs
  line: 144
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    What::Capped is the one variant that did not gain agent, so the watch stream publishes an
    assignment UUID in a field named goal with nothing saying whose turn was refused, while
    CappedGoal on /status gained both unit and agent.
- file: src/web/src/runtime.ts
  line: 112
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the turn and agent event types and interface CappedGoal were not given the agent, unit fields
    the Rust side now publishes, so the only consumer of the stream still cannot tell two members
    apart and nothing in the gate compares the two files.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 901
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    agent_directory maps every non-alphanumeric character to '-', so agent ids differing only in
    punctuation share one work directory and one transcript-name space, which is the defect F5 and
    clause 5 exist to close; all eleven historical logs use plain alphanumeric names, so nothing
    was shown to reach it.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 964
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the swarm e2e-1789212621 is live (last lifecycle event SwarmResumed) and holds a 3812-byte
    work/NOTES.md that the new per-agent directory leaves unreferenced, and the prompt describes the
    shared directory as one directory per agent, which does not name a loose file at its root.
- file: src/runtime/swarm-server/src/coordinator.rs
  line: 337
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    keeping resolve swarm-wide is right for which binary a swarm runs, but Launch::Metaharness now
    carries a permission inside a type named for a program, which is how one coordinator's config
    comes to set every member's frame.
```

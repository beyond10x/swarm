---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13a-unit-b-pass-2
kind: review-result
status: active
title: Adversary, wave 13a unit B, pass 2
relations:
- reviews: story:a-recorded-run-shows-two-agents-working
revision: 1
---
Adversary, wave 2026-09-13a unit B, **pass 2**. Against
`story:a-recorded-run-shows-two-agents-working`, worktree `…/wave-20260913a-b`, branch
`wave/2026-09-13a/unit-b`, base `1da5334`, untracked working tree.

## Header, as returned

```
unit: unit B of wave 2026-09-13a — examples/two-agents/
verdict: NEEDS-CHANGE
cases: executed 57→62, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none
```

Claim verified first: `md5sum test_adversary.py` = `acb52e714af77dccd25c986c8c9c0b22`, matching the
unit's stated value. Pass 1's five cases still run and still pass. No assertion weakened. One path
added, `test_adversary_pass2.py`, a test file.

Per-lane, run separately because a lane that selects nothing exits 0: author 52 OK; pass 1 5 OK;
pass 2 5 FAILED. `<before>` = 57, measured with the new file deselected.

## Findings

**F1 — `fixtures.py:205` spawns a role the domain has no variant for.** The default fixture writes
`("builder", "Worker")` into `AgentSpawned.role`; `swarm.agent.Role` (`agent.yaml:32-35`) has exactly
one variant, `Coordinator`. `swarm.agent.Worker` exists at `agent.yaml:1241` as an **actor type**,
not a Role variant. The suite's green on clauses 1, 2 and 4 rests on this value.
*What reaches it*: the only spawn site is `swarm.rs:1145`, which hardcodes `role: "Coordinator"`, and
all ten logs under `data/swarms/` carry `Coordinator` on every `AgentSpawned` — `builder`, `reviewer`
and `tester` included. Honest caveat recorded by the adversary: `require_declared_input`
(`apply.rs:199-216`) checks input **presence only, not type**, so a hand-crafted `Spawn` carrying
`role:"Worker"` would not be refused.
CONFIRMED / introduced, blocker.

**F2 — `README.md:85` § *What no run can meet yet* omits clauses 1, 2 and 4.** The section exists to
be read before spending money and names only 5 and 7. Clause 1 needs `role != Coordinator`; clauses 2
and 4 both go through `handovers()`, which gates on `records.workers` — the same role test.
CONFIRMED / introduced, blocker.

**F3 — `check-two-agents.py:533` — clause 4's `looked_for` promises an assignment match the code
never performs.** It prints "for the assignment clause 2 matched"; the code reduces handover pairs to
a set of **agent ids** and filters on membership. No assignment identity is compared, though the
record carries it on both sides. A finish of assignment A satisfies a take of assignment B by the
same agent.
CONFIRMED / introduced, blocker.

**F4 — `:533` — clause 4 has no ordering constraint between the finish and the take.** An
`AssignmentDone` at seq 8 satisfies an `AssignmentTaken` at seq 12 — a finish that predates the work
being posted. `handovers()` requires `take.seq > post.seq`; that care stops at clause 4.
CONFIRMED / introduced, warning.

**F5 — `:341` — `handovers()` pairs posts to takes by `agent_id` alone, so clause 2 cites the wrong
post.** The cross product puts the **oldest** post first, and clause 2 prints `pairs[0]` — the
posting that was abandoned, not the one the take answered. The verdict is right; the citation is not.
*What reaches it*: post, fault, re-post, take — the ordinary retry shape.
CONFIRMED / introduced, warning.

## Attacked and could not break

- **Clause 5's census/record mismatch reporting** — a census claiming any number of denials cannot
  turn a real frame refusal not-met.
- **Both class guards iterate `report.clauses` dynamically**, and `check()`'s
  `assert [numbers] == [1..7]` means an eighth clause breaks the guard rather than dodging it.
- **`Report.ok` and the exit status** — could not construct a swarm meeting zero clauses that exits 0.
- **`finisher_of()`** — could not make it name an agent other than the assignment's owner.
- **All four `agent_of()` paths gate on `set(records.spawned)`**; the gate holds against unspawned
  names on the record, file-name, cwd and spend paths.

## How the coordinator routed it

**F1 and F2 are an artefact of the branch split and were not sent back to unit B.** Unit A's working
tree declares `swarm.agent.Role` variants `Coordinator` **and `Worker`** — the spelling unit B's
fixture anticipated. **The coordinator cited that as settled and was corrected by unit B**: unit A has
committed nothing, so `git show wave/2026-09-13a/unit-a:src/core/domains/agent.yaml` still shows one
variant, and the evidence lives only in an uncommitted working tree. Unit B measured the claim
instead of accepting it, by building a probe tree from unit A's `agent.yaml` plus its own directory
and running pass 2 there: **exit 0, all five green**. F2 was answered by rewriting the README section
to describe the merged tree and say so.

F3, F4 and F5 went back to unit B and are fixed: `handovers()` now pairs on the assignment
(`AssignmentTaken.ref` → `AssignmentRecorded` → the latest post preceding it), clause 2 prints
`pairs[-1]`, clause 4 requires the assignment to match where both sides name one and **says so when
they do not**, and adds `event.seq > take.seq`. Verified by the coordinator reading the diff: both
adversary files byte-identical by md5, `pairs[0]` gone, the `ref`-absent branch present.

**A defect in the story, not in the unit:** clause 4's sentence — "for the assignment clause 2
matched" — was written by the coordinator between unit B's rounds, and `AssignmentPosted` carries no
assignment identity at all. The sentence promised what the record could not supply. The clause now
states its own limit.

```findings
- file: examples/two-agents/fixtures.py
  line: 205
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the default fixture writes AgentSpawned.role "Worker", which swarm.agent.Role
    (agent.yaml:32-35) has no variant for at this base and which none of the ten real logs under
    data/swarms/ produces, and clauses 1, 2 and 4 are green only against it
- file: examples/two-agents/README.md
  line: 85
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the section that exists to stop money being spent on an unwinnable run names only clauses 5
    and 7, but clauses 1, 2 and 4 are equally unmeetable at this base because every AgentSpawned
    this runtime writes carries role Coordinator, the only variant the enum has
- file: examples/two-agents/check-two-agents.py
  line: 533
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    clause 4's looked_for promises "for the assignment clause 2 matched" while hits compares
    agent ids only, so a finish of one assignment satisfies a take of another by the same agent
- file: examples/two-agents/check-two-agents.py
  line: 533
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    clause 4 puts no ordering between the finish and the take, so an AssignmentDone recorded
    before the assignment was posted reports the work finished by the agent that took it
- file: examples/two-agents/check-two-agents.py
  line: 341
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    handovers() pairs posts to takes by agent_id alone and clause 2 prints pairs[0], the oldest
    post, so the evidence names the posting that was abandoned rather than the one the take answered
```

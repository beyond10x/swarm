---
format: aep.planning-md/1
id: review-result:plan-critic-acceptance-goal-round-2
kind: review-result
status: active
title: Acceptance critic, goal decomposition, round 2
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Acceptance critic, round 2, over the revised set. Returned as the critic returned it.

---

needs-revision

story:a-turn-is-confined-by-a-frame — clause 4 chains two independently-failable behaviors ("never starts a process" for a bad-digest frame, and "refuses to launch a turn with no frame at all") under one number, so a run where the digest check works but the missing-frame path silently falls back to `--decisions observe` reports neither met-4 nor unmet-4 — .engineering/planning/story/a-turn-is-confined-by-a-frame.md:73-74
story:a-recorded-run-shows-two-agents-working — clause 6 chains two independently-failable facts ("spend rows for both agents" and "the agent ceiling refusing one of them") under one number, so per-agent recording could work while per-agent enforcement doesn't (or vice versa) and clause 6 alone can't say which — .engineering/planning/story/a-recorded-run-shows-two-agents-working.md:65-66

What I read: all four artifacts in full via `aep plan artifact show` (the three stories' revised Acceptance sections and the epic's new body), `aep plan artifact kinds` and `lifecycle story`/`lifecycle epic` to confirm `implemented` is a real terminal status (so the epic's "Done When… are implemented" is a status reference, not vague prose), and cross-checked cited symbols/commands against the tree: `git grep` for `metaharness_argv`, `turn_file`, `"--decisions"`, and the event names (`AgentSpawned`, `AssignmentPosted`, `AssignmentTaken`, `MailboxOpened`, `GateGreen`, `FinishAssignment`, `spend_by_agent`) in `src/core/domains/*.yaml` and `src/runtime/swarm-server/src/*.rs`; ran `metaharness run claude --help` and `ess specify validate --help` to confirm both are real, invocable commands; and listed `data/swarms/*/turns` to confirm the event-log paths cited exist. 4 of 4 artifacts read in full.

What I could not establish: whether the installed `metaharness` binary (0.6.4) versus the 0.7.0 the frame mechanism is described against (`a-turn-is-confined-by-a-frame.md`, "The mechanism, read from metaharness 0.7.0") changes what clause 4 can actually observe today — this is a version-pinning/design risk already named in the epic's own Risks section, not something I can resolve from acceptance wording alone, and it is out of my lane (design coupling, not acceptance checkability) so it did not set my verdict. All other clauses I checked (story:a-turn-is-confined-by-a-frame clauses 1–3, story:a-recorded-run-shows-two-agents-working clauses 1–5 and the "unattended" operational definition, story:spawn-a-second-agent clauses 1–5) hold up as single, independently-checkable statements — the round-1 chained-conjunction defect is otherwise resolved.

```findings
- file: .engineering/planning/story/a-turn-is-confined-by-a-frame.md
  line: 73
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "clause 4 chains two independently-failable behaviors (a bad-digest frame never starting a process, and a missing frame being refused at launch) under one number, so a run can satisfy one half and fail the other while clause 4 reports neither met nor unmet"
- file: .engineering/planning/story/a-recorded-run-shows-two-agents-working.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "clause 6 chains two independently-failable facts (spend rows recorded for both agents, and the agent ceiling refusing one of them), so per-agent recording can pass while per-agent enforcement fails, or the reverse, with no way to say which happened"
```

**Coordinator's note, added when recording.** Both findings are closed: the frame story's acceptance
is now five clauses, with the missing-frame refusal split out as clause 5 and its own reasoning
("`observe` became the status quo by being the thing nobody had to ask for"); the demonstration
story's is now seven, with per-agent recording (6) split from per-agent enforcement (7).

The version discrepancy this critic raised under *what I could not establish* was **also a real
defect and is corrected**: the mechanism was read from the 0.7.0 source checkout while the installed
binary is 0.6.4. The section now says so, and records that the coordinator's probe ran against the
installed 0.6.4 and that the mechanism works there. It was raised out of lane and was right.

---
format: aep.planning-md/1
id: review-result:plan-critic-acceptance-goal-round-1
kind: review-result
status: active
title: Acceptance critic, goal decomposition, round 1
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Acceptance critic, round 1, over the set drafted 2026-09-13 from the goal "an autonomous, sandboxed
multi-agent swarm, proven". Returned as the critic returned it, including its own corrected first
line.

---

approve

Wait — that's wrong; findings exist below. Correcting to the actual verdict.

needs-revision

- story:a-turn-is-confined-by-a-frame — the acceptance packs four independently-failable claims into one paragraph (turn launches with `--decisions frame`; a disallowed operation is refused and reaches the event record; an out-of-scope write is refused by the subject scope; a digest mismatch is refused before any process starts) so a run that satisfies three of the four is neither "done" nor "not done" under this text — .engineering/planning/story/a-turn-is-confined-by-a-frame.md:65-69
- story:a-recorded-run-shows-two-agents-working — the acceptance is a "shows **all** of" list joining six independently-checkable outcomes (non-Coordinator spawn, assignment taken, two distinct transcript files, a finish/gate event, a frame refusal, an agent-ceiling refusal) into a single conjunctive check, so a real, expensive, money-costing run that clears five of six leaves no way to say which single fact still needs a re-run — .engineering/planning/story/a-recorded-run-shows-two-agents-working.md:60-70
- story:spawn-a-second-agent — the acceptance is one sentence chaining five outcomes with "and" (a spawned agent with a non-Coordinator role; that agent gets a mailbox; the runtime runs it as its own metaharness session against an assignment; two agents work concurrently; each transcript is recorded separately), so a partial result — e.g. the agent runs but its transcript still overwrites the coordinator's, which the story's own Scope section names as a live risk (`turn_file()`'s name carries no agent) — has no way to register as anything but a vote on the whole sentence; unchanged by today's revision, which only rewrote `## Notes` (lines 46-70) — .engineering/planning/story/spawn-a-second-agent.md:37-39

What I read: all three artifacts in full via `aep plan artifact show`; `aep plan artifact kinds` and `aep plan artifact lifecycle story`; `AGENTS.md` at the repo root; the critic rubric; `git diff HEAD -- .engineering/planning/story/spawn-a-second-agent.md` and `git log --oneline` on that path to confirm today's edit was confined to `## Notes`; `grep -n "decisions\|frame"` in `src/runtime/swarm-server/src/coordinator.rs` to confirm no `--decisions frame` path exists yet (so story 1's acceptance is a real before/after transition, not a no-op).

What I could not establish: whether "unattended from start to verdict" (story:a-recorded-run-shows-two-agents-working, line 60) has an operational definition anywhere in the repo that a reviewer could check against a log — I found none, but did not raise it as a separate finding since the six-clause bundling finding already forces a rewrite of that acceptance. Whether clause 6 of that same acceptance ("the agent ceiling refusing one of them") is achievable at all given story:spawn-a-second-agent's own Notes say the per-agent in-flight claim is still owed is a cross-artifact coupling question — out of my lane (plan-critic-design's), noted here only so it isn't lost, and it does not affect my verdict.

```findings
- file: .engineering/planning/story/a-turn-is-confined-by-a-frame.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance packs four independently-failable claims into one paragraph (decisions-frame launch, operation refusal reaching the event record, subject-scope refusal, digest-mismatch refusal at start), so a run satisfying three of four is neither done nor not done under this text"
- file: .engineering/planning/story/a-recorded-run-shows-two-agents-working.md
  line: 60
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the shows-all-of list joins six independently-checkable outcomes (spawn, assignment taken, two distinct transcripts, finish or gate, frame refusal, agent-ceiling refusal) into one conjunctive check, so a real costly run that clears five of six has no way to say which single fact still needs a re-run"
- file: .engineering/planning/story/spawn-a-second-agent.md
  line: 37
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "the acceptance is one sentence chaining five outcomes with and (non-Coordinator role, mailbox, its own metaharness session, concurrent work, separate transcripts), so a partial result such as the agent running but its transcript still overwriting the coordinator's has no way to register as anything but a vote on the whole sentence"
```

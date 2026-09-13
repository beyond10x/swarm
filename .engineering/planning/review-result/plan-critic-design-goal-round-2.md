---
format: aep.planning-md/1
id: review-result:plan-critic-design-goal-round-2
kind: review-result
status: active
title: Design critic, goal decomposition, round 2
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Design critic, round 2, over the revised set. Returned as the critic returned it.

---

**needs-revision**

story:spawn-a-second-agent — its own body says the order is fixed ("Sequence them, that story first", story/spawn-a-second-agent.md:100) and `story:a-turn-is-confined-by-a-frame` says the same in the other direction ("The order is this story first", story/a-turn-is-confined-by-a-frame.md:110), but `relations:` (story/spawn-a-second-agent.md:7-8) carries only `decomposes` — no `depends_on` on `story:a-turn-is-confined-by-a-frame` — so the ordering constraint both bodies agree on is invisible to anything that reads the graph rather than the prose — `aep plan artifact graph`

What I read: all 4 artifacts in full (`aep plan artifact show` on `story:a-recorded-run-shows-two-agents-working`, `story:spawn-a-second-agent`, `story:a-turn-is-confined-by-a-frame`, `epic:executable-boxes`), `aep plan artifact relations`, `aep plan artifact graph` (walked all 51 printed edges, including the ones to `vision:swarm-builds-itself` and the 16 other stories outside this set — no back-edge into this trio from anywhere outside it), `aep plan artifact validate`, and the two round-1 review-results (`plan-critic-design-goal-round-1`, `plan-critic-parallel-safety-goal-round-1`) to see what was already raised and what "fixed" meant here. Grepped raw frontmatter of both colliding stories to confirm line numbers.

What I could not establish: whether the missing `depends_on` edge is a deliberate choice left to the wave coordinator rather than the store (the epic's "Scope" section already numbers the three capabilities "in the order their dependencies allow," which argues the author knows the order and simply didn't encode it on the two stories directly) — I read that as supporting the finding, not excusing it, but the choice between an edge and the epic's ordering prose is close enough to note. The `verification-report` kind question is not a finding: `story:a-recorded-run-shows-two-agents-working` now builds two real artifacts (a scenario, a checker) independent of any single run and only files the run's outcome as a future `verification-report`, which answers round 1's objection — `kind: story` is defensible now. Whether the checker's six clauses are independently checkable is `plan-critic-acceptance`'s lane, not mine, and whether `story:a-recorded-run-shows-two-agents-working`'s scope correctly excludes `spec-facts.json` is `plan-critic-scope`'s, not mine — I read both only to confirm this round's fix didn't reopen the design question.

```findings
- file: .engineering/planning/story/spawn-a-second-agent.md
  line: 7
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its body and story:a-turn-is-confined-by-a-frame's body agree on a fixed order ('Sequence them, that story first' / 'The order is this story first') because both edit the same 35-line function, but relations: carries only decomposes -- no depends_on on story:a-turn-is-confined-by-a-frame -- so the ordering constraint is invisible to anything reading the graph rather than the prose"
```

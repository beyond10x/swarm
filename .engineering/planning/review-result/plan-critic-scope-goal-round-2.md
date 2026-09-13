---
format: aep.planning-md/1
id: review-result:plan-critic-scope-goal-round-2
kind: review-result
status: active
title: Scope critic, goal decomposition, round 2
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Scope critic, round 2, over the revised set, judged against an `epic:executable-boxes` that now has a
real body. Returned as the critic returned it.

---

approve

**What I read:** 4 artifacts — `epic:executable-boxes` (parent, read first, whole body, before any child), `story:spawn-a-second-agent`, `story:a-turn-is-confined-by-a-frame`, `story:a-recorded-run-shows-two-agents-working` (all via `aep plan artifact show <id>`), plus `aep plan artifact graph` to check for other claimants of the parent's outcomes.

**Promise extraction:** 4 promises extracted from the parent, 4 traced to an item.

1. "A role that is not `Coordinator`, and a runtime that runs it" (Scope §1, `agent.yaml`'s `Assign`/`TakeAssignment`/`FinishAssignment`) → claimed by `story:spawn-a-second-agent` clauses 1 and 3.
2. "A turn narrowed by a frame it cannot widen" (Scope §2, `--frame`/`--decisions frame`/subject scope) → claimed by `story:a-turn-is-confined-by-a-frame` clauses 1–4.
3. "A demonstration anybody can re-run, with a checker that reads the log and reports each clause by number" (Scope §3) → claimed by `story:a-recorded-run-shows-two-agents-working`'s scenario + checker deliverable.
4. Done When's `verification-report` recording a run against the third story's six clauses, "including the ones it failed" → claimed by the same story's "The run itself is then evidence" / "Then correct what is published" sections, which name the same fallback (report says it wasn't met; `AGENTS.md`'s sentence stays).

The Why Now table's own word-by-word attribution (multi-agent → `spawn-a-second-agent`, sandboxed → `a-turn-is-confined-by-a-frame`, proven → `a-recorded-run-shows-two-agents-working`) matches this mapping exactly, so I count it as the same three promises restated rather than additional ones.

Checked each Out-of-Scope exclusion against all three bodies: kernel confinement (the frame story explicitly disclaims it, matching the epic's own wording), agents spawning agents / more than two agents (both relevant stories carry matching "depth one, breadth two" exclusions), a concurrency ceiling (`spawn-a-second-agent` clause 4 re-keys an in-flight claim to stop incidental serialization — it does not add a `turns_in_flight()` refusal, so it is not the excluded ceiling), and the UI box library / Service box (untouched by all three — those live in other draft/implemented stories outside this set, per the graph, e.g. `story:render-a-ui-box`, `story:connectors-as-service-boxes`). No item claims an excluded outcome, and no two items claim the same promise (the third story `verifies` the other two rather than re-claiming their outcomes).

The Outcome's "a person watching can see it happen and stop it" is not a separate uncovered promise: "see it happen" is the event-log/checker observability the third story delivers, "stop it" is the frame's decision-seam refusal the second story delivers — both already counted under promises 2 and 3.

**What I could not establish:** none.

The `metaharness_argv` collision between the first two stories (both note it and sequence themselves) is real but is a parallel-safety concern, not a coverage one — out of my lane, and it does not change this verdict.

```findings
[]
```

---
format: aep.planning-md/1
id: review-result:plan-critic-scope-goal-round-1
kind: review-result
status: active
title: Scope critic, goal decomposition, round 1
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Scope critic, round 1, over the set drafted 2026-09-13 from the goal "an autonomous, sandboxed
multi-agent swarm, proven". Returned as the critic returned it.

---

`approve`

**What I read** — 5 artifacts in full (`epic:executable-boxes`, `vision:swarm-builds-itself`, `story:a-turn-is-confined-by-a-frame`, `story:a-recorded-run-shows-two-agents-working`, `story:spawn-a-second-agent`), plus `story:spend-is-bounded-per-goal-only` (pre-existing, checked because `story:a-recorded-run-shows-two-agents-working`'s clause 6 leans on per-agent spend bounding) and the full `aep plan artifact graph` output, `aep plan artifact kinds`, `aep plan artifact relations`.

**Promise extraction** — `epic:executable-boxes` is confirmed empty (every section — Outcome, Why Now, Scope, Out of Scope, Risks, Ambiguities, Done When — still holds the unfilled italic template prompt; only its one-line summary, "A drawn box runs, renders or reaches a service, and an agent can create one," is real text, and it names none of "autonomous," "sandboxed," "multi-agent," or "proven"). An empty parent supplies no promises to check a gap against and no exclusions to check reach against, so I did not treat it as the thing being covered. `vision:swarm-builds-itself` has real content but a different frame (self-extension, a watcher who can stop it) and likewise never uses the operator's three adjectives. The actual text I judged the set against is the operator's goal as relayed in this task, which is not written into any artifact body — I extracted 4 promises from it: **autonomous**, **sandboxed**, **multi-agent**, **proven**. I traced 4 of 4 to the set:

- *autonomous* — `story:a-recorded-run-shows-two-agents-working` Acceptance: "One recorded run of one swarm, **unattended from start to verdict**..." This is a fresh demonstration, not a restatement of the coordinator's cited history.
- *sandboxed* — `story:a-turn-is-confined-by-a-frame`, refusal at the decision seam; the story itself names the narrower claim it is *not* making ("Do not describe this story's outcome as containment, sandboxing in the kernel sense, or an escape-proof boundary") — an honest narrowing, not a silent one.
- *multi-agent* — `story:spawn-a-second-agent`, a role other than `Coordinator` that the runtime actually runs.
- *proven* — `story:a-recorded-run-shows-two-agents-working`'s six-clause acceptance plus its requirement that the run be recorded as a `verification-report`.

On the coordinator's claim specifically: citing 109 `GoalPursued`/106 `GoalNotReached` and `dsfsdf`'s 38 unattended turns as reason to skip an "autonomous" story is evidence about a single-agent, unsandboxed swarm — on its own it would not establish the *compound* claim the operator asked for. But the set does not actually rest on that citation: `story:a-recorded-run-shows-two-agents-working` requires a **new** unattended run, integrated with the multi-agent and frame-refusal evidence, so the compound claim is re-earned inside the set rather than assumed from history. The coordinator's stated rationale is weaker than what the artifacts themselves deliver, but nothing is left unclaimed as a result.

No duplicate claims (each story owns a distinct outcome: build confinement / build second-agent execution / demonstrate both together), and the "breadth two, depth one" narrowing is stated as a deliberate Out-of-Scope line in two of the three stories, not silent.

**What I could not establish** — the operator's verbatim goal exists nowhere in the store as a citable sentence (not in the epic, not in the vision), so had I found a gap or reach defect I could not have given it a `path:line` inside the repository for the promise's source itself, only for the item making or missing the claim. I could not confirm whether `story:a-recorded-run-shows-two-agents-working`'s acceptance clause 6 ("the agent ceiling refusing one of them") is actually delivered by `story:spawn-a-second-agent`'s own Acceptance text, which does not mention spend caps at all — the capability may already exist from the 2026-09-12d wave per `story:spawn-a-second-agent`'s own notes, or it may not be closed by anything in this set. That is a dependency/acceptance-completeness question, not a scope-against-parent question (the operator's goal never mentions spend caps), so I leave it for `plan-critic-design` or `plan-critic-acceptance` and it does not affect this verdict.

```findings
[]
```

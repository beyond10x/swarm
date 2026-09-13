---
format: aep.planning-md/1
id: review-result:plan-critic-parallel-safety-goal-round-2
kind: review-result
status: active
title: Parallel-safety critic, goal decomposition, round 2
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Parallel-safety critic, round 2, over the revised set. Returned as the critic returned it.

---

approve

**What I read:** 3 artifacts — `story:a-turn-is-confined-by-a-frame`, `story:spawn-a-second-agent`, `story:a-recorded-run-shows-two-agents-working` — via `aep plan artifact show <id>` on each, cross-checked against `aep plan artifact waves --kind story --status draft` (68 collisions computed across the whole backlog, 0 unassessed) to confirm the collision detector's own view of this pair.

**Verification against round 1's three findings:**

1. **`metaharness_argv` collision** — both `story:a-turn-is-confined-by-a-frame` and `story:spawn-a-second-agent` now name each other, the function `metaharness_argv`, and the range `coordinator.rs:544-580` as cited (both bodies list `coordinator.rs` in typed scope, no `(inferred)` tag). Both state the pair "cannot be worked at the same time" and give the same resolution: sequence, frame story first. This is a named collision with a citation and a stated resolution — the round-1 defect (unnamed collision) is closed. `aep plan artifact waves` independently keeps them apart (frame in wave 1, spawn in wave 6) purely from the shared `coordinator.rs` scope entry, which does not depend on the prose at all.
2. **`spec-facts.json`** — confirmed absent from the typed `scope:` block of both `story:a-recorded-run-shows-two-agents-working` (scope is now only `AGENTS.md`, `README.md`, both cited) and `story:spawn-a-second-agent` (scope list has 8 entries, none is `spec-facts.json`). Both bodies state the derivation (`website/scripts/spec-facts.mjs` from `src/core/domains/**`) and that gate step 11 catches staleness.
3. **`trigger.rs` / `swarm.rs`** — confirmed removed from `story:a-turn-is-confined-by-a-frame`'s typed scope (now only `config.yaml`, `coordinator.rs`). The `## Collides with` section states the reason: neither file has frame/decisions/subject-scope code, and names the critic that caught it.

No new collision was introduced by these edits: the frame story's remaining scope (`config.yaml`, `coordinator.rs`) and the spawn story's remaining scope don't produce any pair I can find undisclosed; `a-recorded-run-shows-two-agents-working`'s scope (`AGENTS.md`, `README.md`) shares nothing with the other two, matching its own "would collide with nothing" claim.

**What I could not establish / out of my lane:** The stated resolution ("frame story first") lives only as prose in both bodies' `## Collides with` sections — neither artifact carries a `depends_on` edge encoding that direction. Mechanically this doesn't leave the pair unprotected — `aep plan artifact waves` already refuses to co-schedule them in one wave purely from the shared cited scope entry on `coordinator.rs`, independent of the prose — but the *direction* (frame before spawn, not the reverse) is asserted only in text and isn't machine-checked the way a `depends_on` edge would be. Whether that should be tightened into a graph edge is a sequencing/process call, which the rubric places outside parallel-safety's lane; I surface it for the operator rather than treat it as a finding.

**Numbers:** 3 items assessed; surfaces established **cited** for all 3 (frame: 2/2 scope entries cited; spawn: 5 cited + 3 inferred entries, with the collision-relevant one, `coordinator.rs`, cited; a-recorded-run: 2/2 cited); 0 inferred-only; 0 unplaceable.

**Coordinator's note, added when recording:** the direction this critic surfaced and declined to make a finding of *was* made a finding by `plan-critic-design-goal-round-2`, and it is closed — `story:spawn-a-second-agent depends_on story:a-turn-is-confined-by-a-frame` was added to the graph before this record was written.

```findings
[]
```

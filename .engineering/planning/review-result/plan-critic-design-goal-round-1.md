---
format: aep.planning-md/1
id: review-result:plan-critic-design-goal-round-1
kind: review-result
status: active
title: Design critic, goal decomposition, round 1
relations:
- reviews: story:a-turn-is-confined-by-a-frame
- reviews: story:a-recorded-run-shows-two-agents-working
- reviews: story:spawn-a-second-agent
revision: 1
---
Design critic, round 1, over the set drafted 2026-09-13 from the goal "an autonomous, sandboxed
multi-agent swarm, proven": `story:a-turn-is-confined-by-a-frame`,
`story:a-recorded-run-shows-two-agents-working`, `story:spawn-a-second-agent`.

Returned as the critic returned it.

---

**needs-revision**

story:a-recorded-run-shows-two-agents-working — its `kind: story` and `decomposes: epic:executable-boxes` (frontmatter lines 4, 8) present it as a build unit of the epic on the same footing as the other two, but its own body says it "builds nothing that those two do not" (line 99) and that its outcome is "recorded as a `verification-report`" (lines 73, 88) — a kind this store's own vocabulary already carries (`aep plan artifact kinds` lists `verification-report` as an output kind) alongside a `verifies` relation (`aep plan artifact relations`: "Establishes that something holds"), and neither is used; the artifact is shaped and related as if it were epic-work when its content is evidence about two other artifacts' acceptance — story/a-recorded-run-shows-two-agents-working.md:4,8,99

**What I read**: all 3 named artifacts in full (`aep plan artifact show` on each), plus `aep plan artifact relations`, `aep plan artifact graph` (walked all 51 printed edges, not just the 4 touching this set — the 3 items plus their 6 declared edges to `epic:executable-boxes`/`vision:swarm-builds-itself`/each other), `aep plan artifact validate` (reports clean, so nothing here duplicates it), `aep plan artifact kinds`, and `aep plan artifact lifecycle story|verification-report` to confirm the two kinds' lifecycles actually differ (story tracks draft→…→implemented; verification-report has none). I also read the raw frontmatter of story:a-recorded-run-shows-two-agents-working to cite exact lines.

**What I could not establish**: whether `story:a-turn-is-confined-by-a-frame` and `story:spawn-a-second-agent` sharing `coordinator.rs`/`swarm.rs`/`trigger.rs` in their scope sections is disclosed anywhere as a concurrency concern between the two — that question is parallel-safety's lane, not mine, and I did not chase it further. No cycle exists in the declared `depends_on`/`decomposes`/`serves` edges among these three or in the wider graph; A and B are independent of each other and of C, so the fan-in to C is a diamond, not a queue, and I found no chain-serialization or split-abstraction defect between A and B — each is independently demonstrable without the other.

```findings
- file: .engineering/planning/story/a-recorded-run-shows-two-agents-working.md
  line: 4
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its kind (story) and decomposes: epic:executable-boxes edge present it as a build unit of the epic, but its body says it \"builds nothing that those two do not\" (line 99) and that its outcome is \"recorded as a verification-report\" (lines 73, 88) — a kind and a verifies relation this store's own vocabulary already carries and neither is used, so an evidence artifact is shaped and related as if it were epic-work"
```

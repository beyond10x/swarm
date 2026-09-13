---
format: aep.planning-md/1
id: review-result:next-wave-20260913-acceptance-round-1
kind: review-result
status: active
title: Next wave acceptance review, round 1
relations:
- reviews: epic:trustworthy-swarm-control
- reviews: story:pause-and-stop-control-runtime-work
- reviews: story:verify-two-agent-evidence-in-rust
- reviews: story:show-command-issuers-in-the-canvas
revision: 1
---
needs-revision

epic:trustworthy-swarm-control — the acceptance joins child-story evidence, an integration gate result, and publication accuracy as three independently passing outcomes instead of one observable completion outcome — .engineering/planning/epic/trustworthy-swarm-control.md:31
story:verify-two-agent-evidence-in-rust — the acceptance requires unchanged output and exit behavior for every existing regression scenario without exempting the intentionally changed inaccessible-evidence behavior specified under Required evidence — .engineering/planning/story/verify-two-agent-evidence-in-rust.md:39

Read all 4 assigned artifacts with `aep plan artifact show`: epic:trustworthy-swarm-control and stories pause-and-stop-control-runtime-work, verify-two-agent-evidence-in-rust, and show-command-issuers-in-the-canvas; queried kinds and epic/story lifecycles, ran artifact validation, inspected acceptance line numbers and referenced runtime/checker/canvas symbols, and confirmed the original demonstration directory contains SQLite and turn inputs.

Could not establish historical input completeness or regression equivalence without executing the future implementation; those remain implementation evidence requirements. Validation passes with four existing prose-only review advisories, which are not findings here.

```findings
- file: .engineering/planning/epic/trustworthy-swarm-control.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance joins child-story evidence, an integration gate result, and publication accuracy as three independently passing outcomes instead of one observable completion outcome
- file: .engineering/planning/story/verify-two-agent-evidence-in-rust.md
  line: 39
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance requires unchanged output and exit behavior for every existing regression scenario without exempting the intentionally changed inaccessible-evidence behavior specified under Required evidence
```

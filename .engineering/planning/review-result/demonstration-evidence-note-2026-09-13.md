---
format: aep.planning-md/1
id: review-result:demonstration-evidence-note-2026-09-13
kind: review-result
status: archived
title: Created in error, retired
relations:
- reviews: story:a-recorded-run-shows-two-agents-working
revision: 2
---
Created by accident on 2026-09-13 by the wave coordinator: a shell guard of the form
`aep plan artifact new … --title "x" --from /dev/null >/dev/null 2>&1 || true` succeeded rather than
failing, and left an empty review-result in the store.

It reviews nothing and records nothing. It is kept only because the store refuses physical deletion —
an `rm` was attempted first and `validate` reported it as *"deleted: … nothing is physically deleted
through a command, so this was `rm`"*, which is the guardrail working. Retired with
`move --to archived`, which is the store's own remedy and keeps the record of the mistake.

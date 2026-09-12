---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-a-pass-2
kind: review-result
status: active
title: Adversary, bug wave unit A, pass 2
relations:
- reviews: story:request-key-is-not-idempotency
revision: 1
---
```
unit: story:request-key-is-not-idempotency (unit A, pass 2) — worktree wave-20260912b-a, HEAD 61a196b plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 25→27, red 2
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: no
```

Recorded by the coordinator from the adversary's returned report, unedited.

### 1. `git --no-pager diff --stat`

```
$ git --no-pager diff --stat        # (empty — the file is untracked, not modified)
$ git --no-pager status --short
?? src/runtime/swarm-server/tests/the_request_key_contract_under_attack.rs
```

One path, a test file. No implementation file touched.

### 2. Cases added

The file deliberately does not spell the story id — it assembles it in `story()` — because
`documents_about_the_request_key()` enumerates every `.rs` under `tests/` that does. An adversary
file must not join the population it measures.

**(a) `a_refuted_promise_wrapped_across_a_comment_line_is_still_a_refuted_promise`** — red. Applies
the unit's own rule to text normalised the way a reader reads it: comment markers stripped,
whitespace collapsed.

```
panicked at tests/the_request_key_contract_under_attack.rs:85:5:
a promise this story measured false is quoted with nothing saying it is history, and the check that
exists to catch exactly that missed it because the sentence is wrapped across a comment line.
Matching raw file text makes the guard a function of where the wrap fell
(story:request-key-is-not-idempotency). Normalise the text before matching. Found:

tests/the_request_key_contract.rs: ["not a second request"] in:
... `tests/redelivery_under_attack.rs` said in the present tense that "`http.rs` documents
`issue::request` as \"the idempotency key. retrying with the same one is a retry, not a second
request\"" — the sentence the same commit had removed from `http.rs`. ...

test result: FAILED. 0 passed; 1 failed; 1 filtered out
```

**(b) `the_log_answers_that_a_request_which_landed_is_absent`** — red. Issues under key `LOST`,
records 200 further events, reads the log at the TypeScript client's own default limit.

```
panicked at tests/the_request_key_contract_under_attack.rs:123:5:
the log offered at the client's own default limit does not carry the request the first attempt was
written under, so a client that follows the documented instruction — read the log, resend only if
the attempt is absent — resends a command that landed. `GET /swarms/{slug}/log` has only `limit`,
and `Store::history` keeps the tail, so no request the client can make reaches an older attempt.
... The log held 200 events, oldest request Some("later-0")

test result: FAILED. 0 passed; 1 failed; 1 filtered out
```

### 3. The suite

```
$ cargo fmt -p swarm-server -- --check          → exit 0
$ cargo clippy -p swarm-server --all-targets    → Finished `dev` profile, no warnings
$ cargo test -p swarm-server
  src/lib.rs                                     ok.      10 passed; 0 failed
  src/main.rs                                    ok.       0 passed; 0 failed
  tests/redelivery_under_attack.rs               ok.       2 passed; 0 failed
  tests/serves_a_swarm.rs                        ok.       6 passed; 0 failed
  tests/the_cascade_of_a_caused_command.rs       ok.       3 passed; 0 failed
  tests/the_request_key_contract.rs              ok.       4 passed; 0 failed
  tests/the_request_key_contract_under_attack.rs FAILED.   0 passed; 2 failed
exit status 101
```

25 is the same run with the adversary's two cases excluded; 27 with them.

### 4. Findings

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| 1 | `the_request_key_contract.rs:345` (`refuted_in`), consumed at `:269` | NEEDS-CHANGE | introduced | `refuted_in` matches each refuted sentence as one raw substring. Every doc comment wraps near column 100, so a sentence split by `\n///` is invisible. The tree contains exactly one such wrapped quotation, at `:242-243`, and its paragraph carries **no** `PAST` marker | `cargo test -p swarm-server` — the guard passes on the one example its own doc holds up as the defect. The same weakness defeats the sibling check: a drift-back in `http.rs` wrapped mid-sentence passes both |
| 2 | `src/http.rs:57` and `src/web/src/runtime.ts:282` | CONFIRMED | introduced | Both say the log "answers exactly whether the first attempt landed" and `runtime.ts` adds "resend only if it is absent". `LogPage` has only `limit`, capped at 1000; `Store::history` returns the tail. Past `limit` events the attempt is unreachable and reported absent | `runtime.ts:336` `log(slug, limit = 200)` is the documented default, and there is no offset, cursor or request-key filter on the surface. Frequency unmeasured; the absence of any API that could answer is not |
| 3 | `the_request_key_contract.rs:422` | INFEASIBLE | introduced | `collect` matches only `.rs` and `.ts`. `src/web/src` holds 35 `.vue` files against 14 `.ts` | nothing found — no `.vue` names the story or any refuted sentence. Constructed, not live |
| 4 | `the_request_key_contract.rs:345` / `:360` | INFEASIBLE | introduced | `refuted_in` returns offsets into the lowercased text; `paragraph_around` indexes the original. Any character whose lowercase differs in UTF-8 length shifts every later offset | nothing found — the four surfaces are ASCII plus em dashes. Latent |
| 5 | `the_request_key_contract.rs:269` | CONFIRMED | introduced | `PAST` is matched against the raw paragraph while `refuted_in` lowercases. A paragraph opening "Used to say … not a second request" is flagged as unmarked | any author who begins a sentence with a marker. False red on correct prose |
| 6 | `the_request_key_contract.rs:342` | CONFIRMED | introduced | `PAST` includes the bare word `wrote`, satisfied by any paragraph saying anyone wrote anything. It is already the sole marker keeping `redelivery_under_attack.rs:142-150` green | on the documented path — the guard's own doc tells authors to add a marker |
| 7 | `src/http.rs:59` | CONFIRMED | introduced | enumerates a canvas record as entity, id, state, fields, revision. The instance carries `identity_field` too, and the unit's own test doc lists it | a reader comparing against the wire. Note only; the load-bearing half is true and asserted |

Origin for 1, 2, 5, 6, 7 read at the base with `git show 0c24ac0:…`. `Store::history`'s tail
behaviour is pre-existing; the claim about it is this unit's.

### 5. Attacked and could not break

- The move of `every_document_of_the_request_key_states_the_same_contract` (3→2 in one file, 0→4 in the new one): a move and a strengthening — same name, same three surfaces, `REFUTED` grew 4→6. No case lost.
- `doc_block_above`'s exactly-one-line assertion: both declarations match one trimmed line each.
- `issue_command` mints a fresh uuid when the key is absent — true, `http.rs:169-171`.
- The log carries the `request` each event was written under — true at the wire level.
- A canvas record names no key at all — true.
- The measured 1-append-against-2 for `MoveBox` — reproduced.
- Every `.rs`/`.ts`/`.vue`/`.md`/`.py` outside `.engineering/` grepped for a refuted sentence not naming the story — none.

### 6. Paths written outside the worktree

- `~/.cache/swarm-wave-2026-09-12b/unit-a/scratch/adv2/probe.py` and its directory.

```findings
- file: src/runtime/swarm-server/tests/the_request_key_contract.rs
  line: 345
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: refuted_in matches each refuted sentence as one raw substring, so a sentence wrapped across a comment line is invisible to the guard, and the tree already contains exactly that at the_request_key_contract.rs:242 in a paragraph with no past-tense marker.
- file: src/runtime/swarm-server/src/http.rs
  line: 57
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: http.rs and runtime.ts:282 tell a disconnected client the log answers exactly whether its attempt landed and to resend if absent, but the log endpoint takes only a limit capped at 1000 and Store::history returns the tail, so an attempt older than the window is reported absent and the client resends a command that landed.
- file: src/runtime/swarm-server/tests/the_request_key_contract.rs
  line: 422
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the derivation matches only .rs and .ts, so the 35 .vue files under web/src and every markdown page are outside the set it claims to derive; no such file names the story today, so the gap is constructed rather than live.
- file: src/runtime/swarm-server/tests/the_request_key_contract.rs
  line: 360
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: refuted_in returns offsets into the lowercased text while paragraph_around indexes the original, so any character whose lowercase differs in UTF-8 length shifts the slice into the wrong paragraph or panics on a non-char boundary; the current surfaces are ASCII plus em dashes, so it is latent.
- file: src/runtime/swarm-server/tests/the_request_key_contract.rs
  line: 269
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: past-tense markers are matched case-sensitively against the raw paragraph while refuted sentences are matched case-insensitively, so a paragraph that opens "Used to say ..." is flagged as an unmarked stale citation and reddens the gate on correct prose.
- file: src/runtime/swarm-server/tests/the_request_key_contract.rs
  line: 342
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the PAST list includes the bare word "wrote", which any paragraph mentioning that somebody wrote anything satisfies without attributing the quotation, and it is already the only marker holding redelivery_under_attack.rs:142 green.
- file: src/runtime/swarm-server/src/http.rs
  line: 59
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the canvas record is enumerated as entity, id, state, fields, revision and omits identity_field, which the instance actually carries and which the unit's own test doc in the same commit lists.
```

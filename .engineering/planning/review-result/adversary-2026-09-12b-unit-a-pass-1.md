---
format: aep.planning-md/1
id: review-result:adversary-2026-09-12b-unit-a-pass-1
kind: review-result
status: active
title: Adversary, bug wave unit A, pass 1
relations:
- reviews: story:request-key-is-not-idempotency
revision: 1
---
```
unit: unit A, story:request-key-is-not-idempotency, worktree wave-20260912b-a at e314c1d (working tree, one untracked test file added)
verdict: NEEDS-CHANGE
cases: executed 22→25, red 3
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (listed in part 6)
needs-coordinator: none
```

Recorded by the coordinator from the adversary's returned report. One change was forced on the
findings block below and it is the only one: finding 6's message contains a colon and a space
inside `` `request: String,` ``, which the store's YAML reader rejects, so that one message is
single-quoted. No word is altered.

## 1. `git --no-pager diff --stat`

Empty — the only change is an untracked file:

```
?? src/runtime/swarm-server/tests/the_request_key_contract.rs
```

No tracked file modified. No implementation file touched.

## 2. Cases added — `tests/the_request_key_contract.rs`

| case | asserts | now |
|---|---|---|
| `omitting_the_request_key_is_the_same_guarantee_as_repeating_one` | `http.rs:57` / `runtime.ts:283` — omitting the field "is the same guarantee" | red |
| `the_canvas_carries_the_request_every_record_was_written_under` | `runtime.ts:282` — reading the canvas shows the `request` each record was written under | red |
| `a_quotation_of_http_rs_is_a_quotation_of_what_http_rs_says` | `redelivery_under_attack.rs:50` quotes http.rs for a sentence http.rs no longer contains | red |

Red output of the first two:

```
running 2 tests
test the_canvas_carries_the_request_every_record_was_written_under ... FAILED
test omitting_the_request_key_is_the_same_guarantee_as_repeating_one ... FAILED

---- the_canvas_carries_the_request_every_record_was_written_under stdout ----
panicked at tests/the_request_key_contract.rs:181:9:
runtime.ts:281 tells a caller to read the canvas because "every record carries the `request` it was written under", and this canvas record carries no such field: {"entity":"swarm.blackbox.Box","fields":{...},"id":"601b5f12-...","identity_field":"box_id","revision":1,"state":"Draft"} (story:request-key-is-not-idempotency)

---- omitting_the_request_key_is_the_same_guarantee_as_repeating_one stdout ----
assertion `left == right` failed: omitting the request key is documented as "the same guarantee" (http.rs:57, runtime.ts:283), and it is not: one key spent twice appended 1 BoxMoved, two minted keys appended 2.
  left: 2
 right: 1

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

Third case:

```
running 1 test
test a_quotation_of_http_rs_is_a_quotation_of_what_http_rs_says ... FAILED
panicked at tests/the_request_key_contract.rs:225:9:
redelivery_under_attack.rs:50 says `http.rs` DOCUMENTS "retrying with the same one is a retry, not a second request", and http.rs no longer contains that sentence anywhere. The citation is stale in the same commit that made it stale (story:request-key-is-not-idempotency)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out
```

## 3. The suite, after the cases exist

`cargo test -p swarm-server` — exit **101**:

```
test result: ok. 10 passed; 0 failed   (lib)
test result: ok.  0 passed; 0 failed   (bin main)
test result: ok.  3 passed; 0 failed   (redelivery_under_attack — includes the unit's new doc check)
test result: ok.  6 passed; 0 failed   (serves_a_swarm)
test result: ok.  3 passed; 0 failed   (the_cascade_of_a_caused_command)
test result: FAILED. 0 passed; 3 failed (the_request_key_contract)
```

`executed 22` is a second run with the adversary's file deselected, exit 0, 10+3+6+3 = 22.
`cargo fmt -p swarm-server -- --check` exit 0; `cargo clippy -p swarm-server --all-targets` clean.

## 4. Findings

| # | file:line | verdict / origin | what was measured | what reaches it |
|---|---|---|---|---|
| 1 | `src/runtime/swarm-server/src/http.rs:57` | NEEDS-CHANGE / introduced | "Omitting the field … is the same guarantee" is false. Same key twice on `MoveBox` → 1 `BoxMoved`; two minted keys → 2 | `Issue::request` is `#[serde(default)]`; `http.rs:162` mints a fresh uuid |
| 2 | `src/web/src/runtime.ts:283` | NEEDS-CHANGE / introduced | same sentence, TS wording | every TS caller of `runtime.issue`; no caller in `stores/swarms.ts` passes one |
| 3 | `src/web/src/runtime.ts:282` | NEEDS-CHANGE / introduced | "Read canvas or log — every record carries the `request`" — the canvas does not | the documented recovery procedure; `http.rs:55` states the pair correctly, so the two surfaces disagree |
| 4 | `tests/redelivery_under_attack.rs:50` | CONFIRMED / introduced | present-tense quotation of a sentence this diff removed from http.rs | both corrected surfaces point the reader at this file by name |
| 5 | `tests/redelivery_under_attack.rs:210` | INFEASIBLE / introduced | the check is a hardcoded 3-path list plus a 4-sentence blacklist | cannot be made red without editing a file the adversary may not edit |
| 6 | `tests/redelivery_under_attack.rs:257` | INFEASIBLE / introduced | `doc_block_above` binds by the first line trim-equal to a short declaration | latent; unique in all three files today |

## 5. Attacked and could not break

- Both characterisation cases still pass and still measure what they claim.
- `http.rs:55` "the log … carries each event's `request`" — true (`store.rs:319,336`).
- "`Swarm::issue` applies before it appends" — true (`swarm.rs:660` apply, `:673` commit).
- `store.rs:136` accurate, backed by `survives_a_restart.rs:195,224`.
- No fifth surface: `swarm-cli` sends no request key; no `.vue`, store or markdown surface documents it.
- No caller blind-resends, so the old false promise was not relied on in code.

## 6. Paths written outside the worktree

- `~/.cache/swarm-wave-2026-09-12b/unit-a/scratch/adv3-suite-{before,after,final}.log`

## 7.

```findings
- file: src/runtime/swarm-server/src/http.rs
  line: 57
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the new doc says omitting the request key "is the same guarantee", and it is not — http.rs:162 mints a fresh key, so the append guard cannot fire and a repeated MoveBox appends twice where one repeated key appends once.
- file: src/web/src/runtime.ts
  line: 283
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the TypeScript doc repeats the same false claim that omitting `request` costs nothing, on the surface whose callers never pass one.
- file: src/web/src/runtime.ts
  line: 282
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the doc sends a caller to the canvas because "every record carries the `request` it was written under", and canvas records are instances that carry no request field at all, so the documented recovery procedure reports every first attempt as absent.
- file: src/runtime/swarm-server/tests/redelivery_under_attack.rs
  line: 50
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a fourth reader-facing document — the one both corrected surfaces point at by name — still says in the present tense that http.rs documents "retrying with the same one is a retry, not a second request", which this same commit removed from http.rs.
- file: src/runtime/swarm-server/tests/redelivery_under_attack.rs
  line: 210
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the new check is a hardcoded three-path list and a four-sentence blacklist, so a fourth surface fails nothing and a reworded false promise containing "instance's stream" and the story id passes; finding 4 is that gap already realised on a live document, and making the check itself red would require editing a file I may not edit.
- file: src/runtime/swarm-server/tests/redelivery_under_attack.rs
  line: 257
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'doc_block_above binds each surface by the first line trim-equal to a short declaration such as `request: String,`, so a second such line added earlier in the file would silently point the check at the wrong doc block; unique in all three files today.'
```

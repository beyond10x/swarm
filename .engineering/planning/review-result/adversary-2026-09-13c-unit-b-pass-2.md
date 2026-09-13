---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13c-unit-b-pass-2
kind: review-result
status: active
title: Rust checker adversary pass 2
relations:
- reviews: story:verify-two-agent-evidence-in-rust
revision: 1
---
unit: Rust two-agent evidence checker, final pass 2, commit 8ac92f786b22c163a469a71ffc58fcd1f2852c18
verdict: NEEDS-CHANGE
cases: executed 118→123, red 3
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 28 retained scratch files; own lease registry and shared compiler cache
needs-coordinator: route residual quoting and newly measured legacy-reader acceptance findings; this is the final approved attack

```text
git --no-pager diff --stat
(no tracked modifications; exit 0)
git --no-pager diff --no-index --stat /dev/null src/runtime/swarm-check/tests/adversary_value_boundaries.rs
 .../tests/adversary_value_boundaries.rs            | 110 +++++++++++++++++++++
 1 file changed, 110 insertions(+)
(exit 1: the new test differs from /dev/null)
```
Only the added test file is dirty. No implementation, existing test, AEP artifact, branch, commit, or integration file was changed by this pass.

## 2. Cases added and run alone

The first case was written before any test run in this pass. Each of the other cases was also written before its isolated execution. No compilation failed. Existing pass-1 tests were not run before the first new case. rustfmt later changed line numbers in the new file only; current assertion lines below correspond to the final suite output.

Commands used RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2. The isolated runs also set SWARM_ADVERSARY_RETAIN=/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2 to keep synthetic inputs for the saved legacy CLI. Cargo used the assigned checkout target directory.

### nested_combining_unicode_keeps_python_printable_spelling

src/runtime/swarm-check/tests/adversary_value_boundaries.rs:50; red now. Both CLIs exit 0; Python preserves the printable combining accent in the nested string, while Rust writes literal backslash-u0301 inside found.

`cargo test -p swarm-check --test adversary_value_boundaries nested_combining_unicode_keeps_python_printable_spelling -- --exact --nocapture` — exit 101

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 1 test

thread 'nested_combining_unicode_keeps_python_printable_spelling' (2002856) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:39:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['Cafe\\u0301']")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['Cafe\u{301}']"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test nested_combining_unicode_keeps_python_printable_spelling ... FAILED

failures:

failures:
    nested_combining_unicode_keeps_python_printable_spelling

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

error: test failed, to rerun pass `-p swarm-check --test adversary_value_boundaries`
```

### nan_reason_keeps_legacy_decision_record

src/runtime/swarm-check/tests/adversary_value_boundaries.rs:60; red now. Legacy CLI exit 0; Rust CLI exit 1 because the tool.decided row carrying a NaN reason is skipped, making clause 5 NOT MET.

`cargo test -p swarm-check --test adversary_value_boundaries nan_reason_keeps_legacy_decision_record -- --exact --nocapture` — exit 101

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.23s
     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 1 test

thread 'nan_reason_keeps_legacy_decision_record' (2005845) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:47:5:
assertion `left == right` failed: the legacy reader retains this tool.decided: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"2 tool call(s) decided in 2 turn record(s), none denied; the session censuses claim 1 denied call(s) and 0 `tool.decided` record(s) name a denier","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":false,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason.aKZd5DpA0gaY/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test nan_reason_keeps_legacy_decision_record ... FAILED

failures:

failures:
    nan_reason_keeps_legacy_decision_record

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.13s

error: test failed, to rerun pass `-p swarm-check --test adversary_value_boundaries`
```

### escaped_lone_surrogate_reason_keeps_legacy_decision_record

src/runtime/swarm-check/tests/adversary_value_boundaries.rs:74; red now. Legacy CLI exit 0; Rust CLI exit 1 because the tool.decided row carrying a nested escaped lone surrogate is skipped, making clause 5 NOT MET.

`cargo test -p swarm-check --test adversary_value_boundaries escaped_lone_surrogate_reason_keeps_legacy_decision_record -- --exact --nocapture` — exit 101

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 1 test

thread 'escaped_lone_surrogate_reason_keeps_legacy_decision_record' (2005919) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:56:5:
assertion `left == right` failed: the legacy reader retains this tool.decided: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"2 tool call(s) decided in 2 turn record(s), none denied; the session censuses claim 1 denied call(s) and 0 `tool.decided` record(s) name a denier","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":false,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason.42Nbw8EGPYY3/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test escaped_lone_surrogate_reason_keeps_legacy_decision_record ... FAILED

failures:

failures:
    escaped_lone_surrogate_reason_keeps_legacy_decision_record

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.13s

error: test failed, to rerun pass `-p swarm-check --test adversary_value_boundaries`
```

### duplicate_members_keep_first_position_and_last_value

src/runtime/swarm-check/tests/adversary_value_boundaries.rs:89; green now. Both CLIs exit 0 and preserve the first position of a duplicated key while using its final value.

`cargo test -p swarm-check --test adversary_value_boundaries duplicate_members_keep_first_position_and_last_value -- --exact --nocapture` — exit 0

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 1 test
test duplicate_members_keep_first_position_and_last_value ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.14s

```

### runtime_json_order_and_wide_number_encoding_stay_local

src/runtime/swarm-check/tests/adversary_value_boundaries.rs:100; green now. Ordinary serde_json keeps sorted map output and converts an integer wider than u64 to f64; the evidence value alone preserves source map order and the exact integer.

`cargo test -p swarm-check --test adversary_value_boundaries runtime_json_order_and_wide_number_encoding_stay_local -- --exact --nocapture` — exit 0

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 1 test
test runtime_json_order_and_wide_number_encoding_stay_local ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

```

Novel differential baselines were obtained only with the pre-existing executable /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py, byte-equal to git show c935d85:examples/two-agents/check-two-agents.py. No new Python was written or invoked by the Rust tests. Each saved-reader command was `python3 <saved-reader> <retained-fixture> --json`, exit 0, with stdout captured in the correspondingly named *-legacy.json file below. The runtime-isolation test has no legacy executable call.

## 3. Suite, after all five additions existed

Before 118 comes from correction-1-report.md. `cargo test -p swarm-check --no-fail-fast` with the environment above exited 101. Actual package execution: 118 old tests green, 2 added tests green, 3 added tests red = 123 executed. Full output:

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.16s
     Running unittests src/lib.rs (target/debug/deps/swarm_check-a0ee706b9b6fbb38)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/swarm_check-db99701498604c23)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-78f9233f1b20fbab)

running 5 tests
test boolean_iteration_missing_transcript_retains_legacy_failing_verdict ... ok
test unsigned_iteration_missing_transcript_retains_legacy_failing_verdict ... ok
test false_denial_reason_retains_legacy_empty_suffix ... ok
test structured_denial_reason_retains_legacy_rendering ... ok
test integral_float_iterations_retain_legacy_spend_attribution ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/adversary_value_boundaries.rs (target/debug/deps/adversary_value_boundaries-b5f1141d5f79210b)

running 5 tests
test runtime_json_order_and_wide_number_encoding_stay_local ... ok
test duplicate_members_keep_first_position_and_last_value ... ok
test nan_reason_keeps_legacy_decision_record ... FAILED
test escaped_lone_surrogate_reason_keeps_legacy_decision_record ... FAILED
test nested_combining_unicode_keeps_python_printable_spelling ... FAILED

failures:

---- nan_reason_keeps_legacy_decision_record stdout ----

thread 'nan_reason_keeps_legacy_decision_record' (2007632) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:60:5:
assertion `left == right` failed: the legacy reader retains this tool.decided: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"2 tool call(s) decided in 2 turn record(s), none denied; the session censuses claim 1 denied call(s) and 0 `tool.decided` record(s) name a denier","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":false,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason.TTBYKLpG6cb9/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- escaped_lone_surrogate_reason_keeps_legacy_decision_record stdout ----

thread 'escaped_lone_surrogate_reason_keeps_legacy_decision_record' (2007631) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:74:5:
assertion `left == right` failed: the legacy reader retains this tool.decided: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"2 tool call(s) decided in 2 turn record(s), none denied; the session censuses claim 1 denied call(s) and 0 `tool.decided` record(s) name a denier","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":false,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason.2C05NuyQDsY2/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0

---- nested_combining_unicode_keeps_python_printable_spelling stdout ----

thread 'nested_combining_unicode_keeps_python_printable_spelling' (2007633) panicked at src/runtime/swarm-check/tests/adversary_value_boundaries.rs:50:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['Cafe\\u0301']")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — ['Cafe\u{301}']"


failures:
    escaped_lone_surrogate_reason_keeps_legacy_decision_record
    nan_reason_keeps_legacy_decision_record
    nested_combining_unicode_keeps_python_printable_spelling

test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

error: test failed, to rerun pass `-p swarm-check --test adversary_value_boundaries`
     Running tests/compatibility.rs (target/debug/deps/compatibility-52983d293f767703)

running 15 tests
test absent_and_corrupt_sqlite_are_not_created_or_modified ... ok
test missing_database ... ok
test corrupt_database ... ok
test slug_with_data_root_has_the_same_json_and_exit_status ... ok
test unreadable_capped_record_fails_closed ... ok
test missing_assignment_identity ... ok
test missing_turns ... ok
test malformed_event_payloads ... ok
test unreadable_transcript_fails_closed ... ok
test malformed_json_lines ... ok
test bare_denial ... ok
test legacy_turn_names ... ok
test unknown_issuer_mark ... ok
test denial_words ... ok
test bound_object ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/metadata.rs (target/debug/deps/metadata-d76e07c77611e169)

running 9 tests
test test_adversary_pass2_thefixtureagainstthedomain_test_the_fixture_spawns_a_role_the_domain_has_no_variant_for ... ok
test test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_the_guard_is_green_against_the_four ... ok
test test_checker_theissuervocabulary_test_the_checker_names_every_value_the_runtime_can_write ... ok
test test_checker_theissuervocabulary_test_a_variant_with_no_label_arm_at_all_is_caught_by_name ... ok
test test_adversary_issuer_pass_1_theissuervocabularyguard_test_a_fifth_variant_whose_label_arm_is_a_block_is_caught ... ok
test test_checker_theissuervocabulary_test_a_variant_whose_label_is_computed_is_a_missing_label_and_not_a_silence ... ok
test test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_a_struct_variant_is_a_missing_label_and_not_a_silence ... ok
test test_adversary_issuer_pass_1_theissuervocabularyguard_test_the_guard_is_green_against_the_store_it_was_written_for ... ok
test test_checker_theissuervocabulary_test_the_fixture_speaks_the_same_vocabulary ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/regression.rs (target/debug/deps/regression-d58f871c7f88aa08)

running 74 tests
test test_adversary_issuer_pass_1_theunattendedconditiononanemptywindow_test_a_window_holding_only_the_excepted_opener_is_not_unattended ... ok
test test_adversary_clausethreeagainstitsownsentence_test_not_met_when_the_two_transcripts_name_agents_the_log_never_spawned ... ok
test test_adversary_clausesevenagainstclausesix_test_not_met_when_the_second_agent_to_bound_is_only_a_name_on_a_spend_row ... ok
test test_adversary_issuer_pass_2_thewindowthatrunspasttheverdict_test_an_operator_after_the_verdict_is_not_a_hand_on_the_run ... ok
test test_checker_clausefive_test_not_met_when_there_are_no_turn_records_at_all ... ok
test test_checker_clausefour_test_not_met_when_nobody_took_the_work_there_is_no_finisher_to_check ... ok
test test_adversary_pass2_clausetwoandthehandoveritnames_test_clause_two_names_a_posting_that_was_never_answered ... ok
test test_adversary_pass2_thereadmeagainsttheruntime_test_the_readme_names_every_clause_no_run_can_meet ... ok
test test_adversary_issuer_pass_2_amemberwhoseslugbeginswithaquestionmark_test_a_member_that_took_its_own_work_is_not_read_as_a_digest ... ok
test test_checker_clausefour_test_met_by_a_finished_assignment ... ok
test test_adversary_issuer_pass_1_theexportedevidence_test_the_evidence_quotes_an_unattended_line_this_checker_still_prints ... ok
test test_checker_clausefive_test_not_met_when_the_denial_did_not_come_from_the_frame ... ok
test test_checker_clausefour_test_met_by_a_green_gate ... ok
test test_adversary_clausethreeagainstitsownsentence_test_not_met_when_an_attempt_left_no_transcript_at_any_turn_number ... ok
test test_checker_clausefive_test_not_met_when_every_call_was_allowed ... ok
test test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_of_an_assignment_that_was_never_taken ... ok
test test_adversary_clausefiveagainsttherealcensus_test_not_met_when_the_frame_only_allowed_and_another_decider_denied ... ok
test test_checker_clausefive_test_met_when_the_frame_refused_a_tool_call ... ok
test test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_recorded_before_the_work_was_posted ... ok
test test_checker_clausefour_test_not_met_when_the_coordinator_reports_a_green_gate_on_the_builder_s_work ... ok
test test_checker_clausefour_test_not_met_when_the_work_was_only_started ... ok
test test_checker_clauseone_test_met_when_an_agent_was_spawned_with_a_role_that_is_not_coordinator ... ok
test test_checker_clauseone_test_not_met_when_nothing_was_spawned ... ok
test test_checker_clauseone_test_not_met_when_every_spawned_agent_is_a_coordinator ... ok
test test_checker_clauseseven_test_not_met_when_there_was_only_one_agent_to_bound ... ok
test test_adversary_theunattendedconditionagainstareallog_test_an_operator_pausing_and_resuming_by_hand_is_not_unattended ... ok
test test_checker_clauseseven_test_met_when_the_refusal_is_recorded_on_the_spend_row ... ok
test test_checker_clauseseven_test_met_when_the_refusal_is_recorded_beside_the_transcripts ... ok
test test_checker_clauseseven_test_met_when_the_agent_ceiling_refused_an_agent ... ok
test test_checker_clauseseven_test_not_met_when_nothing_was_refused ... ok
test test_checker_clausesix_test_not_met_when_there_is_no_spend_record ... ok
test test_checker_clausesix_test_not_met_when_the_second_named_agent_was_never_spawned ... ok
test test_checker_clausesix_test_not_met_when_only_one_agent_is_named ... ok
test test_checker_clausesix_test_not_met_when_the_rows_name_nobody ... ok
test test_checker_clausesix_test_met_when_two_agents_each_name_themselves ... ok
test test_checker_clausethree_test_not_met_when_both_transcripts_are_the_same_agent ... ok
test test_checker_clausethree_test_met_when_two_turn_files_are_attributed_to_two_agents ... ok
test test_checker_thecommandline_test_an_absent_swarm_exits_one_rather_than_raising ... ok
test test_checker_clausethree_test_not_met_when_an_attempt_left_no_file_beside_it ... ok
test test_checker_clausethree_test_not_met_when_no_turn_file_can_be_attributed ... ok
test test_checker_clausethree_test_not_met_with_one_transcript ... ok
test test_checker_clausetwo_test_it_says_it_is_coping_with_a_log_written_before_issuers ... ok
test test_checker_clausetwo_test_met_when_the_agent_it_was_posted_to_took_it ... ok
test test_checker_theshapeofthereport_test_every_clause_is_reported_even_when_there_is_no_event_log ... ok
test test_checker_theshapeofthereport_test_every_clause_is_reported_when_the_swarm_directory_is_absent ... ok
test test_checker_clausetwo_test_met_when_the_member_itself_issued_the_taking ... ok
test test_checker_clausetwo_test_met_when_the_runtime_issued_the_taking ... ok
test test_checker_clausetwo_test_not_met_when_nothing_was_posted ... ok
test test_checker_clausetwo_test_not_met_when_another_agent_issued_the_taking_on_its_behalf ... ok
test test_checker_clausetwo_test_not_met_when_another_agent_took_it ... ok
test test_checker_clausetwo_test_not_met_when_an_operator_issued_the_taking ... ok
test test_checker_clausetwo_test_not_met_when_an_earlier_handover_was_the_forged_one ... ok
test test_checker_theshapeofthereport_test_nothing_passes_by_finding_nothing ... ok
test test_checker_clausetwo_test_not_met_when_the_assignment_was_posted_and_never_taken ... ok
test test_checker_clausetwo_test_not_met_when_the_taking_names_an_agent_the_log_never_spawned ... ok
test test_checker_thecommandline_test_a_swarm_that_misses_one_clause_exits_one_and_names_the_number ... ok
test test_checker_clausetwo_test_not_met_when_the_taking_came_before_the_posting ... ok
test test_checker_thecommandline_test_a_swarm_that_meets_everything_exits_zero ... ok
test test_checker_thecommandline_test_every_clause_number_is_in_the_output ... ok
test test_checker_theshapeofthereport_test_a_not_met_clause_says_what_it_looked_for_and_what_it_found ... ok
test test_checker_thecommandline_test_json_output_carries_every_clause ... ok
test test_checker_theshapeofthereport_test_no_clause_is_met_by_a_name_the_log_never_spawned ... ok
test test_checker_theshapeofthereport_test_a_swarm_that_meets_everything_meets_all_seven ... ok
test test_checker_theunattendedcondition_test_a_swarm_that_never_started_is_not_reported_as_unattended ... ok
test test_checker_theshapeofthereport_test_no_clause_is_met_by_a_tally_instead_of_a_record ... ok
test test_checker_theunattendedcondition_test_it_still_does_not_decide_the_exit_status ... ok
test test_checker_theunattendedcondition_test_met_when_every_command_in_the_window_came_from_the_loop ... ok
test test_checker_theunattendedcondition_test_not_met_when_an_operator_reached_into_the_window ... ok
test test_checker_theunattendedcondition_test_not_met_when_an_issuer_names_an_agent_the_log_never_spawned ... ok
test test_checker_theunattendedcondition_test_the_actor_column_no_longer_decides_it ... ok
test test_checker_theunattendedcondition_test_not_met_when_the_log_predates_issuers_and_it_says_so ... ok
test test_checker_theunattendedcondition_test_the_command_that_starts_a_swarm_does_not_count_against_it ... ok
test test_checker_clausethree_test_met_by_each_way_a_turn_file_names_its_agent ... ok
test test_checker_theshapeofthereport_test_each_clause_can_be_taken_the_other_way_on_its_own ... ok

test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s

     Running tests/value_semantics.rs (target/debug/deps/value_semantics-34eb80a50dff8fd6)

running 15 tests
test ordered_evidence_does_not_change_runtime_json_map_serialization ... ok
test event_json_preserves_source_member_order ... ok
test tool_name_representation ... ok
test spawned_role_representation ... ok
test decider_representation ... ok
test posted_agent_representation ... ok
test taken_agent_representation ... ok
test equivalent_integer_keys_keep_first_spelling ... ok
test counting_is_distinct_from_numeric_equality ... ok
test operation_iteration_and_representation ... ok
test denial_reason_truthiness ... ok
test numeric_attribution_equality ... ok
test integer_attempt_claims ... ok
test census_integer_membership_and_unbounded_sum ... ok
test recursive_reason_representation ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.42s

   Doc-tests swarm_check

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p swarm-check --test adversary_value_boundaries`
```

## 4. Findings

Both findings cover 8ac92f786b22c163a469a71ffc58fcd1f2852c18 (correction 0292605 plus coordinator-reviewed lock 8ac92f7), against implementation base c935d85. They concern the explicitly retained readable/malformed-input parity promise, not the inaccessible-evidence exception.

| Location | Verdict | Origin | Measured assertion / exit | What reaches it |
|---|---|---|---|---|
| src/runtime/swarm-check/src/evidence.rs:270 | NEEDS-CHANGE | introduced | New test :50 fails: Python found contains the actual combining accent in Café; Rust found contains a literal escape sequence. Both CLI exits 0; isolated and suite test exits 101. | Documented directory CLI → main.rs → check → read_jsonl → clause five optional reason → records::py → Value::python/repr → python_quote. value-semantics.md:16 explicitly promises recursive Python representation; the regression corpus already treats container reasons as supported compatibility inputs. No runtime emitter of this container reason was established; this is a CLI report-contract finding. |
| src/runtime/swarm-check/src/evidence.rs:26 (also :61) | NEEDS-CHANGE | introduced | New tests :60 and :74 fail: NaN and escaped lone-surrogate reason rows yield Python clause 5 met / exit 0, but Rust drops the entire decision row, clause 5 NOT MET / exit 1. Isolated and suite exits 101. | The same documented CLI loads readable supplied transcript files through records::read_jsonl; serde parsing errors are silently filtered out. Python json.loads accepted these token/string forms. No actual runtime emitter was found for these unusual values. The acceptance explicitly keeps malformed-input differential behavior, so this is not a claim that production JSON must emit them. |

The first is residue of pass 1’s recursive-rendering class at a new helper location. The second is newly measured reader-acceptance ground: moving to raw_value preserves large numeric lexemes and map order but still relies on serde’s narrower lexical/string acceptance. Against the story base, the saved Python reader gives the asserted outputs, so both origins are introduced by the Rust port. Neither is a pre-existing Python failure.

Pass-1 findings status: all five unchanged cases now pass (SHA256 7448560301300678fa6cfd4bb43846adbaa16b8b9d53ab04f3bafe4a1708033b). Its numeric membership/equality and wide-counter examples remained green, including correction siblings. Its false/structured-reason examples pass, but the representation class still has the combining-character residue above. No third attack is requested or implied.

## 5. Attacks that did not break the unit

- Original unit/diff and caller review from pass 1 was retained; all correction source, manifest/lock changes, local evidence value, value-semantics contract, test helpers and 14 fixture-group shapes were inspected in this pass. The unchanged original 83 mapping and regression claims still ran green.
- The correction’s 81 captured reports in 14 groups execute through full parsed-report comparisons: integer/boolean membership, integral-float attribution, distinct counter rules, first-spelling coalescing, large signed/unsigned census totals, truthiness, nested repr at every py call site, operation iteration and source-order ceiling lookup all pass.
- A duplicate-member case independently confirms first-position/last-value object behavior.
- A new runtime JSON guard checks both ordering and wide-number encoding against the evidence-only behavior. It passes. `cargo tree -e features -i serde_json` exits 0 and lists default, raw_value and std only; neither preserve_order nor arbitrary_precision is enabled.
- Missing/corrupt SQLite and unreadable transcript/capped-file tests pass; all seven-versus-unattended, assignment/issuer, historical name and vocabulary cases remain green.
- Coordinator-owned integration README.md, AGENTS.md, store.rs, website/scripts/spec-facts.mjs and website/src/pages/index.js reference swarm-check as intended. They were read in the integration checkout and not changed. The assigned checker branch still has old coordinator-owned doc references pending integration; that is ownership sequencing, not a new finding.
- No original demonstration or historical export was modified. No AEP, remote integration, paid model or daemon was invoked.
- Worktree inspection showed 24 GiB free before build, above the 10 GiB floor.

Feature output:

```text
serde_json v1.0.151
├── serde_json feature "default"
│   ├── axum v0.8.9
│   │   ├── axum feature "default"
│   │   │   └── swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-server)
│   │   │       └── swarm-server feature "default" (command-line)
│   │   ├── axum feature "form"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "http1"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "json"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "matched-path"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "original-uri"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "query"
│   │   │   └── axum feature "default" (*)
│   │   ├── axum feature "tokio"
│   │   │   ├── axum feature "default" (*)
│   │   │   └── axum feature "tokio" (*)
│   │   ├── axum feature "tower-log"
│   │   │   └── axum feature "default" (*)
│   │   └── axum feature "tracing"
│   │       └── axum feature "default" (*)
│   ├── ess-compiler v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067)
│   │   └── ess-compiler feature "default"
│   │       └── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime)
│   │           └── ess-runtime feature "default" (command-line)
│   │               └── swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-server) (*)
│   ├── ess-domain v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067)
│   │   └── ess-domain feature "default"
│   │       ├── ess-compiler v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067) (*)
│   │       ├── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime) (*)
│   │       └── swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-server) (*)
│   ├── ess-primitives v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067)
│   │   └── ess-primitives feature "default"
│   │       ├── ess-compiler v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067) (*)
│   │       ├── ess-domain v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067) (*)
│   │       └── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime) (*)
│   ├── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime) (*)
│   ├── eventlog-core v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
│   │   └── eventlog-core feature "default"
│   │       ├── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime) (*)
│   │       └── eventlog-sqlite v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
│   │           └── eventlog-sqlite feature "default"
│   │               └── ess-runtime v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/ess-runtime) (*)
│   ├── eventlog-sqlite v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080) (*)
│   ├── schemars v0.8.22
│   │   ├── schemars feature "default"
│   │   │   ├── ess-domain v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067) (*)
│   │   │   └── ess-primitives v0.23.0 (https://github.com/beyond10x/ess?tag=0.23.0#c8023067) (*)
│   │   ├── schemars feature "derive"
│   │   │   └── schemars feature "default" (*)
│   │   └── schemars feature "schemars_derive"
│   │       └── schemars feature "derive" (*)
│   ├── swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
│   │   └── swarm-check feature "default" (command-line)
│   ├── swarm-cli v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-cli)
│   │   └── swarm-cli feature "default" (command-line)
│   └── swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-server) (*)
├── serde_json feature "raw_value"
│   ├── axum v0.8.9 (*)
│   └── swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check) (*)
└── serde_json feature "std"
    └── ureq v3.4.1
        └── ureq feature "json"
            └── swarm-cli v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-cli) (*)
    └── serde_json feature "default" (*)
```

## 6. Outside-worktree paths

Retained files under assigned scratch, excluding the pre-existing brief:

- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-reason/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-reason/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-reason/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/combining-reason/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-members/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-members/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-members/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/duplicate-members/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/nan-reason/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/report.md
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/runtime-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/serde-features.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/suite.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2/surrogate-reason/turns/spend.jsonl

Per-test TempDir paths were created below /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-2 and dropped automatically by the fixture. No scratch directory or build output was manually removed. Cargo build output stayed under /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/target. The mandated tools additionally maintain /home/timo/.cache/sccache (existing shared cache, 10 GiB limit) and /home/timo/.local/state/worktree (only this session’s lease); their internal files are tool-owned rather than additional custom scratch. No alternative target or /tmp path was used.

Handoff: tree id swarm-wave-20260913c-checker, path /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker, branch wave/2026-09-13c/checker. The coordinator owns correction routing, integration and final worktree lifecycle. Own session lease wave-20260913c-checker-adversary-2 was released with worktree hook session-end, exit 0.

```findings
- file: src/runtime/swarm-check/src/evidence.rs
  line: 270
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Recursive evidence representation escapes printable combining characters that Python preserves, changing parsed report reason strings.
- file: src/runtime/swarm-check/src/evidence.rs
  line: 26
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Legacy-readable NaN and escaped lone-surrogate reason values are rejected and their decision rows dropped, changing clause-five and CLI verdicts.
```

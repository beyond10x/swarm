---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13c-unit-b-pass-1
kind: review-result
status: active
title: Rust checker adversary pass 1
relations:
- reviews: story:verify-two-agent-evidence-in-rust
revision: 1
---
unit: Rust two-agent evidence checker, pass 1, commit d0eac57ed4f8cf0fdacbe622c61b06ba0891b58e
verdict: NEEDS-CHANGE
cases: executed 98→103, red 5
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 32 retained files in assigned scratch; worktree lease registry and shared compiler cache
needs-coordinator: route the two compatibility findings for correction

```text
git --no-pager diff --stat
(no tracked modifications; exit 0)
git --no-pager diff --no-index --stat /dev/null src/runtime/swarm-check/tests/adversary_port_parity.rs
 .../swarm-check/tests/adversary_port_parity.rs     | 124 +++++++++++++++++++++
 1 file changed, 124 insertions(+)
(exit 1 means the displayed untracked test differs from /dev/null)
```
Only the new test file is dirty. No implementation, original test, planning artifact, branch, commit, or historical export was edited. The coordinator owns the next action and tree lifecycle.

## 2. Added cases, each run alone first

All five cases are red now. The first case was written before any test or suite was executed. Each subsequent case was also written before its isolated execution. The first three assertion line numbers in the earliest logs precede rustfmt; the current locations below are after formatting. No compilation failed. The first run emitted one unused-import warning, removed from the new file before the other cases.

All Rust test commands used RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1. Isolated runs additionally set SWARM_ADVERSARY_RETAIN=/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1 to retain synthetic inputs. The target directory stayed inside the assigned checkout.

### boolean_iteration_missing_transcript_retains_legacy_failing_verdict

src/runtime/swarm-check/tests/adversary_port_parity.rs:43. A boolean iteration on an unrecorded goal is a claimed attempt to the legacy reader; Rust ignores it. Saved legacy CLI exit 1; Rust CLI exit 0. The isolated Rust test exits 101.

Command: `cargo test -p swarm-check --test adversary_port_parity boolean_iteration_missing_transcript_retains_legacy_failing_verdict -- --exact --nocapture`

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
warning: unused import: `fs`
 --> src/runtime/swarm-check/tests/adversary_port_parity.rs:4:11
  |
4 | use std::{fs, process::Command};
  |           ^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `swarm-check` (test "adversary_port_parity") generated 1 warning (run `cargo fix --test "adversary_port_parity" -p swarm-check` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 1 test

thread 'boolean_iteration_missing_transcript_retains_legacy_failing_verdict' (1948479) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:34:5:
assertion `left == right` failed: legacy reader counts this claimed attempt: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"3 row(s), 2 agent(s) named: `builder` ×2, `coordinator` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":7,"not_met":0,"ok":true,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration.QSoTmf1cMx4r/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test boolean_iteration_missing_transcript_retains_legacy_failing_verdict ... FAILED

failures:

failures:
    boolean_iteration_missing_transcript_retains_legacy_failing_verdict

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
```

Legacy comparison used only the existing saved executable: `python3 /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py <retained-input> --json`. Its stdout is /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-legacy.json, exit 1. No Python code was added, and the Rust tests invoke no Python.

### integral_float_iterations_retain_legacy_spend_attribution

src/runtime/swarm-check/tests/adversary_port_parity.rs:60. Integral float iteration values join transcripts in the legacy reader; Rust does not attribute either transcript. Saved legacy CLI exit 0; Rust CLI exit 1. The isolated Rust test exits 101.

Command: `cargo test -p swarm-check --test adversary_port_parity integral_float_iterations_retain_legacy_spend_attribution -- --exact --nocapture`

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 1 test

thread 'integral_float_iterations_retain_legacy_spend_attribution' (1950408) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:46:5:
assertion `left == right` failed: legacy reader attributes integral float iterations: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 0 attributed: none; 0 distinct agent(s): none; 2 file(s) no agent is named on: 0001-01-73a4ac74.jsonl, 0002-01-73a4ac74.jsonl","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":false,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations.K01Vx2SxCd0c/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test integral_float_iterations_retain_legacy_spend_attribution ... FAILED

failures:

failures:
    integral_float_iterations_retain_legacy_spend_attribution

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.14s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
```

Legacy comparison used only the existing saved executable: `python3 /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py <retained-input> --json`. Its stdout is /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-legacy.json, exit 0. No Python code was added, and the Rust tests invoke no Python.

### unsigned_iteration_missing_transcript_retains_legacy_failing_verdict

src/runtime/swarm-check/tests/adversary_port_parity.rs:81. An unsigned iteration above i64::MAX is a claimed attempt to the legacy reader; Rust ignores it. Saved legacy CLI exit 1; Rust CLI exit 0. The isolated Rust test exits 101.

Command: `cargo test -p swarm-check --test adversary_port_parity unsigned_iteration_missing_transcript_retains_legacy_failing_verdict -- --exact --nocapture`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 1 test

thread 'unsigned_iteration_missing_transcript_retains_legacy_failing_verdict' (1950486) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:58:5:
assertion `left == right` failed: legacy reader counts this claimed attempt: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"3 row(s), 2 agent(s) named: `builder` ×2, `coordinator` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":7,"not_met":0,"ok":true,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration.4ucyGBVJeyYt/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test unsigned_iteration_missing_transcript_retains_legacy_failing_verdict ... FAILED

failures:

failures:
    unsigned_iteration_missing_transcript_retains_legacy_failing_verdict

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.14s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
```

Legacy comparison used only the existing saved executable: `python3 /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py <retained-input> --json`. Its stdout is /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-legacy.json, exit 1. No Python code was added, and the Rust tests invoke no Python.

### false_denial_reason_retains_legacy_empty_suffix

src/runtime/swarm-check/tests/adversary_port_parity.rs:101. A false denial reason has no suffix in the legacy report; Rust appends an em dash and False. Saved legacy CLI exit 0; Rust CLI exit 0. The isolated Rust test exits 101.

Command: `cargo test -p swarm-check --test adversary_port_parity false_denial_reason_retains_legacy_empty_suffix -- --exact --nocapture`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 1 test

thread 'false_denial_reason_retains_legacy_empty_suffix' (1950560) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:70:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — False")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test false_denial_reason_retains_legacy_empty_suffix ... FAILED

failures:

failures:
    false_denial_reason_retains_legacy_empty_suffix

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.14s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
```

Legacy comparison used only the existing saved executable: `python3 /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py <retained-input> --json`. Its stdout is /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason-legacy.json, exit 0. No Python code was added, and the Rust tests invoke no Python.

### structured_denial_reason_retains_legacy_rendering

src/runtime/swarm-check/tests/adversary_port_parity.rs:120. A structured denial reason retains Python repr text inside the JSON found string; Rust instead emits compact sorted JSON. Saved legacy CLI exit 0; Rust CLI exit 0. The isolated Rust test exits 101.

Command: `cargo test -p swarm-check --test adversary_port_parity structured_denial_reason_retains_legacy_rendering -- --exact --nocapture`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 1 test

thread 'structured_denial_reason_retains_legacy_rendering' (1950634) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:82:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {\"limits\":[1,2],\"why\":true}")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {'why': True, 'limits': [1, 2]}"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test structured_denial_reason_retains_legacy_rendering ... FAILED

failures:

failures:
    structured_denial_reason_retains_legacy_rendering

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.13s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
```

Legacy comparison used only the existing saved executable: `python3 /home/timo/.cache/swarm-wave-2026-09-13c/checker/baseline/check-two-agents.py <retained-input> --json`. Its stdout is /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason-legacy.json, exit 0. No Python code was added, and the Rust tests invoke no Python.

The saved legacy executable was byte-compared to `git show c935d85:examples/two-agents/check-two-agents.py`; equal, SHA256 9f4fd72f2b06b40c61a2ead35349c6cf9a692b772108622bfcdbeef7454b716a. Thus the compatibility differences are introduced by this port.

## 3. Suite after all cases existed

Before count 98 is from the implementor report, not a pre-attack suite run. Command: `cargo test -p swarm-check --no-fail-fast`, with the environment above. Exit 101. Actual results: 74 regression + 9 metadata + 15 compatibility passed; all 5 new tests failed; 103 executed total. Full output:

```text
   Compiling swarm-check v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker/src/runtime/swarm-check)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running unittests src/lib.rs (target/debug/deps/swarm_check-4e051ca40dd9d014)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/swarm_check-07a831ae807832d1)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_port_parity.rs (target/debug/deps/adversary_port_parity-62a61d4df8cfdbbb)

running 5 tests
test unsigned_iteration_missing_transcript_retains_legacy_failing_verdict ... FAILED
test false_denial_reason_retains_legacy_empty_suffix ... FAILED
test integral_float_iterations_retain_legacy_spend_attribution ... FAILED
test boolean_iteration_missing_transcript_retains_legacy_failing_verdict ... FAILED
test structured_denial_reason_retains_legacy_rendering ... FAILED

failures:

---- unsigned_iteration_missing_transcript_retains_legacy_failing_verdict stdout ----

thread 'unsigned_iteration_missing_transcript_retains_legacy_failing_verdict' (1951804) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:81:5:
assertion `left == right` failed: legacy reader counts this claimed attempt: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"3 row(s), 2 agent(s) named: `builder` ×2, `coordinator` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":7,"not_met":0,"ok":true,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration.6tCMEh9AYVq9/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- false_denial_reason_retains_legacy_empty_suffix stdout ----

thread 'false_denial_reason_retains_legacy_empty_suffix' (1951801) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:101:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — False")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl"

---- integral_float_iterations_retain_legacy_spend_attribution stdout ----

thread 'integral_float_iterations_retain_legacy_spend_attribution' (1951802) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:60:5:
assertion `left == right` failed: legacy reader attributes integral float iterations: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 0 attributed: none; 0 distinct agent(s): none; 2 file(s) no agent is named on: 0001-01-73a4ac74.jsonl, 0002-01-73a4ac74.jsonl","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":false,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"2 row(s), 2 agent(s) named: `coordinator` ×1, `builder` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":6,"not_met":1,"ok":false,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations.XeBZ4UDu78Zu/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 1
 right: 0

---- boolean_iteration_missing_transcript_retains_legacy_failing_verdict stdout ----

thread 'boolean_iteration_missing_transcript_retains_legacy_failing_verdict' (1951800) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:43:5:
assertion `left == right` failed: legacy reader counts this claimed attempt: {"clauses":[{"found":"1 of 2 spawned agent(s): `builder` (Worker)","looked_for":"a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`","met":true,"number":1,"title":"a non-Coordinator agent exists"},{"found":"1 `swarm.agent.AssignmentPosted` (to `builder`); 1 `swarm.agent.AssignmentTaken` (by `builder`); posted to `builder` at seq 6 and taken by it at seq 8, assignment 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f, and the `runtime` issued that taking, on the member's behalf","looked_for":"a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name","met":true,"number":2,"title":"work was handed over"},{"found":"2 turn file(s), 2 attributed: `coordinator` ×1 (spend.jsonl), `builder` ×1 (spend.jsonl); 2 distinct agent(s): builder, coordinator","looked_for":"two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept","met":true,"number":3,"title":"two transcripts, neither overwritten"},{"found":"`swarm.agent.AssignmentDone` at seq 9 by `builder`, which took the assignment at seq 8, both naming 5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f","looked_for":"a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which","met":true,"number":4,"title":"the work was finished, not merely started"},{"found":"1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — this step only admits `file.read` operations","looked_for":"a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything","met":true,"number":5,"title":"the confinement acted"},{"found":"3 row(s), 2 agent(s) named: `builder` ×2, `coordinator` ×1","looked_for":"rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned","met":true,"number":6,"title":"spend is attributed per agent"},{"found":"2 spawned agent(s) to bound: builder, coordinator; refused: `builder` (`swarm.agent.NoteEmitted` at seq 10)","looked_for":"a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had","met":true,"number":7,"title":"the bound held with more than one agent to bound"}],"met":7,"not_met":0,"ok":true,"swarm":"/home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration.EQTnpRMBKbsL/swarm","unattended":{"found":"8 event(s) from seq 3 to seq 10, the end of the log, every one naming its issuer and every one of them the runtime or an agent this log spawned; issuers: agent:coordinator, operator, runtime","looked_for":"at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction","met":true,"number":0,"title":"unattended"}}
  left: 0
 right: 1

---- structured_denial_reason_retains_legacy_rendering stdout ----

thread 'structured_denial_reason_retains_legacy_rendering' (1951803) panicked at src/runtime/swarm-check/tests/adversary_port_parity.rs:120:5:
assertion `left == right` failed
  left: String("1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {\"limits\":[1,2],\"why\":true}")
 right: "1 of 3 decided call(s) refused by the frame: `Bash` (shell) in 0002-01-73a4ac74.jsonl — {'why': True, 'limits': [1, 2]}"


failures:
    boolean_iteration_missing_transcript_retains_legacy_failing_verdict
    false_denial_reason_retains_legacy_empty_suffix
    integral_float_iterations_retain_legacy_spend_attribution
    structured_denial_reason_retains_legacy_rendering
    unsigned_iteration_missing_transcript_retains_legacy_failing_verdict

test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

error: test failed, to rerun pass `-p swarm-check --test adversary_port_parity`
     Running tests/compatibility.rs (target/debug/deps/compatibility-627c867870c5f522)

running 15 tests
test absent_and_corrupt_sqlite_are_not_created_or_modified ... ok
test missing_database ... ok
test corrupt_database ... ok
test legacy_turn_names ... ok
test missing_turns ... ok
test missing_assignment_identity ... ok
test malformed_event_payloads ... ok
test unreadable_capped_record_fails_closed ... ok
test unreadable_transcript_fails_closed ... ok
test denial_words ... ok
test bound_object ... ok
test malformed_json_lines ... ok
test bare_denial ... ok
test slug_with_data_root_has_the_same_json_and_exit_status ... ok
test unknown_issuer_mark ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/metadata.rs (target/debug/deps/metadata-2f52d28497c9a153)

running 9 tests
test test_adversary_pass2_thefixtureagainstthedomain_test_the_fixture_spawns_a_role_the_domain_has_no_variant_for ... ok
test test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_the_guard_is_green_against_the_four ... ok
test test_checker_theissuervocabulary_test_a_variant_with_no_label_arm_at_all_is_caught_by_name ... ok
test test_checker_theissuervocabulary_test_the_checker_names_every_value_the_runtime_can_write ... ok
test test_checker_theissuervocabulary_test_a_variant_whose_label_is_computed_is_a_missing_label_and_not_a_silence ... ok
test test_adversary_issuer_pass_2_thevocabularyguardonastructvariant_test_a_struct_variant_is_a_missing_label_and_not_a_silence ... ok
test test_adversary_issuer_pass_1_theissuervocabularyguard_test_a_fifth_variant_whose_label_arm_is_a_block_is_caught ... ok
test test_adversary_issuer_pass_1_theissuervocabularyguard_test_the_guard_is_green_against_the_store_it_was_written_for ... ok
test test_checker_theissuervocabulary_test_the_fixture_speaks_the_same_vocabulary ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/regression.rs (target/debug/deps/regression-2149ec56e36e22e3)

running 74 tests
test test_adversary_issuer_pass_1_theunattendedconditiononanemptywindow_test_a_window_holding_only_the_excepted_opener_is_not_unattended ... ok
test test_adversary_clausethreeagainstitsownsentence_test_not_met_when_the_two_transcripts_name_agents_the_log_never_spawned ... ok
test test_adversary_clausesevenagainstclausesix_test_not_met_when_the_second_agent_to_bound_is_only_a_name_on_a_spend_row ... ok
test test_adversary_issuer_pass_2_thewindowthatrunspasttheverdict_test_an_operator_after_the_verdict_is_not_a_hand_on_the_run ... ok
test test_adversary_pass2_thereadmeagainsttheruntime_test_the_readme_names_every_clause_no_run_can_meet ... ok
test test_adversary_pass2_clausetwoandthehandoveritnames_test_clause_two_names_a_posting_that_was_never_answered ... ok
test test_checker_clausefour_test_not_met_when_nobody_took_the_work_there_is_no_finisher_to_check ... ok
test test_checker_clausefive_test_not_met_when_the_denial_did_not_come_from_the_frame ... ok
test test_checker_clausefive_test_not_met_when_there_are_no_turn_records_at_all ... ok
test test_checker_clausefive_test_met_when_the_frame_refused_a_tool_call ... ok
test test_adversary_issuer_pass_1_theexportedevidence_test_the_evidence_quotes_an_unattended_line_this_checker_still_prints ... ok
test test_adversary_clausefiveagainsttherealcensus_test_not_met_when_the_frame_only_allowed_and_another_decider_denied ... ok
test test_checker_clausefour_test_met_by_a_green_gate ... ok
test test_checker_clausefour_test_met_by_a_finished_assignment ... ok
test test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_recorded_before_the_work_was_posted ... ok
test test_checker_clausefive_test_not_met_when_every_call_was_allowed ... ok
test test_adversary_clausethreeagainstitsownsentence_test_not_met_when_an_attempt_left_no_transcript_at_any_turn_number ... ok
test test_adversary_issuer_pass_2_amemberwhoseslugbeginswithaquestionmark_test_a_member_that_took_its_own_work_is_not_read_as_a_digest ... ok
test test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_of_an_assignment_that_was_never_taken ... ok
test test_checker_clausefour_test_not_met_when_the_coordinator_reports_a_green_gate_on_the_builder_s_work ... ok
test test_checker_clausefour_test_not_met_when_the_work_was_only_started ... ok
test test_adversary_theunattendedconditionagainstareallog_test_an_operator_pausing_and_resuming_by_hand_is_not_unattended ... ok
test test_checker_clauseone_test_met_when_an_agent_was_spawned_with_a_role_that_is_not_coordinator ... ok
test test_checker_clauseseven_test_not_met_when_there_was_only_one_agent_to_bound ... ok
test test_checker_clauseone_test_not_met_when_every_spawned_agent_is_a_coordinator ... ok
test test_checker_clauseone_test_not_met_when_nothing_was_spawned ... ok
test test_checker_clauseseven_test_met_when_the_agent_ceiling_refused_an_agent ... ok
test test_checker_clauseseven_test_met_when_the_refusal_is_recorded_on_the_spend_row ... ok
test test_checker_clauseseven_test_met_when_the_refusal_is_recorded_beside_the_transcripts ... ok
test test_checker_clauseseven_test_not_met_when_nothing_was_refused ... ok
test test_checker_clausesix_test_not_met_when_the_second_named_agent_was_never_spawned ... ok
test test_checker_clausesix_test_met_when_two_agents_each_name_themselves ... ok
test test_checker_clausesix_test_not_met_when_there_is_no_spend_record ... ok
test test_checker_clausethree_test_not_met_when_an_attempt_left_no_file_beside_it ... ok
test test_checker_clausesix_test_not_met_when_only_one_agent_is_named ... ok
test test_checker_clausethree_test_not_met_when_both_transcripts_are_the_same_agent ... ok
test test_checker_clausethree_test_met_when_two_turn_files_are_attributed_to_two_agents ... ok
test test_checker_clausesix_test_not_met_when_the_rows_name_nobody ... ok
test test_checker_thecommandline_test_an_absent_swarm_exits_one_rather_than_raising ... ok
test test_checker_clausethree_test_not_met_when_no_turn_file_can_be_attributed ... ok
test test_checker_clausethree_test_not_met_with_one_transcript ... ok
test test_checker_clausetwo_test_it_says_it_is_coping_with_a_log_written_before_issuers ... ok
test test_checker_clausetwo_test_met_when_the_member_itself_issued_the_taking ... ok
test test_checker_theshapeofthereport_test_every_clause_is_reported_even_when_there_is_no_event_log ... ok
test test_checker_clausetwo_test_not_met_when_nothing_was_posted ... ok
test test_checker_theshapeofthereport_test_every_clause_is_reported_when_the_swarm_directory_is_absent ... ok
test test_checker_clausetwo_test_met_when_the_runtime_issued_the_taking ... ok
test test_checker_clausetwo_test_not_met_when_an_operator_issued_the_taking ... ok
test test_checker_clausetwo_test_met_when_the_agent_it_was_posted_to_took_it ... ok
test test_checker_clausetwo_test_not_met_when_another_agent_took_it ... ok
test test_checker_clausetwo_test_not_met_when_another_agent_issued_the_taking_on_its_behalf ... ok
test test_checker_theshapeofthereport_test_nothing_passes_by_finding_nothing ... ok
test test_checker_clausetwo_test_not_met_when_the_assignment_was_posted_and_never_taken ... ok
test test_checker_thecommandline_test_a_swarm_that_misses_one_clause_exits_one_and_names_the_number ... ok
test test_checker_clausetwo_test_not_met_when_the_taking_came_before_the_posting ... ok
test test_checker_clausetwo_test_not_met_when_an_earlier_handover_was_the_forged_one ... ok
test test_checker_clausetwo_test_not_met_when_the_taking_names_an_agent_the_log_never_spawned ... ok
test test_checker_thecommandline_test_every_clause_number_is_in_the_output ... ok
test test_checker_thecommandline_test_a_swarm_that_meets_everything_exits_zero ... ok
test test_checker_thecommandline_test_json_output_carries_every_clause ... ok
test test_checker_theshapeofthereport_test_a_not_met_clause_says_what_it_looked_for_and_what_it_found ... ok
test test_checker_theshapeofthereport_test_no_clause_is_met_by_a_name_the_log_never_spawned ... ok
test test_checker_theshapeofthereport_test_a_swarm_that_meets_everything_meets_all_seven ... ok
test test_checker_theunattendedcondition_test_a_swarm_that_never_started_is_not_reported_as_unattended ... ok
test test_checker_theshapeofthereport_test_no_clause_is_met_by_a_tally_instead_of_a_record ... ok
test test_checker_theunattendedcondition_test_it_still_does_not_decide_the_exit_status ... ok
test test_checker_theunattendedcondition_test_met_when_every_command_in_the_window_came_from_the_loop ... ok
test test_checker_theunattendedcondition_test_not_met_when_the_log_predates_issuers_and_it_says_so ... ok
test test_checker_theunattendedcondition_test_not_met_when_an_operator_reached_into_the_window ... ok
test test_checker_theunattendedcondition_test_not_met_when_an_issuer_names_an_agent_the_log_never_spawned ... ok
test test_checker_theunattendedcondition_test_the_actor_column_no_longer_decides_it ... ok
test test_checker_theunattendedcondition_test_the_command_that_starts_a_swarm_does_not_count_against_it ... ok
test test_checker_clausethree_test_met_by_each_way_a_turn_file_names_its_agent ... ok
test test_checker_theshapeofthereport_test_each_clause_can_be_taken_the_other_way_on_its_own ... ok

test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.52s

   Doc-tests swarm_check

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p swarm-check --test adversary_port_parity`
```

## 4. Findings against d0eac57

| Location | Verdict | Origin | Measured finding | What reaches it |
|---|---|---|---|---|
| src/runtime/swarm-check/src/clauses.rs:273 (also :253) | NEEDS-CHANGE | introduced | Numeric spend compatibility is narrower than the legacy reader: boolean and large unsigned missing attempts become a successful verdict, while integral-float attribution becomes a failing verdict. New tests at :43, :60, :81 above fail; isolated and suite exits 101. | The documented directory CLI in examples/two-agents/README.md:16 invokes main.rs:24 → check in lib.rs:38 → read_jsonl → clause three. User-supplied readable spend files reach it with no schema rejection. The story explicitly includes malformed-input differential parity. No runtime emitter found for boolean/float iterations; record_spend in swarm-server/src/swarm.rs:470 accepts u64 and writes it at :485, so the unsigned input is representable by the API, but no ordinary run reaching u64::MAX was established. |
| src/runtime/swarm-check/src/clauses.rs:529 (also records.rs:130) | NEEDS-CHANGE | introduced | Non-string denial reasons change the clause-five found field through different truthiness and composite-value rendering. New tests at :101 and :120 fail; isolated and suite exits 101; both legacy and Rust CLI exit 0. | The same documented CLI loads tool.decided rows via records.rs:246 and clauses.rs:456, then serializes the found string through main.rs:29. Readable malformed reason fields are part of the explicit parsed-JSON/reason compatibility promise. No actual runtime emitter of false/object reason fields was found; this is a malformed-file contract issue, not a claim about the historical demonstration. |

Both are acceptance contract findings. The explicit exception covers inaccessible evidence and error text; these inputs are fully readable. Restoring compatibility must preserve the distinction between matching a numeric iteration for attribution and deciding whether it creates a missing-attempt claim. For composite reason text, changing outer JSON whitespace cannot restore equality because the text is itself the value of found. No new evidence criteria are requested.

## 5. Attacks that did not break the unit

- Audited all 83 original case names against mapping and corpus keys, and byte-equivalent parsed corpus against the retained pre-port capture. None missing. Seven independent-clause subcases preserve [1,2,4], [2,4], [3], [4], [5], [6], [7] failures; four attribution subcases retain distinct spend, record, cwd and filename reasons.
- The 74 behavioral cases compare full parsed JSON and CLI exits through assert_case; nine metadata cases retain variant-name failures for missing, computed/block and struct arms, real-store agreement, fixture-domain role checks, README clause coverage and exported unattended text. All ran green in this pass.
- Assignment identities, last handover, early forged handover, known-agent claims, pre-issuer logs, question-mark slug, final-verdict window, seven-versus-unattended exit policy, denial vocabulary, missing/corrupt SQLite and inaccessible transcript/capped-file regressions all remained green.
- Read coordinator current-command changes in integration README.md, AGENTS.md, store.rs and website scripts. No stale current checker command found there. Historical exported evidence has no unit diff.
- Original demonstration was not mutated or rerun. Its prior differential comparison and input hashes are in the implementor report. No paid model, daemon, remote integration or AEP command was called.
- Free space was 25 GiB before and after testing, above the assigned 10 GiB minimum.

## 6. Outside-worktree paths

Retained files in assigned scratch (the brief existed before this pass and is not counted):

- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-iteration/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/boolean-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/false-reason/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-iterations/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/float-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/report.md
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason-legacy.json
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/structured-reason/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/suite.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-alone.log
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration/eventlog.sqlite3
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration/turns/0001-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration/turns/0002-01-73a4ac74.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-iteration/turns/spend.jsonl
- /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1/unsigned-legacy.json

TempDir-created per-case subdirectories under /home/timo/.cache/swarm-wave-2026-09-13c/checker/adversary-1 were dropped by the test fixture RAII on completion; no scratch data was manually removed. Cargo used the assigned worktree target only. Tool-managed writes also touch /home/timo/.cache/sccache (existing shared compiler cache; bounded 10 GiB) and /home/timo/.local/state/worktree (own session lease only). Their exact internal cache/registry files are owned by those tools, not custom scratch artifacts. No alternate target or /tmp path was used.

The worktree is /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-checker, id swarm-wave-20260913c-checker, branch wave/2026-09-13c/checker. Commit covered is d0eac57ed4f8cf0fdacbe622c61b06ba0891b58e plus the added test file. The parent coordinator owns correction routing and eventual lifecycle cleanup. Own lease wave-20260913c-checker-adversary-1 was released with worktree hook session-end, exit 0, on report handoff.

```findings
- file: src/runtime/swarm-check/src/clauses.rs
  line: 273
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Numeric spend compatibility is narrower than the legacy reader, reversing clause-three and CLI verdicts for boolean, integral-float, and large unsigned iteration inputs.
- file: src/runtime/swarm-check/src/clauses.rs
  line: 529
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Non-string denial reasons change the clause-five found field through different truthiness and composite-value rendering.
```

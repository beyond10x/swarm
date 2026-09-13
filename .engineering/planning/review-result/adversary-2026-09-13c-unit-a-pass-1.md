---
format: aep.planning-md/1
id: review-result:adversary-2026-09-13c-unit-a-pass-1
kind: review-result
status: active
title: Runtime control adversary pass 1
relations:
- reviews: story:pause-and-stop-control-runtime-work
revision: 1
---
unit: story:pause-and-stop-control-runtime-work — pass 1 at 4098ad109c9b36eb7c074d4a132bbd7c11055582
verdict: NEEDS-CHANGE
cases: executed 129→130, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned scratch root and 3 retained files; inventory in part 6
needs-coordinator: route the worker bookkeeping race back to implementation

1. Test-only diffstat

`git --no-pager diff --stat` exits 0 with no tracked edits. Including the untracked new test with `git --no-pager diff --no-index --stat /dev/null src/runtime/swarm-server/tests/adversary_control_pass_1.rs` (exit 1 denotes a present diff):

```
 .../swarm-server/tests/adversary_control_pass_1.rs | 288 +++++++++++++++++++++
 1 file changed, 288 insertions(+)
```

No implementation, existing test, planning store, branch or commit changes. The new file includes fixture helpers copied from the implementation test, and one new case. No mutant was made.

2. Case first, isolated execution first

`src/runtime/swarm-server/tests/adversary_control_pass_1.rs:284`, `pause_after_finish_does_not_strand_a_worker_in_working`, is RED now. It asserts that a worker whose assignment completed just before acknowledged pause can resume in Idle, allowing normal reassignment. The observed worker remains Working while its assignment is Done, and the OpenAssignments view is empty after real HTTP ResumeSwarm and a dispatch attempt.

The test polls the actual public `coordinator::work_an_assignment` future until it has recorded spend and yields while FinishAssignment persists with the world lock held. A real HTTP PauseSwarm request queues for that world lock before the worker is polled again. A small router middleware only signals that the handler is entered; it does not replace or alter any handler, response, command or state. On the single-thread Tokio runtime the pause handler reaches its lock wait before that signal's receiver continues. This controls scheduling at an existing async boundary. The worker then finishes its assignment, pause commits next, and the worker's separate GoIdle acquisition sees the invalidated generation. HTTP pause nevertheless returns 200 paused. HTTP resume returns 200, the assignment is Done, dispatch sees no open assignment, and the worker remains Working.

No model calls. The fake shell program returns a completed verdict under the supported Program launch protocol. Timeouts are failure bounds; no sleeps coordinate the case. The production caller is `trigger::one_assignment` at `src/runtime/swarm-server/src/trigger.rs:307`, which awaits exactly the worker function driven here. The two writes in that function acquire the same world lock separately, so an HTTP lifecycle handler can queue between them in ordinary operation.

First command, before any suite run (exit 101, executed 1 test):

`RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=/home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1 cargo test -p swarm-server --test adversary_control_pass_1 pause_after_finish_does_not_strand_a_worker_in_working -- --exact`

Verbatim isolated output (the new file was subsequently rustfmt-formatted, moving assertion line 227 to 284 without changing behavior):

```
   Compiling swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control/src/runtime/swarm-server)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.78s
     Running tests/adversary_control_pass_1.rs (target/debug/deps/adversary_control_pass_1-e2c2db48d54951a8)

running 1 test
test pause_after_finish_does_not_strand_a_worker_in_working ... FAILED

failures:

---- pause_after_finish_does_not_strand_a_worker_in_working stdout ----

thread 'pause_after_finish_does_not_strand_a_worker_in_working' (1954765) panicked at src/runtime/swarm-server/tests/adversary_control_pass_1.rs:227:5:
assertion `left == right` failed: a finished assignment must not leave its worker permanently Working across pause/resume
  left: String("Working")
 right: "Idle"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    pause_after_finish_does_not_strand_a_worker_in_working

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

error: test failed, to rerun pass `-p swarm-server --test adversary_control_pass_1`

```

3. Suite after the new case

Before count 129 is taken from the implementor's GREEN report, not a pre-case suite run. Full package suite command (exit 101):

`RUSTC_WRAPPER=/usr/bin/sccache CARGO_BUILD_JOBS=2 TMPDIR=/home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1 cargo test -p swarm-server --no-fail-fast`

The runner summaries sum to 130 executed: 129 passed, 1 failed, 0 ignored. `--no-fail-fast` allows every existing target to execute after the adversarial case fails. The excluded wall-clock race target remains excluded per AGENTS.md. No AEP call was made and this is not a claim to have rerun the full repository gate.

Verbatim suite output:

```
   Compiling swarm-server v0.2.0 (/home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control/src/runtime/swarm-server)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.66s
     Running unittests src/lib.rs (target/debug/deps/swarm_server-88d9f0e4d5034626)

running 48 tests
test budget::retries::a_turn_that_never_answers_still_counts ... ok
test budget::tests::a_goal_under_both_caps_runs ... ok
test budget::tests::turns_are_checked_before_spend ... ok
test budget::tests::a_lifted_cap_never_stops_anything ... ok
test budget::tests::an_unpriced_turn_cannot_trip_the_spend_cap ... ok
test coordinator::tests::a_metaharness_turn_with_no_frame_has_no_argv_at_all ... ok
test coordinator::tests::a_refusal_at_the_seam_is_counted_and_not_merely_streamed_past ... ok
test coordinator::resolution::two_agents_never_share_a_directory_however_their_slugs_are_spelled ... ok
test coordinator::tests::a_verdict_is_read_from_the_last_line ... ok
test coordinator::tests::a_turn_launches_under_a_frame_and_never_under_observe ... ok
test coordinator::tests::both_prompts_say_where_an_agent_writes_and_where_it_may_only_read ... ok
test coordinator::tests::usage_is_summed_and_cost_is_read_not_computed ... ok
test coordinator::tests::the_last_verdict_wins_and_prose_is_not_one ... ok
test frame::tests::an_operation_metaharness_does_not_know_is_refused_by_name ... ok
test coordinator::tests::the_prompt_states_what_the_frame_admits_and_where ... ok
test frame::tests::the_hash_answers_the_published_vectors ... ok
test state::tests::a_record_whose_state_cannot_be_read_counts_as_live ... ok
test frame::tests::the_digest_is_metaharnesss_own_and_the_fixture_proves_it ... ok
test state::tests::the_removal_and_the_listing_read_the_state_field_the_same_way ... ok
test frame::tests::the_shared_directory_cannot_admit_what_the_turn_does_not ... ok
test frame::tests::a_frame_this_runtime_seals_states_the_digest_its_contents_imply ... ok
test frame::tests::the_scope_is_the_agents_own_directory_and_a_readable_shared_one ... ok
test frame::tests::an_edited_frame_no_longer_states_its_own_digest ... ok
test frame::tests::the_operations_are_sealed_in_one_order_whatever_order_they_arrive_in ... ok
test coordinator::resolution::the_config_declares_what_a_frame_admits_and_never_where ... ok
test swarm::roster::a_role_that_is_not_the_coordinator_is_declared ... ok
test swarm::attribution::an_agents_total_asks_nothing_about_the_goal ... ok
test swarm::tests::a_delivery_that_cannot_apply_is_bounded_and_announced ... ok
test swarm::attribution::spend_is_attributable_per_agent_as_well_as_per_goal ... ok
test control::tests::aborting_the_turn_does_not_unregister_its_live_process ... ok
test control::tests::claim_before_pause_cannot_spawn_or_publish_and_ack_waits_for_release ... ok
test control::tests::failed_termination_is_an_http_host_error_and_cleanup_is_retryable ... ok
test control::tests::a_finished_process_cannot_publish_across_pause_and_rapid_resume ... ok
test swarm::tests::a_due_delivery_that_applies_is_committed_and_no_longer_owed ... ok
test swarm::roster::a_second_agent_gets_a_mailbox_of_its_own ... ok
test coordinator::resolution::a_config_naming_claude_resolves_to_metaharness ... ok
test coordinator::resolution::a_config_narrows_the_frame_and_is_refused_when_it_would_leave_nothing ... ok
test coordinator::resolution::the_account_of_a_launch_names_only_the_model_that_launch_will_use ... ok
test coordinator::resolution::a_config_this_host_cannot_launch_is_refused_rather_than_replaced ... ok
test coordinator::resolution::the_environment_answers_when_no_config_does ... ok
test coordinator::resolution::the_swarms_config_names_the_coordinator_that_runs ... ok
test coordinator::tests::a_member_is_bounded_by_its_own_knobs_and_never_by_the_coordinators ... ok
test coordinator::resolution::with_no_config_and_no_variable_the_default_is_metaharness_and_says_so ... ok
test trigger::bounds::a_coordinator_that_never_answers_is_stopped_at_three_attempts ... ok
test trigger::bounds::a_goal_with_a_cap_of_three_stops_at_three_turns ... ok
test trigger::bounds::a_stuck_agent_is_stopped_across_two_goals_by_the_other_guard ... ok
test trigger::bounds::an_agent_is_stopped_by_its_own_record_across_two_goals ... ok
test trigger::bounds::no_cap_is_lifted_by_a_value_that_is_merely_wrong ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s

     Running unittests src/main.rs (target/debug/deps/swarm_server-7b3efebd0b1d7af3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/a_second_agent_runs.rs (target/debug/deps/a_second_agent_runs-a88dcb1f16845593)

running 8 tests
test two_units_of_work_are_two_claims ... ok
test a_cap_report_says_which_kind_of_unit_it_is_about ... ok
test two_agents_on_one_unit_write_two_transcripts ... ok
test a_cap_that_refuses_a_turn_is_recorded_where_a_reader_finds_it ... ok
test the_runtime_runs_a_member_and_assignment_taken_fires ... ok
test a_cap_published_live_says_whose_turn_it_refused ... ok
test the_watch_stream_says_which_agent_a_turn_belongs_to ... ok
test two_members_of_one_swarm_do_not_share_one_work_directory ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s

     Running tests/a_swarm_is_removed.rs (target/debug/deps/a_swarm_is_removed-bb6c525e92dd4fb9)

running 11 tests
test a_handle_can_only_live_where_this_lookup_looks ... ok
test a_slug_the_server_does_not_hold_is_not_found ... ok
test a_place_is_not_made_outside_the_swarms_directory_either ... ok
test a_place_with_no_swarm_in_it_is_removed ... ok
test no_route_that_takes_a_slug_reaches_outside_the_swarms_directory ... ok
test a_running_swarm_is_refused_with_the_specification_s_own_error ... ok
test a_slug_holding_a_live_record_is_listed_however_its_records_sort ... ok
test one_rule_for_what_a_slug_is_holds_at_every_entry_point ... ok
test a_swarm_in_a_terminal_state_is_not_in_the_list ... ok
test a_deleted_swarm_takes_its_log_and_its_wal_with_it ... ok
test a_removal_racing_an_open_leaves_the_map_agreeing_with_the_disk ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.31s

     Running tests/a_turn_is_confined_by_a_frame.rs (target/debug/deps/a_turn_is_confined_by_a_frame-d6a5a0a1c7a815e6)

running 2 tests
test a_refusal_reaches_the_turns_own_record_and_its_figures ... ok
test the_frame_a_turn_ran_under_is_named_by_the_runtime_and_kept_beside_its_transcript ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s

     Running tests/adversary_control_pass_1.rs (target/debug/deps/adversary_control_pass_1-e2c2db48d54951a8)

running 1 test
test pause_after_finish_does_not_strand_a_worker_in_working ... FAILED

failures:

---- pause_after_finish_does_not_strand_a_worker_in_working stdout ----

thread 'pause_after_finish_does_not_strand_a_worker_in_working' (1956395) panicked at src/runtime/swarm-server/tests/adversary_control_pass_1.rs:284:5:
assertion `left == right` failed: a finished assignment must not leave its worker permanently Working across pause/resume
  left: String("Working")
 right: "Idle"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    pause_after_finish_does_not_strand_a_worker_in_working

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

error: test failed, to rerun pass `-p swarm-server --test adversary_control_pass_1`
     Running tests/adversary_issuer_pass_1.rs (target/debug/deps/adversary_issuer_pass_1-046cb4b9b91c573c)

running 1 test
test two_members_whose_names_the_log_cannot_hold_are_not_the_same_record ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/adversary_pending_removal.rs (target/debug/deps/adversary_pending_removal-91b31f41bd28ee27)

running 3 tests
test a_retried_delete_does_not_answer_204_while_the_holder_is_alive ... ok
test a_retried_removal_does_not_unlink_under_a_holder_the_first_one_parked ... ok
test a_parked_slug_holding_a_live_record_still_refuses_removal ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/adversary_removal.rs (target/debug/deps/adversary_removal-5a60f93061446097)

running 6 tests
test a_removal_does_not_reach_outside_the_swarms_directory ... ok
test a_command_on_a_handle_taken_before_a_removal_is_not_told_it_succeeded ... ok
test what_an_escaping_slug_answers ... ok
test a_running_swarm_is_not_removed_under_an_alias_of_its_slug ... ok
test a_slug_holding_a_running_record_is_refused_however_its_records_sort ... ok
test listing_does_not_wait_on_every_swarm_s_world ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s

     Running tests/an_event_says_which_agent_acted.rs (target/debug/deps/an_event_says_which_agent_acted-7bcccea2ca016e95)

running 6 tests
test a_name_the_log_cannot_hold_is_neither_refused_nor_recorded_as_somebody_else ... ok
test a_command_issued_about_another_member_is_not_that_members_own_action ... ok
test an_operator_at_the_door_is_not_the_runtime_issuing_to_itself ... ok
test two_members_of_one_actor_type_are_two_records ... ok
test one_member_the_log_cannot_name_keeps_one_mark_across_its_commands ... ok
test the_mail_door_says_which_agent_posted ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

     Running tests/open_is_one_swarm_under_contention.rs (target/debug/deps/open_is_one_swarm_under_contention-40e3b9ba9804fded)

running 2 tests
test an_open_with_no_handle_to_fall_back_on_is_still_refused ... ok
test no_concurrent_open_of_one_slug_is_refused ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s

     Running tests/pause_and_stop_control_runtime_work.rs (target/debug/deps/pause_and_stop_control_runtime_work-aab41e7009295cca)

running 8 tests
test fake_process ... ok
test fake_descendant ... ok
test every_nonrunning_state_refuses_goals_and_assignments_before_and_after_replay ... ok
test aborted_turn_preserves_observed_usage_until_pause_reaps ... ok
test pause_isolates_the_other_swarm_and_reaps_children_of_exited_launchers ... ok
test paused_assignments_do_not_launch ... ok
test pause_reaps_coordinator_worker_and_descendants_then_resume_continues ... ok
test stop_reaps_coordinator_worker_and_descendants_then_start_continues ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s

     Running tests/redelivery_under_attack.rs (target/debug/deps/redelivery_under_attack-09d823b0c2f35eef)

running 2 tests
test a_retry_under_the_same_request_key_answers_wrong_state_today ... ok
test one_request_key_spent_twice_on_a_creating_command_creates_two_instances_today ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/serves_a_swarm.rs (target/debug/deps/serves_a_swarm-085b8d1f01545432)

running 7 tests
test the_ui_can_ask_the_server_what_the_system_is ... ok
test two_opens_of_one_slug_at_once_are_one_swarm ... ok
test a_command_the_specification_refuses_is_a_bad_request_not_a_crash ... ok
test a_failed_at_least_once_delivery_is_kept_and_retried_rather_than_dropped ... ok
test a_redelivery_that_succeeds_commits_what_the_first_attempt_could_not ... ok
test a_swarm_is_created_started_and_runs_its_loop_to_a_reached_goal ... ok
test a_swarm_survives_the_server_being_restarted ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/the_cascade_of_a_caused_command.rs (target/debug/deps/the_cascade_of_a_caused_command-0fd10a9437cb7ebb)

running 3 tests
test a_delivery_the_cascade_owes_survives_the_drain_that_caused_it ... ok
test a_redelivered_commands_failing_cascade_does_not_hang_the_swarm ... ok
test a_command_a_period_caused_is_pumped_like_one_a_caller_asked_for ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

     Running tests/the_ceiling_under_attack.rs (target/debug/deps/the_ceiling_under_attack-97b3e18bf1057f2a)

running 2 tests
test a_goal_that_spent_nothing_is_refused_for_what_its_agent_spent ... ok
test a_capped_goal_reports_the_figures_the_bound_was_measured_on ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/the_frame_states_the_attempt_it_frames.rs (target/debug/deps/the_frame_states_the_attempt_it_frames-9ab093013ded562a)

running 1 test
test the_frame_a_retry_runs_under_states_the_attempt_it_is ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/the_request_key_contract.rs (target/debug/deps/the_request_key_contract-74605e8beb0a54b8)

running 4 tests
test every_document_of_the_request_key_states_the_same_contract ... ok
test no_document_presents_a_refuted_promise_as_current ... ok
test only_the_log_carries_the_request_a_record_was_written_under ... ok
test omitting_the_request_key_gives_up_the_only_guarantee_it_has ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/the_request_key_contract_under_attack.rs (target/debug/deps/the_request_key_contract_under_attack-ef81cfc9724e12bf)

running 2 tests
test a_refuted_promise_wrapped_across_a_comment_line_is_still_a_refuted_promise ... ok
test the_log_answers_only_within_the_window_it_returns ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s

     Running tests/the_second_agent_under_attack_pass_2.rs (target/debug/deps/the_second_agent_under_attack_pass_2-7e3ac87142d9aaee)

running 6 tests
test a_members_launch_is_not_reported_with_the_coordinators_model ... ok
test an_admitted_subject_beside_an_outside_one_does_not_admit_the_outside_one ... ok
test every_single_subject_claim_the_scope_doc_makes_holds_under_metaharnesss_matcher ... ok
test a_members_first_turn_at_a_new_assignment_is_not_capped_as_that_assignments_own ... ok
test an_activated_config_cannot_admit_more_than_the_default_frame_does ... ok
test the_frame_of_a_third_attempt_says_three ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/the_turn_cap_under_attack.rs (target/debug/deps/the_turn_cap_under_attack-b080c717d41aef7d)

running 3 tests
test a_negative_spend_cap_is_not_a_lifted_spend_cap ... ok
test an_agents_spend_is_bounded_across_the_goals_it_works ... ok
test an_answered_turn_names_the_agent_that_spent_it ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/the_turn_cap_under_attack_pass_2.rs (target/debug/deps/the_turn_cap_under_attack_pass_2-13062266ad2409db)

running 4 tests
test a_cap_of_n_stops_at_exactly_n ... ok
test a_spend_cap_of_nan_is_not_a_cap ... ok
test resolve_refuses_rather_than_coin_flipping_between_two_active_configs ... ok
test a_goal_stopped_by_the_silent_guard_is_still_reported_capped ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.78s

   Doc-tests swarm_server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p swarm-server --test adversary_control_pass_1`

```

4. Findings

Covered commit: 4098ad109c9b36eb7c074d4a132bbd7c11055582, plus the sole untracked adversarial test.

| File:line | Category / severity | Verdict | Origin | Finding |
|---|---|---|---|---|
| src/runtime/swarm-server/src/swarm.rs:829 | concurrency / blocker | NEEDS-CHANGE | introduced | Pause can invalidate the worker between committed FinishAssignment and its separate GoIdle, leaving a Done assignment with a Working agent that normal resume and dispatch cannot repair. |

What was measured: `adversary_control_pass_1.rs:284`, isolated and full-suite exit 101, actual state Working versus required Idle; all preceding assertions show successful real HTTP pause/resume, Done assignment and no eligible assignment for dispatch.

What reaches it: `trigger::one_assignment` → `coordinator::work_an_assignment` → `work_an_assignment_claimed`; FinishAssignment at coordinator.rs:1471 completes before the separate GoIdle at :1493. A real HTTP PauseSwarm handler competes for `Swarm::issue_as`'s world lock. The new blanket task-local generation check at swarm.rs:829 now rejects that GoIdle. The function only logs the failure and returns its successful Answer (:1508). Its own comment (:1483–1486) describes why a Working member cannot be retasked. Resuming does not select the completed assignment again.

Origin is introduced by the new generation guard making this previously reachable two-write sequence cancellable midway. `git show c935d85:.../swarm.rs` confirms the base has no guard; `git show c935d85:.../coordinator.rs` confirms the separate GoIdle was already present. No base execution or pre-existing reproduction is claimed.

The repair should preserve both cancellation and the member's bookkeeping, e.g. make successful completion and returning the member to Idle one serialized host operation, or provide an explicit safe reconciliation for committed completion. Merely allowing arbitrary cancelled turn commands through the generation check would weaken the late-result guarantee. This report applies no repair.

5. Attacks without additional findings

The package's existing process-group, orphan descendant, cancellation failure/retry, claim-before-launch, rapid reopening, dropped-future and cross-swarm cases all executed green alongside the new case.
The new case preserved the successful verdict and assignment completion before cancellation; it found the subsequent bookkeeping gap rather than a stale verdict after acknowledged cancellation.
Read README.md and AGENTS.md runtime-control paragraphs in the integration tree; no separate documentation finding is raised.
Agent-originated HTTP commands run in independent HTTP tasks and do not inherit the turn's task-local generation. No additional red case establishes that ownership boundary within this acceptance; claimed issuer identity is not authenticated ownership, so no containment or authenticated-client finding is asserted.
Linux is the tested process host. No unsupported platform or escaped-daemon guarantee was inferred.

6. Outside-worktree writes and handoff

Assigned scratch root used for all probe output and TMPDIR-backed test/compiler temporary files:
- /home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1

Retained files written by this pass:
- /home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1/isolated.log
- /home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1/suite.log
- /home/timo/.cache/swarm-wave-2026-09-13c/control/adversary-1/report.md

Test fixtures use tempdir beneath that exact assigned TMPDIR and its standard fixture lifetime cleanup; no ad hoc /tmp directory or external mutant/build copy was made. Cargo target remained /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control/target; the prescribed sccache wrapper managed its normal compiler cache. Worktree CLI managed this session's lease state. Free space was 25 GiB before and after the run, above the 10 GiB floor.

Managed tree id: swarm-wave-20260913c-control. Path: /home/timo/.local/state/worktree/trees/b10x/swarm/swarm-wave-20260913c-control. Branch: wave/2026-09-13c/control. HEAD: 4098ad109c9b36eb7c074d4a132bbd7c11055582. Own lease wave-20260913c-control-adversary-1 is released on report return; other leases and cleanup belong to the coordinator. Next owner: coordinator, to route the failing test back to the implementor and retain the new test. No commit or publication was attempted by this pass.

```findings
- file: src/runtime/swarm-server/src/swarm.rs
  line: 829
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Pause can invalidate the worker between committed FinishAssignment and its separate GoIdle, leaving a Done assignment with a Working agent that normal resume and dispatch cannot repair.
```

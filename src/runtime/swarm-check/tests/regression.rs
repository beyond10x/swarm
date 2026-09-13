mod support;

#[test]
fn test_adversary_clausefiveagainsttherealcensus_test_not_met_when_the_frame_only_allowed_and_another_decider_denied()
 {
    support::assert_case(
        "test_adversary.ClauseFiveAgainstTheRealCensus.test_not_met_when_the_frame_only_allowed_and_another_decider_denied",
    );
}

#[test]
fn test_adversary_clausesevenagainstclausesix_test_not_met_when_the_second_agent_to_bound_is_only_a_name_on_a_spend_row()
 {
    support::assert_case(
        "test_adversary.ClauseSevenAgainstClauseSix.test_not_met_when_the_second_agent_to_bound_is_only_a_name_on_a_spend_row",
    );
}

#[test]
fn test_adversary_clausethreeagainstitsownsentence_test_not_met_when_an_attempt_left_no_transcript_at_any_turn_number()
 {
    support::assert_case(
        "test_adversary.ClauseThreeAgainstItsOwnSentence.test_not_met_when_an_attempt_left_no_transcript_at_any_turn_number",
    );
}

#[test]
fn test_adversary_clausethreeagainstitsownsentence_test_not_met_when_the_two_transcripts_name_agents_the_log_never_spawned()
 {
    support::assert_case(
        "test_adversary.ClauseThreeAgainstItsOwnSentence.test_not_met_when_the_two_transcripts_name_agents_the_log_never_spawned",
    );
}

#[test]
fn test_adversary_theunattendedconditionagainstareallog_test_an_operator_pausing_and_resuming_by_hand_is_not_unattended()
 {
    support::assert_case(
        "test_adversary.TheUnattendedConditionAgainstARealLog.test_an_operator_pausing_and_resuming_by_hand_is_not_unattended",
    );
}

#[test]
fn test_adversary_issuer_pass_1_theexportedevidence_test_the_evidence_quotes_an_unattended_line_this_checker_still_prints()
 {
    support::assert_case(
        "test_adversary_issuer_pass_1.TheExportedEvidence.test_the_evidence_quotes_an_unattended_line_this_checker_still_prints",
    );
}

#[test]
fn test_adversary_issuer_pass_1_theunattendedconditiononanemptywindow_test_a_window_holding_only_the_excepted_opener_is_not_unattended()
 {
    support::assert_case(
        "test_adversary_issuer_pass_1.TheUnattendedConditionOnAnEmptyWindow.test_a_window_holding_only_the_excepted_opener_is_not_unattended",
    );
}

#[test]
fn test_adversary_issuer_pass_2_amemberwhoseslugbeginswithaquestionmark_test_a_member_that_took_its_own_work_is_not_read_as_a_digest()
 {
    support::assert_case(
        "test_adversary_issuer_pass_2.AMemberWhoseSlugBeginsWithAQuestionMark.test_a_member_that_took_its_own_work_is_not_read_as_a_digest",
    );
}

#[test]
fn test_adversary_issuer_pass_2_thewindowthatrunspasttheverdict_test_an_operator_after_the_verdict_is_not_a_hand_on_the_run()
 {
    support::assert_case(
        "test_adversary_issuer_pass_2.TheWindowThatRunsPastTheVerdict.test_an_operator_after_the_verdict_is_not_a_hand_on_the_run",
    );
}

#[test]
fn test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_of_an_assignment_that_was_never_taken()
 {
    support::assert_case(
        "test_adversary_pass2.ClauseFourAndTheAssignmentItNeverChecks.test_clause_four_is_met_by_a_finish_of_an_assignment_that_was_never_taken",
    );
}

#[test]
fn test_adversary_pass2_clausefourandtheassignmentitneverchecks_test_clause_four_is_met_by_a_finish_recorded_before_the_work_was_posted()
 {
    support::assert_case(
        "test_adversary_pass2.ClauseFourAndTheAssignmentItNeverChecks.test_clause_four_is_met_by_a_finish_recorded_before_the_work_was_posted",
    );
}

#[test]
fn test_adversary_pass2_clausetwoandthehandoveritnames_test_clause_two_names_a_posting_that_was_never_answered()
 {
    support::assert_case(
        "test_adversary_pass2.ClauseTwoAndTheHandoverItNames.test_clause_two_names_a_posting_that_was_never_answered",
    );
}

#[test]
fn test_adversary_pass2_thereadmeagainsttheruntime_test_the_readme_names_every_clause_no_run_can_meet()
 {
    support::assert_case(
        "test_adversary_pass2.TheReadmeAgainstTheRuntime.test_the_readme_names_every_clause_no_run_can_meet",
    );
}

#[test]
fn test_checker_clausefive_test_met_when_the_frame_refused_a_tool_call() {
    support::assert_case("test_checker.ClauseFive.test_met_when_the_frame_refused_a_tool_call");
}

#[test]
fn test_checker_clausefive_test_not_met_when_every_call_was_allowed() {
    support::assert_case("test_checker.ClauseFive.test_not_met_when_every_call_was_allowed");
}

#[test]
fn test_checker_clausefive_test_not_met_when_the_denial_did_not_come_from_the_frame() {
    support::assert_case(
        "test_checker.ClauseFive.test_not_met_when_the_denial_did_not_come_from_the_frame",
    );
}

#[test]
fn test_checker_clausefive_test_not_met_when_there_are_no_turn_records_at_all() {
    support::assert_case(
        "test_checker.ClauseFive.test_not_met_when_there_are_no_turn_records_at_all",
    );
}

#[test]
fn test_checker_clausefour_test_met_by_a_finished_assignment() {
    support::assert_case("test_checker.ClauseFour.test_met_by_a_finished_assignment");
}

#[test]
fn test_checker_clausefour_test_met_by_a_green_gate() {
    support::assert_case("test_checker.ClauseFour.test_met_by_a_green_gate");
}

#[test]
fn test_checker_clausefour_test_not_met_when_nobody_took_the_work_there_is_no_finisher_to_check() {
    support::assert_case(
        "test_checker.ClauseFour.test_not_met_when_nobody_took_the_work_there_is_no_finisher_to_check",
    );
}

#[test]
fn test_checker_clausefour_test_not_met_when_the_coordinator_reports_a_green_gate_on_the_builder_s_work()
 {
    support::assert_case(
        "test_checker.ClauseFour.test_not_met_when_the_coordinator_reports_a_green_gate_on_the_builder_s_work",
    );
}

#[test]
fn test_checker_clausefour_test_not_met_when_the_work_was_only_started() {
    support::assert_case("test_checker.ClauseFour.test_not_met_when_the_work_was_only_started");
}

#[test]
fn test_checker_clauseone_test_met_when_an_agent_was_spawned_with_a_role_that_is_not_coordinator() {
    support::assert_case(
        "test_checker.ClauseOne.test_met_when_an_agent_was_spawned_with_a_role_that_is_not_coordinator",
    );
}

#[test]
fn test_checker_clauseone_test_not_met_when_every_spawned_agent_is_a_coordinator() {
    support::assert_case(
        "test_checker.ClauseOne.test_not_met_when_every_spawned_agent_is_a_coordinator",
    );
}

#[test]
fn test_checker_clauseone_test_not_met_when_nothing_was_spawned() {
    support::assert_case("test_checker.ClauseOne.test_not_met_when_nothing_was_spawned");
}

#[test]
fn test_checker_clauseseven_test_met_when_the_agent_ceiling_refused_an_agent() {
    support::assert_case(
        "test_checker.ClauseSeven.test_met_when_the_agent_ceiling_refused_an_agent",
    );
}

#[test]
fn test_checker_clauseseven_test_met_when_the_refusal_is_recorded_beside_the_transcripts() {
    support::assert_case(
        "test_checker.ClauseSeven.test_met_when_the_refusal_is_recorded_beside_the_transcripts",
    );
}

#[test]
fn test_checker_clauseseven_test_met_when_the_refusal_is_recorded_on_the_spend_row() {
    support::assert_case(
        "test_checker.ClauseSeven.test_met_when_the_refusal_is_recorded_on_the_spend_row",
    );
}

#[test]
fn test_checker_clauseseven_test_not_met_when_nothing_was_refused() {
    support::assert_case("test_checker.ClauseSeven.test_not_met_when_nothing_was_refused");
}

#[test]
fn test_checker_clauseseven_test_not_met_when_there_was_only_one_agent_to_bound() {
    support::assert_case(
        "test_checker.ClauseSeven.test_not_met_when_there_was_only_one_agent_to_bound",
    );
}

#[test]
fn test_checker_clausesix_test_met_when_two_agents_each_name_themselves() {
    support::assert_case("test_checker.ClauseSix.test_met_when_two_agents_each_name_themselves");
}

#[test]
fn test_checker_clausesix_test_not_met_when_only_one_agent_is_named() {
    support::assert_case("test_checker.ClauseSix.test_not_met_when_only_one_agent_is_named");
}

#[test]
fn test_checker_clausesix_test_not_met_when_the_rows_name_nobody() {
    support::assert_case("test_checker.ClauseSix.test_not_met_when_the_rows_name_nobody");
}

#[test]
fn test_checker_clausesix_test_not_met_when_the_second_named_agent_was_never_spawned() {
    support::assert_case(
        "test_checker.ClauseSix.test_not_met_when_the_second_named_agent_was_never_spawned",
    );
}

#[test]
fn test_checker_clausesix_test_not_met_when_there_is_no_spend_record() {
    support::assert_case("test_checker.ClauseSix.test_not_met_when_there_is_no_spend_record");
}

#[test]
fn test_checker_clausethree_test_met_by_each_way_a_turn_file_names_its_agent() {
    support::assert_case(
        "test_checker.ClauseThree.test_met_by_each_way_a_turn_file_names_its_agent",
    );
}

#[test]
fn test_checker_clausethree_test_met_when_two_turn_files_are_attributed_to_two_agents() {
    support::assert_case(
        "test_checker.ClauseThree.test_met_when_two_turn_files_are_attributed_to_two_agents",
    );
}

#[test]
fn test_checker_clausethree_test_not_met_when_an_attempt_left_no_file_beside_it() {
    support::assert_case(
        "test_checker.ClauseThree.test_not_met_when_an_attempt_left_no_file_beside_it",
    );
}

#[test]
fn test_checker_clausethree_test_not_met_when_both_transcripts_are_the_same_agent() {
    support::assert_case(
        "test_checker.ClauseThree.test_not_met_when_both_transcripts_are_the_same_agent",
    );
}

#[test]
fn test_checker_clausethree_test_not_met_when_no_turn_file_can_be_attributed() {
    support::assert_case(
        "test_checker.ClauseThree.test_not_met_when_no_turn_file_can_be_attributed",
    );
}

#[test]
fn test_checker_clausethree_test_not_met_with_one_transcript() {
    support::assert_case("test_checker.ClauseThree.test_not_met_with_one_transcript");
}

#[test]
fn test_checker_clausetwo_test_it_says_it_is_coping_with_a_log_written_before_issuers() {
    support::assert_case(
        "test_checker.ClauseTwo.test_it_says_it_is_coping_with_a_log_written_before_issuers",
    );
}

#[test]
fn test_checker_clausetwo_test_met_when_the_agent_it_was_posted_to_took_it() {
    support::assert_case("test_checker.ClauseTwo.test_met_when_the_agent_it_was_posted_to_took_it");
}

#[test]
fn test_checker_clausetwo_test_met_when_the_member_itself_issued_the_taking() {
    support::assert_case(
        "test_checker.ClauseTwo.test_met_when_the_member_itself_issued_the_taking",
    );
}

#[test]
fn test_checker_clausetwo_test_met_when_the_runtime_issued_the_taking() {
    support::assert_case("test_checker.ClauseTwo.test_met_when_the_runtime_issued_the_taking");
}

#[test]
fn test_checker_clausetwo_test_not_met_when_an_earlier_handover_was_the_forged_one() {
    support::assert_case(
        "test_checker.ClauseTwo.test_not_met_when_an_EARLIER_handover_was_the_forged_one",
    );
}

#[test]
fn test_checker_clausetwo_test_not_met_when_an_operator_issued_the_taking() {
    support::assert_case("test_checker.ClauseTwo.test_not_met_when_an_operator_issued_the_taking");
}

#[test]
fn test_checker_clausetwo_test_not_met_when_another_agent_issued_the_taking_on_its_behalf() {
    support::assert_case(
        "test_checker.ClauseTwo.test_not_met_when_another_agent_issued_the_taking_on_its_behalf",
    );
}

#[test]
fn test_checker_clausetwo_test_not_met_when_another_agent_took_it() {
    support::assert_case("test_checker.ClauseTwo.test_not_met_when_another_agent_took_it");
}

#[test]
fn test_checker_clausetwo_test_not_met_when_nothing_was_posted() {
    support::assert_case("test_checker.ClauseTwo.test_not_met_when_nothing_was_posted");
}

#[test]
fn test_checker_clausetwo_test_not_met_when_the_assignment_was_posted_and_never_taken() {
    support::assert_case(
        "test_checker.ClauseTwo.test_not_met_when_the_assignment_was_posted_and_never_taken",
    );
}

#[test]
fn test_checker_clausetwo_test_not_met_when_the_taking_came_before_the_posting() {
    support::assert_case(
        "test_checker.ClauseTwo.test_not_met_when_the_taking_came_before_the_posting",
    );
}

#[test]
fn test_checker_clausetwo_test_not_met_when_the_taking_names_an_agent_the_log_never_spawned() {
    support::assert_case(
        "test_checker.ClauseTwo.test_not_met_when_the_taking_names_an_agent_the_log_never_spawned",
    );
}

#[test]
fn test_checker_thecommandline_test_a_swarm_that_meets_everything_exits_zero() {
    support::assert_case(
        "test_checker.TheCommandLine.test_a_swarm_that_meets_everything_exits_zero",
    );
}

#[test]
fn test_checker_thecommandline_test_a_swarm_that_misses_one_clause_exits_one_and_names_the_number()
{
    support::assert_case(
        "test_checker.TheCommandLine.test_a_swarm_that_misses_one_clause_exits_one_and_names_the_number",
    );
}

#[test]
fn test_checker_thecommandline_test_an_absent_swarm_exits_one_rather_than_raising() {
    support::assert_case(
        "test_checker.TheCommandLine.test_an_absent_swarm_exits_one_rather_than_raising",
    );
}

#[test]
fn test_checker_thecommandline_test_every_clause_number_is_in_the_output() {
    support::assert_case("test_checker.TheCommandLine.test_every_clause_number_is_in_the_output");
}

#[test]
fn test_checker_thecommandline_test_json_output_carries_every_clause() {
    support::assert_case("test_checker.TheCommandLine.test_json_output_carries_every_clause");
}

#[test]
fn test_checker_theshapeofthereport_test_a_not_met_clause_says_what_it_looked_for_and_what_it_found()
 {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_a_not_met_clause_says_what_it_looked_for_and_what_it_found",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_a_swarm_that_meets_everything_meets_all_seven() {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_each_clause_can_be_taken_the_other_way_on_its_own() {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_each_clause_can_be_taken_the_other_way_on_its_own",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_every_clause_is_reported_even_when_there_is_no_event_log()
{
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_every_clause_is_reported_even_when_there_is_no_event_log",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_every_clause_is_reported_when_the_swarm_directory_is_absent()
 {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_every_clause_is_reported_when_the_swarm_directory_is_absent",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_no_clause_is_met_by_a_name_the_log_never_spawned() {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_no_clause_is_met_by_a_name_the_log_never_spawned",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_no_clause_is_met_by_a_tally_instead_of_a_record() {
    support::assert_case(
        "test_checker.TheShapeOfTheReport.test_no_clause_is_met_by_a_tally_instead_of_a_record",
    );
}

#[test]
fn test_checker_theshapeofthereport_test_nothing_passes_by_finding_nothing() {
    support::assert_case("test_checker.TheShapeOfTheReport.test_nothing_passes_by_finding_nothing");
}

#[test]
fn test_checker_theunattendedcondition_test_a_swarm_that_never_started_is_not_reported_as_unattended()
 {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_a_swarm_that_never_started_is_not_reported_as_unattended",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_it_still_does_not_decide_the_exit_status() {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_it_still_does_not_decide_the_exit_status",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_met_when_every_command_in_the_window_came_from_the_loop()
 {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_met_when_every_command_in_the_window_came_from_the_loop",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_not_met_when_an_issuer_names_an_agent_the_log_never_spawned()
 {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_not_met_when_an_issuer_names_an_agent_the_log_never_spawned",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_not_met_when_an_operator_reached_into_the_window() {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_not_met_when_an_operator_reached_into_the_window",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_not_met_when_the_log_predates_issuers_and_it_says_so() {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_not_met_when_the_log_predates_issuers_and_it_says_so",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_the_actor_column_no_longer_decides_it() {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_the_actor_column_no_longer_decides_it",
    );
}

#[test]
fn test_checker_theunattendedcondition_test_the_command_that_starts_a_swarm_does_not_count_against_it()
 {
    support::assert_case(
        "test_checker.TheUnattendedCondition.test_the_command_that_starts_a_swarm_does_not_count_against_it",
    );
}

#!/usr/bin/env python3
"""The checker, exercised on records no swarm produced.

Run it:

    python3 -m unittest discover -s examples/two-agents

Every clause is exercised BOTH ways. A checker that answered "met" to everything fails the negative
half; a checker that answered "not met" to everything fails the positive half; and a checker that
skipped a clause it could not evaluate fails `test_every_clause_is_reported_...`, because a skipped
clause is not a clause.

`AssignmentTaken` has fired zero times in this repository's history, so "no rows" is the default
state of the world and no clause may pass by finding nothing. That is not a rule kept by hand here:
`test_nothing_passes_by_finding_nothing` walks every clause the checker reports over an empty log
and refuses any that says met.

Temporary directories come from `tempfile`, which honours `TMPDIR`. Nothing is written to the
repository and nothing is written to a path this file names.
"""

from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import textwrap
import sys
import tempfile
import unittest
from pathlib import Path

import fixtures
from fixtures import BUILDER, COORDINATOR

HERE = Path(__file__).resolve().parent
CHECKER = HERE / "check-two-agents.py"
STORE = HERE.parents[1] / "src" / "runtime" / "ess-runtime" / "src" / "store.rs"

# The checker is `check-two-agents.py`, matching `src/core/bin/check-sets-are-emitted.py` — a name
# no `import` statement can spell. It is registered in `sys.modules` BEFORE it is executed because
# `@dataclass` resolves `sys.modules[cls.__module__]` while the class body is being processed, and
# a module absent from there fails with `'NoneType' object has no attribute '__dict__'`.
_spec = importlib.util.spec_from_file_location("check_two_agents", CHECKER)
checker = importlib.util.module_from_spec(_spec)
sys.modules["check_two_agents"] = checker
_spec.loader.exec_module(checker)


class Case(unittest.TestCase):
    """A temporary directory per case, and the two readings of a fixture swarm."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="two-agents-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "swarm"

    def swarm(self, **overrides) -> Path:
        return fixtures.build(self.root, **overrides)

    def clauses(self, **overrides) -> dict:
        """The seven, by number, for a fixture built with these overrides."""
        report = checker.check(self.swarm(**overrides))
        return {clause.number: clause for clause in report.clauses}

    def report(self, **overrides):
        return checker.check(self.swarm(**overrides))

    def assertMet(self, clause, met: bool) -> None:
        self.assertEqual(
            clause.met,
            met,
            f"clause {clause.number} ({clause.title}) said {'met' if clause.met else 'not met'}; "
            f"looked for: {clause.looked_for}; found: {clause.found}",
        )


class TheShapeOfTheReport(Case):
    def test_a_swarm_that_meets_everything_meets_all_seven(self) -> None:
        report = self.report()
        self.assertEqual([clause.number for clause in report.clauses], [1, 2, 3, 4, 5, 6, 7])
        for clause in report.clauses:
            self.assertMet(clause, True)
        self.assertTrue(report.ok)
        # Reported beside the seven, and it decides nothing — but it is a verdict, not a shrug.
        self.assertMet(report.unattended, True)

    def test_each_clause_can_be_taken_the_other_way_on_its_own(self) -> None:
        """One override per clause, each unmeeting its own and leaving the rest standing.

        This is the case that catches a clause wired to the wrong evidence: unmeeting clause 5
        must not move clause 4. The one pair that genuinely cannot be separated is named in
        `fixtures.COUPLED` and asserted here rather than excused in prose.
        """
        for number, overrides in sorted(fixtures.UNMET.items()):
            with self.subTest(clause=number):
                clauses = self.clauses(**overrides)
                expected_unmet = {number} | fixtures.COUPLED.get(number, set())
                for other, clause in sorted(clauses.items()):
                    self.assertMet(clause, other not in expected_unmet)

    def test_every_clause_is_reported_even_when_there_is_no_event_log(self) -> None:
        """A clause the checker cannot evaluate is not met, never absent."""
        report = checker.check(self.swarm(eventlog=False))
        self.assertEqual([clause.number for clause in report.clauses], [1, 2, 3, 4, 5, 6, 7])
        for clause in report.clauses:
            if clause.number in (1, 2, 4, 7):
                self.assertMet(clause, False)
        self.assertFalse(report.ok)

    def test_every_clause_is_reported_when_the_swarm_directory_is_absent(self) -> None:
        report = checker.check(self.root / "nothing-here")
        self.assertEqual([clause.number for clause in report.clauses], [1, 2, 3, 4, 5, 6, 7])
        for clause in report.clauses:
            self.assertMet(clause, False)
        self.assertFalse(report.ok)

    def test_nothing_passes_by_finding_nothing(self) -> None:
        """An empty log and an empty `turns/` is the default state of the world.

        Enumerated over every clause the checker reports rather than asserted clause by clause, so
        an eighth clause added later cannot quietly pass on an empty swarm.
        """
        (self.root / "turns").mkdir(parents=True)
        fixtures.EventLog(self.root / "eventlog.sqlite3").close()
        report = checker.check(self.root)
        for clause in list(report.clauses) + [report.unattended]:
            self.assertMet(clause, False)
        self.assertFalse(report.ok)

    def test_a_not_met_clause_says_what_it_looked_for_and_what_it_found(self) -> None:
        clause = self.clauses(finished=None)[4]
        self.assertFalse(clause.met)
        self.assertTrue(clause.looked_for.strip())
        self.assertTrue(clause.found.strip())

    def test_no_clause_is_met_by_a_name_the_log_never_spawned(self) -> None:
        """The class two findings of correction round 1 belong to: a name is not an agent.

        `agent_of`'s record path took any string a turn file gave itself, and clause 7 counted a
        string on a spend row as "an agent the swarm had to bound" while clause 6 refused the same
        string as unspawned. Both are instances of one rule — **only an `AgentSpawned` makes an
        agent** — so this walks every clause over a swarm whose records are complete and whose
        every name is invented, rather than asserting it clause by clause where the next one
        added would be missed.
        """
        report = checker.check(
            self.swarm(
                roles=(),
                posted_to="alice",
                taken_by="alice",
                turns=((1, "alice"), (2, "bob")),
                spend=[(1, "alice"), (2, "bob")],
                attribution="record",
                ceiling="capped-file",
            )
        )
        # Clause 5 is the one clause whose predicate names no agent — a tool call refused by the
        # frame is refused whoever ran the turn — so it is met here and that is correct. The
        # partition is asserted rather than assumed: an eighth clause has to be put on one side.
        about_agents = {1, 2, 3, 4, 6, 7}
        about_tool_calls = {5}
        self.assertEqual(
            about_agents | about_tool_calls, {clause.number for clause in report.clauses}
        )
        for clause in report.clauses:
            if clause.number in about_agents:
                self.assertMet(clause, False)

    def test_no_clause_is_met_by_a_tally_instead_of_a_record(self) -> None:
        """The class F5 belongs to: a summary statistic is not an instance of the thing.

        A census counting decisions cannot say the frame denied anything, and no clause may be
        met by one. Built by rewriting the fixture's census to claim denials no `tool.decided`
        record names — which is the shape a real `session.ended` can genuinely have.
        """
        root = self.swarm(refusal=None)
        for path in sorted((root / "turns").glob("*.jsonl")):
            if path.name in ("spend.jsonl", "capped.jsonl"):
                continue
            rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
            for row in rows:
                if row.get("event") == "session.ended":
                    row["census"] = {
                        "allowed": 1,
                        "denied": 3,
                        "replaced": 0,
                        "abstained": 0,
                        "by_seam": {"hook": 4},
                        "by_decider": {"frame": 4},
                    }
            path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        clause = {c.number: c for c in checker.check(root).clauses}[5]
        self.assertMet(clause, False)
        self.assertIn("claim", clause.found)


class ClauseOne(Case):
    def test_met_when_an_agent_was_spawned_with_a_role_that_is_not_coordinator(self) -> None:
        clause = self.clauses()[1]
        self.assertMet(clause, True)
        self.assertIn(BUILDER, clause.found)

    def test_not_met_when_every_spawned_agent_is_a_coordinator(self) -> None:
        clause = self.clauses(roles=((COORDINATOR, "Coordinator"), (BUILDER, "Coordinator")))[1]
        self.assertMet(clause, False)
        self.assertIn("Coordinator", clause.found)

    def test_not_met_when_nothing_was_spawned(self) -> None:
        self.assertMet(self.clauses(roles=())[1], False)


class ClauseTwo(Case):
    def test_met_when_the_agent_it_was_posted_to_took_it(self) -> None:
        clause = self.clauses()[2]
        self.assertMet(clause, True)
        self.assertIn(BUILDER, clause.found)

    def test_not_met_when_the_assignment_was_posted_and_never_taken(self) -> None:
        clause = self.clauses(taken_by=None)[2]
        self.assertMet(clause, False)
        self.assertIn("AssignmentTaken", clause.found + clause.looked_for)

    def test_not_met_when_another_agent_took_it(self) -> None:
        self.assertMet(self.clauses(taken_by=COORDINATOR)[2], False)

    def test_not_met_when_the_taking_came_before_the_posting(self) -> None:
        self.assertMet(self.clauses(take_before_post=True)[2], False)

    def test_not_met_when_nothing_was_posted(self) -> None:
        self.assertMet(self.clauses(posted_to=None)[2], False)

    def test_not_met_when_an_operator_issued_the_taking(self) -> None:
        """The curl the story is about: a `TakeAssignment` posted by hand, naming the member.

        The payload is identical to the member's own — `agent_id` is caller-supplied input — so
        before `story:an-event-cannot-say-which-agent-acted` this was the same record twice and
        the clause said so in its own `looked_for`. It is not the same record any more.
        """
        clause = self.clauses(taken_by_issuer=fixtures.OPERATOR)[2]
        self.assertMet(clause, False)
        self.assertIn("operator", clause.found)

    def test_not_met_when_another_agent_issued_the_taking_on_its_behalf(self) -> None:
        """The other half of the same finding: the coordinator taking the member's work for it.

        `swarm.agent.Worker` is what both record in the `actor` column, so the actor-type check
        cannot separate them and is not asked to.
        """
        clause = self.clauses(taken_by_issuer=fixtures.by(COORDINATOR))[2]
        self.assertMet(clause, False)
        self.assertIn(COORDINATOR, clause.found)

    def test_met_when_the_member_itself_issued_the_taking(self) -> None:
        self.assertMet(self.clauses(taken_by_issuer=fixtures.by(BUILDER))[2], True)

    def test_met_when_the_runtime_issued_the_taking(self) -> None:
        """`trigger.rs:374` issues `TakeAssignment`, not the member — so `runtime` is the shape a
        real two-agent run leaves, and a clause that refused it would be tightened onto something
        this system does not do."""
        self.assertMet(self.clauses(taken_by_issuer=fixtures.RUNTIME)[2], True)

    def test_not_met_when_an_EARLIER_handover_was_the_forged_one(self) -> None:
        """Every handover's taking is read, not only the last one.

        The first pass read `pairs[-1]` alone, so a `curl` forging a taking and the real one
        landing afterwards left the clause met and the forgery unmentioned — a reader of the
        report would never learn a hand had been on it. Correction round 1, finding 6.

        Two postings and two takings of the same assignment: the first taking is an operator's,
        the second is the member's own.
        """
        clause = self.clauses(forged_take_first=True)[2]
        self.assertMet(clause, False)
        self.assertIn("operator", clause.found)

    def test_not_met_when_the_taking_names_an_agent_the_log_never_spawned(self) -> None:
        """A claim is a claim, and `swarm.agent.AgentSpawned` is what settles it.

        Correction round 2, finding J2. `agent:<anything>` used to be read as an agent, so a curl
        adding one field to its body — `{"agent": "ghost"}` — issued a taking that the clause read
        as the member's own. The story excludes AUTHENTICATION, which is proving a claim against a
        credential; it does not exclude checking a claim against the log's own rows, and this file
        has said since 2026-09-13 that "an agent is what an `AgentSpawned` says it is, and nothing
        else says it". The issuer column was the one place that was not obeying it.
        """
        clause = self.clauses(taken_by_issuer=fixtures.by("ghost"))[2]
        self.assertMet(clause, False)
        self.assertIn("ghost", clause.found)

    def test_it_says_it_is_coping_with_a_log_written_before_issuers(self) -> None:
        """A log from before this closed carries no issuer at all, and the clause says so.

        Eleven such logs are on disk under `data/swarms/`, one of them the demonstration's own.
        The answer is not to fail them — the record still says the member took it, which is what
        the clause always checked — and not to stay silent, which would read as a check that was
        made. It is to report the limit in the clause's own output.
        """
        clause = self.clauses(issuers=False)[2]
        self.assertMet(clause, True)
        self.assertIn("before", clause.found)
        self.assertNotIn("cannot establish who issued", clause.looked_for)


class ClauseThree(Case):
    def test_met_when_two_turn_files_are_attributed_to_two_agents(self) -> None:
        clause = self.clauses()[3]
        self.assertMet(clause, True)
        self.assertIn(BUILDER, clause.found)
        self.assertIn(COORDINATOR, clause.found)

    def test_met_by_each_way_a_turn_file_names_its_agent(self) -> None:
        for attribution in ("spend", "record", "cwd", "name"):
            with self.subTest(attribution=attribution):
                # `build` clears the root first, so each pass writes a whole swarm of its own.
                self.assertMet(self.clauses(attribution=attribution)[3], True)

    def test_not_met_with_one_transcript(self) -> None:
        clause = self.clauses(turns=((1, COORDINATOR),), spend=[(1, COORDINATOR), (2, BUILDER)])[3]
        self.assertMet(clause, False)

    def test_not_met_when_both_transcripts_are_the_same_agent(self) -> None:
        clause = self.clauses(turns=((1, COORDINATOR), (2, COORDINATOR)))[3]
        self.assertMet(clause, False)
        self.assertIn("1 distinct agent", clause.found)

    def test_not_met_when_an_attempt_left_no_file_beside_it(self) -> None:
        """Two attempts at one turn and one file is a record that was overwritten.

        `coordinator.rs`'s `turn_file` puts the attempt in the name for exactly this reason: on
        2026-09-12 a first attempt's whole transcript was replaced by the attempt that followed.
        """
        clause = self.clauses(extra_attempts=((1, COORDINATOR),))[3]
        self.assertMet(clause, False)
        self.assertIn("overwritten", (clause.found + clause.looked_for).lower())

    def test_not_met_when_no_turn_file_can_be_attributed(self) -> None:
        self.assertMet(self.clauses(spend=[], attribution="spend")[3], False)


class ClauseFour(Case):
    def test_met_by_a_finished_assignment(self) -> None:
        self.assertMet(self.clauses()[4], True)

    def test_met_by_a_green_gate(self) -> None:
        clause = self.clauses(finished="gate")[4]
        self.assertMet(clause, True)
        self.assertIn("GateGreen", clause.found)

    def test_not_met_when_the_work_was_only_started(self) -> None:
        clause = self.clauses(finished=None)[4]
        self.assertMet(clause, False)
        self.assertIn("GateGreen", clause.looked_for)

    def test_not_met_when_the_coordinator_reports_a_green_gate_on_the_builder_s_work(self) -> None:
        """"Finished" is finished BY THE AGENT THAT TOOK IT.

        A one-coordinator swarm reporting its own gate green is the picture this story exists to
        disprove, and the old rule — any `GateGreen` anywhere in the log — accepted it.
        """
        clause = self.clauses(finished="gate", finished_by=COORDINATOR)[4]
        self.assertMet(clause, False)
        self.assertIn(BUILDER, clause.found)

    def test_not_met_when_nobody_took_the_work_there_is_no_finisher_to_check(self) -> None:
        clause = self.clauses(taken_by=None)[4]
        self.assertMet(clause, False)
        self.assertIn("clause 2", clause.found)


class ClauseFive(Case):
    def test_met_when_the_frame_refused_a_tool_call(self) -> None:
        clause = self.clauses()[5]
        self.assertMet(clause, True)
        self.assertIn("Bash", clause.found)

    def test_not_met_when_every_call_was_allowed(self) -> None:
        clause = self.clauses(refusal=None)[5]
        self.assertMet(clause, False)

    def test_not_met_when_the_denial_did_not_come_from_the_frame(self) -> None:
        """A refusal that happened, not a flag that was set — and not somebody else's refusal."""
        clause = self.clauses(refusal="user")[5]
        self.assertMet(clause, False)
        self.assertIn("user", clause.found)

    def test_not_met_when_there_are_no_turn_records_at_all(self) -> None:
        self.assertMet(self.clauses(turns=())[5], False)


class ClauseSix(Case):
    def test_met_when_two_agents_each_name_themselves(self) -> None:
        clause = self.clauses()[6]
        self.assertMet(clause, True)
        self.assertIn(BUILDER, clause.found)

    def test_not_met_when_only_one_agent_is_named(self) -> None:
        self.assertMet(self.clauses(spend=[(1, COORDINATOR)], attribution="record")[6], False)

    def test_not_met_when_the_rows_name_nobody(self) -> None:
        clause = self.clauses(spend=[], unattributed=2, attribution="record")[6]
        self.assertMet(clause, False)
        self.assertIn("2", clause.found)

    def test_not_met_when_there_is_no_spend_record(self) -> None:
        self.assertMet(self.clauses(spend=[], attribution="record")[6], False)

    def test_not_met_when_the_second_named_agent_was_never_spawned(self) -> None:
        """A figure a ceiling is written against must name who ran it up, and a name the log
        never spawned names nobody."""
        clause = self.clauses(spend=[(1, COORDINATOR), (2, "ghost")], attribution="record")[6]
        self.assertMet(clause, False)
        self.assertIn("ghost", clause.found)


class ClauseSeven(Case):
    def test_met_when_the_agent_ceiling_refused_an_agent(self) -> None:
        clause = self.clauses()[7]
        self.assertMet(clause, True)
        self.assertIn(BUILDER, clause.found)

    def test_met_when_the_refusal_is_recorded_beside_the_transcripts(self) -> None:
        self.assertMet(self.clauses(ceiling="capped-file")[7], True)

    def test_met_when_the_refusal_is_recorded_on_the_spend_row(self) -> None:
        self.assertMet(self.clauses(ceiling="spend-note")[7], True)

    def test_not_met_when_nothing_was_refused(self) -> None:
        clause = self.clauses(ceiling=None)[7]
        self.assertMet(clause, False)

    def test_not_met_when_there_was_only_one_agent_to_bound(self) -> None:
        """Per-agent recording and per-agent enforcement fail independently — and a ceiling that
        refuses the only agent there is has not been shown to bound a swarm of two."""
        clause = self.clauses(
            roles=((COORDINATOR, "Coordinator"),),
            posted_to=None,
            taken_by=None,
            turns=((1, COORDINATOR),),
            spend=[(1, COORDINATOR)],
        )[7]
        self.assertMet(clause, False)


class TheUnattendedCondition(Case):
    """It is a verdict now, and it can go both ways.

    It has been wrong twice, in opposite directions, and both are asserted here.

    **It could not fail.** The first version read a window of machine actors as unattended.
    `store.rs:192` wrote `actor.unwrap_or("system")` and `http.rs:38` made the actor optional, so
    an operator's command and the loop's own were recorded identically — and `dsfsdf`, the swarm
    the story names as hand-driven, passed it with 26 operator commands in its window.

    **Then it could not pass.** The answer was to report UNDETERMINABLE and decide nothing, which
    was honest while the record could not carry the fact and is dishonest now that it can:
    `story:an-event-cannot-say-which-agent-acted` put the issuer on the envelope, so a log written
    since says which commands a hand sent and which the loop did.

    So the rule is: the condition needs POSITIVE evidence. Every event in the window names an
    issuer, and none of them is an operator. A log that cannot carry the fact cannot supply the
    evidence, and is not met — which is the answer `data/swarms/two-agents-proof` gets, and the
    right one: it was driven by an operator's `SetGoal` and `StartSwarm`.
    """

    def test_met_when_every_command_in_the_window_came_from_the_loop(self) -> None:
        report = self.report()
        self.assertMet(report.unattended, True)
        self.assertIn(fixtures.RUNTIME, report.unattended.found)

    def test_not_met_when_an_operator_reached_into_the_window(self) -> None:
        report = self.report(operator_in_window=True)
        self.assertMet(report.unattended, False)
        self.assertIn("operator", report.unattended.found)

    def test_not_met_when_an_issuer_names_an_agent_the_log_never_spawned(self) -> None:
        """The same rule, on the condition that makes the positive claim.

        Correction round 2, finding J2. `unattended` asked only whether an issuer was the literal
        word `operator`, so a hand that put any other string in the `agent` field of its request
        was not a hand — one field, and the condition it defeats is the one the whole story exists
        to make checkable. A name the log never spawned is not evidence that nobody touched the
        swarm; it is a claim with nothing behind it.
        """
        report = self.report(operator_in_window=True, operator_claims="ghost")
        self.assertMet(report.unattended, False)
        self.assertIn("ghost", report.unattended.found)

    def test_the_actor_column_no_longer_decides_it(self) -> None:
        """An actor type the specification declares a person, on a row the loop issued.

        Renamed in correction round 1, finding 7: it used to be called
        `test_not_met_when_a_person_is_visible_in_the_actor_column_either` and it was green
        because its fixture set an OPERATOR ISSUER, not because of the actor column —
        `unattended` never reads `actor` once issuers are present. A case green for a reason
        other than its name is a check nobody can rely on, so it now asserts the property that
        is actually there: the actor type is the specification's question and does not decide
        whose hands were on the swarm. `swarm.goal.Operator` in the window with every issuer the
        loop's is unattended, and `test_not_met_when_an_operator_reached_into_the_window` is
        where a hand is caught.
        """
        report = self.report(human_actor="swarm.goal.Operator")
        self.assertIn(
            "swarm.goal.Operator",
            {event.actor for event in checker.read(self.root).events},
        )
        self.assertMet(report.unattended, True)

    def test_not_met_when_the_log_predates_issuers_and_it_says_so(self) -> None:
        """The eleven logs already on disk. The absence of an operator cannot be read off a record
        that never recorded the presence of one, and saying "unattended" there would be the first
        defect again, exactly."""
        report = self.report(issuers=False)
        self.assertMet(report.unattended, False)
        self.assertIn("before", report.unattended.found)

    def test_the_command_that_starts_a_swarm_does_not_count_against_it(self) -> None:
        """An operator starts a swarm. Counting the `SwarmStarted` that opens the window would
        make the condition unmeetable by construction, which is the mirror of the defect where it
        could not fail — and the story's word is "between the swarm starting and a verdict"."""
        report = self.report()
        started = [event for event in checker.read(self.root).events
                   if event.name == "swarm.manager.SwarmStarted"]
        self.assertEqual(len(started), 1)
        self.assertEqual(started[0].issuer, fixtures.OPERATOR)
        self.assertMet(report.unattended, True)

    def test_it_still_does_not_decide_the_exit_status(self) -> None:
        """Seven clauses is what the demonstration story's acceptance has, and unattended is not
        one of them — it is a condition reported beside them. What changed is that it now says
        met or not met; what did not is whose question it answers."""
        report = self.report(operator_in_window=True)
        self.assertMet(report.unattended, False)
        self.assertTrue(all(clause.met for clause in report.clauses))
        self.assertTrue(report.ok)

    def test_a_swarm_that_never_started_is_not_reported_as_unattended(self) -> None:
        self.assertMet(self.report(started=False).unattended, False)


class TheIssuerVocabulary(unittest.TestCase):
    """The values `subject` can hold are `ess_runtime::Issuer`'s, derived from its VARIANTS.

    A hand-kept copy of another language's vocabulary is the defect and a stale copy is only its
    symptom — so this does not list them, it derives them. What makes the derivation worth having
    is HOW it would go wrong otherwise: rename `runtime` on the Rust side and every row of every
    log reads as one written before issuers existed, so clause 2 reports it is coping and
    `unattended` says NOT MET forever, and no test is red. A silent downgrade to "I cannot tell"
    is the exact failure the whole story was filed to end.

    **The enumeration starts at the variants, not at `label`'s arms**, and that is correction
    round 1, finding 2. It used to walk the arms with a regex ending in `,\n`, and `rustfmt`
    writes any arm too long for a line as a BLOCK ending in `}` — so a fifth variant whose label
    is computed rather than returned was not an unreadable arm, it was no arm at all, and the
    derivation reported the same four labels it always had. The guard whose load-bearing half is
    catching a fifth variant walked straight past one.

    Starting at the variants turns that silence into a name: a variant with no readable label is a
    MISSING label, reported as that variant. The failure being guarded against is silent, so the
    guard has to be loud, and a parser that fails open is not a guard.
    """

    def issuer_variants(self, source: str) -> list:
        """Every variant of `enum Issuer`, in order — all THREE shapes a Rust variant has.

        A unit variant ends in `,`, a tuple variant opens a `(`, and a struct variant opens a `{`
        after a space. The first version of this read the first two, so a struct variant was not a
        variant it had ever heard of — and this guard's own docstring says a parser that fails
        open is not a guard. It failed open at the one step where failing open is invisible: a
        variant it does not see is not a missing label, it is no variant, and the derivation then
        agrees with the checker about a vocabulary that is short by one.

        Correction round 2, finding F3. Nothing found reaches it — `Issuer` has no struct variant
        — and it is fixed anyway, because "nothing found" is the state this whole guard exists to
        stop being reported as "nothing wrong".
        """
        enum = re.search(r"pub enum Issuer \{(.*?)\n\}", source, re.S)
        self.assertIsNotNone(enum, f"`enum Issuer` is where the vocabulary starts, in {STORE}")
        variants = re.findall(r"^    (\w+)\s*[,({]", enum.group(1), re.M)
        self.assertTrue(variants, f"`enum Issuer` has variants, in {STORE}")
        return variants

    def literal_of(self, variant: str, arm: str, consts: dict) -> str:
        """The fixed part of what one arm writes into the column.

        Two of the four are whole values and two are PREFIXES — `agent:` before a slug, `agent:?`
        before a digest — so what is derived is the leading literal either way, which is what the
        checker matches on.
        """
        arm = arm.strip().rstrip(",")
        if literal := re.fullmatch(r'"([^"]*)"\.to_owned\(\)', arm):
            return literal.group(1)
        if named := re.fullmatch(r"(\w+)\.to_owned\(\)", arm):
            return consts[named.group(1)]
        if formatted := re.match(r'format!\("\{(\w+)\}', arm):
            return consts[formatted.group(1)]
        self.fail(
            f"`Issuer::{variant}` has a label this checker cannot read: {arm!r}. A value the log "
            "can hold and the checker cannot name is a row it will report as written before "
            "issuers existed, silently — which is the answer this whole story exists to stop the "
            "checker giving"
        )
        raise AssertionError  # unreachable; `fail` raises. Here so the return type is honest.

    def labels_from_the_runtime(self) -> set:
        return self.labels_from(STORE.read_text())

    def labels_from(self, source: str) -> set:
        consts = dict(re.findall(r'^const (\w+): &str = "([^"]*)";', source, re.M))
        body = re.search(r"pub fn label\(&self\) -> String \{(.*?)\n    \}", source, re.S)
        self.assertIsNotNone(body, f"`Issuer::label` is where the vocabulary is, in {STORE}")

        labels = set()
        for variant in self.issuer_variants(source):
            arm = re.search(
                rf"Self::{variant}\b(?:\([^)]*\)|\s*\{{[^}}]*\}})?\s*=>\s*(.+)",
                body.group(1),
            )
            if arm is None:
                self.fail(
                    f"`Issuer::{variant}` has no arm this checker can find in `Issuer::label`. "
                    "Every variant is a value the log can hold, and one the checker cannot name "
                    "reads as a row written before issuers existed, silently"
                )
            labels.add(self.literal_of(variant, arm.group(1), consts))
        return labels

    def test_the_checker_names_every_value_the_runtime_can_write(self) -> None:
        self.assertEqual(
            self.labels_from_the_runtime(),
            {checker.RUNTIME, checker.OPERATOR, checker.AGENT, checker.UNNAMEABLE_AGENT},
        )

    #: An `Issuer` with a fifth variant whose label is COMPUTED, written here rather than patched
    #: into the real `store.rs`. `rustfmt` writes any arm too long for one line as a block, which
    #: is exactly the shape the first version of this guard walked past in silence.
    #:
    #: Synthetic on purpose. A mutation made by string-replacing the real source is anchored on
    #: the source as it stands, so it stops testing anything the moment that source legitimately
    #: changes — which is what happened to the adversary's own copy of this case in correction
    #: round 1: it anchored on `    UnnameableAgent,\n}`, and the fix for finding 1 gave that
    #: variant a field. A guard that rots when the code it guards is corrected is not a guard.
    A_FIFTH_VARIANT = textwrap.dedent("""\
        pub enum Issuer {
            Runtime,
            Scheduler,
        }

        const AGENT: &str = "agent:";

        impl Issuer {
            pub fn label(&self) -> String {
                match self {
                    Self::Runtime => "runtime".to_owned(),
                    Self::Scheduler => {
                        let mut said = String::from("sched");
                        said.push_str("uler");
                        said
                    }
                }
            }
        }
        """)

    def test_a_variant_whose_label_is_computed_is_a_missing_label_and_not_a_silence(self) -> None:
        """The load-bearing half: a fifth `Issuer` variant fails here, loudly, by name.

        What it costs to get this wrong is silent. A fifth variant is a fifth value in the log's
        `subject` column; the checker names four, so an event carrying the fifth returns `None`
        from `issued_by` and reads as a row written before issuers existed. Clause 2 would report
        it was coping and `unattended` would say NOT MET, forever, and nothing would be red — the
        downgrade to "I cannot tell" this whole story exists to end.

        So the derivation starts at the VARIANTS. A variant whose label cannot be read is a
        missing label, reported as that variant; it is never an arm that simply was not found.
        """
        with self.assertRaises(self.failureException) as caught:
            self.labels_from(self.A_FIFTH_VARIANT)
        self.assertIn("Scheduler", str(caught.exception))

    def test_a_variant_with_no_label_arm_at_all_is_caught_by_name(self) -> None:
        """The other way a variant goes unread: no arm this parser can find for it."""
        without = self.A_FIFTH_VARIANT.replace(
            "            Self::Scheduler => {\n"
            '                let mut said = String::from("sched");\n'
            '                said.push_str("uler");\n'
            "                said\n"
            "            }\n",
            "",
        )
        self.assertNotIn("Self::Scheduler", without, "the arm is gone from the copy")
        with self.assertRaises(self.failureException) as caught:
            self.labels_from(without)
        self.assertIn("Scheduler", str(caught.exception))

    def test_the_fixture_speaks_the_same_vocabulary(self) -> None:
        """`fixtures.py` writes the rows these cases are read from, so a drift there would make
        every case above green against a log the runtime does not produce."""
        self.assertEqual(fixtures.RUNTIME, checker.RUNTIME)
        self.assertEqual(fixtures.OPERATOR, checker.OPERATOR)
        self.assertEqual(fixtures.by("x"), checker.AGENT + "x")


class TheCommandLine(Case):
    def run_checker(self, *args) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(CHECKER), *args],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_a_swarm_that_meets_everything_exits_zero(self) -> None:
        done = self.run_checker(str(self.swarm()))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def test_a_swarm_that_misses_one_clause_exits_one_and_names_the_number(self) -> None:
        done = self.run_checker(str(self.swarm(finished=None)))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn("clause 4", done.stdout)

    def test_every_clause_number_is_in_the_output(self) -> None:
        done = self.run_checker(str(self.swarm(finished=None)))
        for number in range(1, 8):
            self.assertIn(f"clause {number}", done.stdout)

    def test_json_output_carries_every_clause(self) -> None:
        done = self.run_checker(str(self.swarm()), "--json")
        payload = json.loads(done.stdout)
        self.assertEqual([clause["number"] for clause in payload["clauses"]], [1, 2, 3, 4, 5, 6, 7])
        self.assertTrue(all(clause["met"] for clause in payload["clauses"]))

    def test_an_absent_swarm_exits_one_rather_than_raising(self) -> None:
        done = self.run_checker(str(self.root / "nothing-here"))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn("clause 1", done.stdout)


if __name__ == "__main__":
    unittest.main()

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
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import fixtures
from fixtures import BUILDER, COORDINATOR

HERE = Path(__file__).resolve().parent
CHECKER = HERE / "check-two-agents.py"

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
        # Reported beside the seven, never met, and it decides nothing.
        self.assertFalse(report.unattended.determinable)
        self.assertMet(report.unattended, False)

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

    def test_it_says_out_loud_what_it_cannot_establish(self) -> None:
        """The record says the agent took it. Who issued the command is not in this log, and a
        clause that implies otherwise is claiming more than it checked."""
        self.assertIn("cannot establish who issued", self.clauses()[2].looked_for)


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
    """It is undeterminable, and the checker says so instead of guessing.

    The first version of these cases asserted that a window of machine actors meant the run was
    unattended. That rule could not fail: `store.rs:192` writes `actor.unwrap_or("system")` and
    `http.rs:38` makes the actor optional, so an operator's command and the loop's own are
    recorded identically — and `dsfsdf`, the swarm the story names as hand-driven, passed it with
    26 operator commands in its window. So the condition is reported as undeterminable, and the
    cases below hold it to that in both directions.
    """

    def test_it_is_never_met_however_clean_the_actors_look(self) -> None:
        report = self.report()
        self.assertMet(report.unattended, False)
        self.assertFalse(report.unattended.determinable)

    def test_it_is_not_met_when_a_person_is_visible_either(self) -> None:
        self.assertMet(self.report(human_actor="timo@example.com").unattended, False)

    def test_it_says_why_it_cannot_be_read(self) -> None:
        found = self.report().unattended.found + self.report().unattended.looked_for
        self.assertIn("store.rs:192", found)
        self.assertIn("http.rs:38", found)
        self.assertIn("UNDETERMINABLE", found)

    def test_it_does_not_decide_the_exit_status(self) -> None:
        """Seven clauses met is exit 0, even with an operator's name in the log — because the
        checker has no way to tell that name from the loop's, and pretending otherwise is what
        it was doing before."""
        report = self.report(human_actor="timo@example.com")
        self.assertTrue(all(clause.met for clause in report.clauses))
        self.assertTrue(report.ok)

    def test_a_hand_driven_swarm_is_not_reported_as_unattended(self) -> None:
        self.assertMet(self.report(started=False).unattended, False)


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

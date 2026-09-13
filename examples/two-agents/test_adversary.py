#!/usr/bin/env python3
"""Adversarial cases against `check-two-agents.py`.

Written by the adversary of wave 2026-09-13a, pass 1. Every case here asserts what the story's
acceptance — or the checker's own `looked_for` sentence — says the answer should be, against
records whose shape was read off this repository's real `data/swarms/*` rather than invented.

None of these cases changes an implementation file. Where a case needs a record the fixture
builder cannot produce, it builds that record by hand, in the case.

Run one alone:

    python3 -m unittest discover -s examples/two-agents -p 'test_adversary.py'
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

import fixtures
from fixtures import BUILDER, COORDINATOR

HERE = Path(__file__).resolve().parent
CHECKER = HERE / "check-two-agents.py"

# Load the checker the way `test_checker.py` does, but do not load it twice if that module
# already did: `@dataclass` resolves `sys.modules[cls.__module__]`, so the registration has to
# happen before the module body runs, and re-running it would give two incompatible classes.
checker = sys.modules.get("check_two_agents")
if checker is None:
    _spec = importlib.util.spec_from_file_location("check_two_agents", CHECKER)
    checker = importlib.util.module_from_spec(_spec)
    sys.modules["check_two_agents"] = checker
    _spec.loader.exec_module(checker)


class Adversarial(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="two-agents-adversary-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "swarm"

    def swarm(self, **overrides) -> Path:
        return fixtures.build(self.root, **overrides)

    def clauses(self, **overrides) -> dict:
        report = checker.check(self.swarm(**overrides))
        return {clause.number: clause for clause in report.clauses}

    def assertMet(self, clause, met: bool) -> None:
        self.assertEqual(
            clause.met,
            met,
            f"clause {clause.number} ({clause.title}) said "
            f"{'met' if clause.met else 'not met'}; looked for: {clause.looked_for}; "
            f"found: {clause.found}",
        )


class TheUnattendedConditionAgainstARealLog(Adversarial):
    """The condition that decides the exit status, against the actors a real log carries.

    `src/runtime/ess-runtime/src/store.rs:192` writes `actor.unwrap_or("system")`, and
    `src/runtime/swarm-server/src/http.rs:38` makes `actor` an OPTIONAL field of the command
    request body. So an operator who pauses, resumes or stops a swarm through the HTTP surface
    leaves events whose actor is `system` — which `check-two-agents.py:712` counts as a machine.

    Measured on `data/swarms/dsfsdf/eventlog.sqlite3`, the swarm the story itself names as the
    one the operator hand-drove: inside the window from the first `SwarmStarted` there are 8
    `swarm.manager.SwarmPaused`, 4 `swarm.manager.SwarmResumed`, 7 `swarm.manager.SwarmStopped`
    and 6 further `SwarmStarted`, every one of them actor `system`.
    """

    def test_an_operator_pausing_and_resuming_by_hand_is_not_unattended(self) -> None:
        (self.root / "turns").mkdir(parents=True)
        log = fixtures.EventLog(self.root / "eventlog.sqlite3")
        log.append("swarm.manager.SwarmCreated", {"swarm_id": fixtures.SWARM_ID})
        log.append("swarm.manager.SwarmStarted", {"swarm_id": fixtures.SWARM_ID})
        # The operator's hands, recorded the way this runtime records them.
        for _ in range(8):
            log.append("swarm.manager.SwarmPaused", {"swarm_id": fixtures.SWARM_ID})
            log.append("swarm.manager.SwarmResumed", {"swarm_id": fixtures.SWARM_ID})
        log.append("swarm.manager.SwarmStopped", {"swarm_id": fixtures.SWARM_ID})
        log.close()

        report = checker.check(self.root)
        self.assertMet(report.unattended, False)


class ClauseThreeAgainstItsOwnSentence(Adversarial):
    """Clause 3's `looked_for` promises "a file for every attempt the spend record claims".

    `check-two-agents.py:419-424` counts a spend row only when `files_per_key` ALREADY holds a
    key for that (turn, goal-prefix) — that is, only when at least one file survived. An attempt
    whose turn left no file at all matches no key, hits no `break`, and is dropped.

    This is not hypothetical. `data/swarms/e2e-1789212621/turns/` holds exactly this shape:
    `spend.jsonl` claims iterations 1, 2, 3 and 4, and the only turn files are
    `0003-01-73a4ac74.jsonl` and `0004-01-73a4ac74.jsonl`. Two attempts, no transcript.
    """

    def test_not_met_when_an_attempt_left_no_transcript_at_any_turn_number(self) -> None:
        clauses = self.clauses(
            turns=((3, COORDINATOR), (4, BUILDER)),
            spend=[(1, COORDINATOR), (2, COORDINATOR), (3, COORDINATOR), (4, BUILDER)],
        )
        self.assertMet(clauses[3], False)

    def test_not_met_when_the_two_transcripts_name_agents_the_log_never_spawned(self) -> None:
        """Three of `agent_of`'s four paths check the name against the log; the first does not.

        `check-two-agents.py:379-382` returns any non-empty `agent` string a turn record carries,
        unchecked, while the file-name path (:383), the cwd path (:386) and clause 6 (:599) all
        require the log to have spawned it. Two transcripts naming two strings no
        `AgentSpawned` ever named are not "two transcripts attributed to two different agents".
        """
        clauses = self.clauses(
            roles=(),
            posted_to=None,
            taken_by=None,
            attribution="record",
            turns=((1, "alice"), (2, "bob")),
            spend=[],
        )
        self.assertMet(clauses[3], False)


class ClauseSevenAgainstClauseSix(Adversarial):
    """The checker contradicts itself about who counts as an agent, on one set of records.

    `check-two-agents.py:599` makes clause 6 require both named agents to be agents the log says
    were spawned, and `test_not_met_when_the_second_named_agent_was_never_spawned` holds it to
    that. `check-two-agents.py:190` then defines `known_agents` as spawned UNION spend-named, and
    clause 7 (:668) counts `len(known) >= 2` as "more than one agent to bound".

    So on one swarm with exactly one `AgentSpawned`, clause 6 says `ghost` is "named by no
    swarm.agent.AgentSpawned" and clause 7 says there were 2 agents to bound and the ceiling
    refused one of them. Clause 7's own `looked_for` says "at least two agents the swarm had to
    bound"; a string on a spend row is not an agent the swarm had.

    `test_not_met_when_there_was_only_one_agent_to_bound` does not catch it: it takes the second
    agent out of `roles` AND out of `spend` together, so `known_agents`' spend half is never
    exercised against a name the log does not know.
    """

    def test_not_met_when_the_second_agent_to_bound_is_only_a_name_on_a_spend_row(self) -> None:
        root = self.swarm(
            roles=((COORDINATOR, "Coordinator"),),
            posted_to=None,
            taken_by=None,
            turns=((1, COORDINATOR),),
            spend=[(1, COORDINATOR)],
            attribution="record",
            ceiling=None,
        )
        spend = root / "turns" / "spend.jsonl"
        spend.write_text(
            spend.read_text()
            + json.dumps(
                {
                    "at": "2026-09-13T12:02:30Z",
                    "agent": "ghost",
                    "goal": fixtures.GOAL_ID,
                    "iterations": 2,
                    "note": fixtures.AGENT_CEILING_SENTENCE.format(agent="ghost"),
                    "spent": {"cost_usd": 0.07},
                }
            )
            + "\n"
        )
        clauses = {c.number: c for c in checker.check(root).clauses}
        self.assertMet(clauses[1], False)  # the log spawned one agent, and it is the Coordinator
        self.assertMet(clauses[7], False)


class ClauseFiveAgainstTheRealCensus(Adversarial):
    """`by_decider` counts decisions, not denials — so `by_decider["frame"]` is not a refusal.

    Read off a real record, `data/swarms/dsfsdf/turns/0035-77fc1fcc.jsonl`:

        "census":{"allowed":5,"denied":0,"replaced":0,"abstained":0,
                  "by_seam":{"hook":5},"by_decider":{"observe":5}}

    Five ALLOWED calls, zero denied, and `by_decider` counts all five. `fixtures.py:186` writes
    `{"by_decider": {refusal: 1}}` — only the decider that denied — which is the checker's own
    idea of the schema and not the schema. Against the real one,
    `check-two-agents.py:536`'s `(census.get("by_decider") or {}).get("frame")` is true whenever
    the frame decided anything at all, so a session where the frame only ALLOWED and somebody
    else denied reports a frame refusal that never happened — "a flag that was set", which is
    the exact thing clause 5's own `looked_for` says it is not.
    """

    def test_not_met_when_the_frame_only_allowed_and_another_decider_denied(self) -> None:
        root = self.swarm(refusal="user")
        for path in sorted((root / "turns").glob("*.jsonl")):
            if path.name in ("spend.jsonl", "capped.jsonl"):
                continue
            rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
            for row in rows:
                if row.get("event") == "session.ended" and isinstance(row.get("census"), dict):
                    # The census a real metaharness session would write for these same
                    # decisions: the frame allowed one call, `user` denied one.
                    row["census"] = {
                        "allowed": 1,
                        "denied": 1,
                        "replaced": 0,
                        "abstained": 0,
                        "by_seam": {"hook": 2},
                        "by_decider": {"frame": 1, "user": 1},
                    }
            path.write_text("".join(json.dumps(row) + "\n" for row in rows))

        report = checker.check(root)
        clause = {c.number: c for c in report.clauses}[5]
        self.assertMet(clause, False)


if __name__ == "__main__":
    unittest.main()

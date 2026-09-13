#!/usr/bin/env python3
"""Adversarial cases against `check-two-agents.py`, wave 2026-09-13a, pass 2.

Pass 1 attacked the clause predicates. This pass attacks the two documents the unit wrote about
itself — `fixtures.py`, which claims to write "the records a run leaves", and `README.md`, which
tells an operator which clauses are worth spending money on — and drives them against
`src/core/domains/agent.yaml`, which neither half of this unit authored.

Every case here asserts what the domain spec, the story's acceptance or the checker's own
`looked_for` sentence says the answer should be. No implementation file is changed, and
`test_adversary.py` (pass 1) is not touched.

Run this file alone:

    python3 -m unittest discover -s examples/two-agents -p 'test_adversary_pass2.py'
"""

from __future__ import annotations

import importlib.util
import json
import re
import sqlite3
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

import fixtures
from fixtures import BUILDER, COORDINATOR

HERE = Path(__file__).resolve().parent
CHECKER = HERE / "check-two-agents.py"
REPO = HERE.parents[1]
AGENT_YAML = REPO / "src" / "core" / "domains" / "agent.yaml"
README = HERE / "README.md"

# Loaded the way `test_adversary.py` does — registered in `sys.modules` before execution, and not
# executed twice, because `@dataclass` resolves `sys.modules[cls.__module__]` as the class body
# runs.
checker = sys.modules.get("check_two_agents")
if checker is None:
    _spec = importlib.util.spec_from_file_location("check_two_agents", CHECKER)
    checker = importlib.util.module_from_spec(_spec)
    sys.modules["check_two_agents"] = checker
    _spec.loader.exec_module(checker)


def role_variants() -> set:
    """The variants of `swarm.agent.Role` — the declared type of `AgentSpawned.role`.

    Parsed out of `src/core/domains/agent.yaml` rather than restated here, so that a run of this
    case measures the spec as it stands and not a copy of it made on the day it was written.
    """
    text = AGENT_YAML.read_text()
    block = re.search(
        r"^  - name: swarm\.agent\.Role\n(?P<body>(?:    .*\n|\n)*)", text, re.M
    )
    assert block, "swarm.agent.Role is not declared in agent.yaml"
    body = block.group("body")
    variants = re.search(r"^    variants:\n(?P<items>(?:      - .*\n)+)", body, re.M)
    assert variants, "swarm.agent.Role declares no variants"
    return {line.strip()[2:].strip() for line in variants.group("items").splitlines()}


def spawned_roles(root: Path) -> set:
    """Every `role` an `AgentSpawned` in this swarm's log carries."""
    db = sqlite3.connect(f"file:{root / 'eventlog.sqlite3'}?mode=ro", uri=True)
    rows = db.execute(
        "select data from swarm_events where event_name = 'swarm.agent.AgentSpawned'"
    ).fetchall()
    db.close()
    return {json.loads(data).get("role") for (data,) in rows}


class Case(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="two-agents-adversary2-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "swarm"

    def clauses(self, root) -> dict:
        return {clause.number: clause for clause in checker.check(root).clauses}

    def assertMet(self, clause, met: bool) -> None:
        self.assertEqual(
            clause.met,
            met,
            f"clause {clause.number} ({clause.title}) said "
            f"{'met' if clause.met else 'not met'}; looked for: {clause.looked_for}; "
            f"found: {clause.found}",
        )


class TheFixtureAgainstTheDomain(Case):
    """`fixtures.py` says it writes "the records a run leaves". Held against the domain that
    defines those records, one of them is a record no run can leave."""

    def test_the_fixture_spawns_a_role_the_domain_has_no_variant_for(self) -> None:
        """`AgentSpawned.role` is a `swarm.agent.Role`, and that enum has one variant.

        `fixtures.py:205` writes `("builder", "Worker")`. `swarm.agent.Worker` exists in
        `agent.yaml` — as an ACTOR TYPE, a `may:` list — and is not a variant of the `Role` enum
        the spawn payload's `role` field is typed as. Every one of the ten real logs under
        `data/swarms/` carries `"role":"Coordinator"` on every `AgentSpawned`, `builder`,
        `reviewer` and `tester` included, and `swarm.rs:1136` hardcodes that string.

        So the fixture the whole suite rests on writes a value the type cannot hold, and clauses
        1, 2 and 4 are green only against it.
        """
        variants = role_variants()
        used = spawned_roles(fixtures.build(self.root))
        self.assertLessEqual(
            used,
            variants,
            f"fixtures.py writes AgentSpawned.role values {sorted(used - variants)} that "
            f"swarm.agent.Role has no variant for; its variants are {sorted(variants)}",
        )


class TheReadmeAgainstTheRuntime(Case):
    """`README.md` § *What no run can meet yet* is the document an operator reads before spending
    $0.07 a turn. Held against a swarm shaped the way this runtime actually records one, it is
    missing three of the clauses it should name."""

    def unmeetable_here(self) -> set:
        """The clauses a maximal swarm, recorded as THIS runtime records one, still misses.

        Everything the fixture can make true is true: a post, a take, a finish, two transcripts,
        two attributed spend rows, a frame refusal and a ceiling refusal. The single change is the
        `role` — `Coordinator` for both agents, which is the only variant `swarm.agent.Role` has
        and the only value any real log carries.
        """
        root = fixtures.build(
            self.root,
            roles=((COORDINATOR, "Coordinator"), (BUILDER, "Coordinator")),
            ceiling="capped-file",
        )
        return {c.number for c in checker.check(root).clauses if not c.met}

    def test_the_readme_names_every_clause_no_run_can_meet(self) -> None:
        section = README.read_text().split("## What no run can meet yet")[1]
        section = section.split("\n## ")[0]
        named = {int(n) for n in re.findall(r"[Cc]lause (\d)", section)}
        missing = self.unmeetable_here() - named
        self.assertEqual(
            missing,
            set(),
            "clause(s) "
            + ", ".join(str(n) for n in sorted(missing))
            + " cannot be met by any swarm this runtime records — every AgentSpawned it writes "
            "carries role Coordinator, the only variant swarm.agent.Role has — and the README "
            "section that exists to say so before money is spent names only "
            + ", ".join(str(n) for n in sorted(named)),
        )


class ClauseFourAndTheAssignmentItNeverChecks(Case):
    """Clause 4's own `looked_for` says "for the assignment clause 2 matched". It compares agents
    only: `handovers()` pairs a post to a take by `agent_id`, and `clause_four` reduces the pairs
    to `takers`, a set of agent ids. The assignment identity is on the record — the take carries
    `ref`, the done carries `assignment_id` — and is never read."""

    def log(self):
        (self.root / "turns").mkdir(parents=True, exist_ok=True)
        log = fixtures.EventLog(self.root / "eventlog.sqlite3")
        log.append(
            "swarm.manager.SwarmCreated",
            {"swarm_id": fixtures.SWARM_ID, "display_name": "two", "home": str(self.root)},
        )
        log.append("swarm.manager.SwarmStarted", {"swarm_id": fixtures.SWARM_ID})
        for agent, role in ((COORDINATOR, "Coordinator"), (BUILDER, "Worker")):
            log.append(
                "swarm.agent.AgentSpawned",
                {
                    "agent_id": agent,
                    "swarm_id": fixtures.SWARM_ID,
                    "role": role,
                    "harness": "ClaudeCode",
                    "display_name": agent,
                    "host": {},
                },
            )
        return log

    @staticmethod
    def post(log, assignment):
        log.append(
            "swarm.agent.AssignmentPosted",
            {
                "agent_id": BUILDER,
                "outcome": "an outcome",
                "signoff": "Timo",
                "forbidden": "nothing",
                "artifact_ref": None,
            },
            actor="swarm.agent.SwarmAgent",
        )
        log.append(
            "swarm.agent.AssignmentRecorded",
            {
                "assignment_id": assignment,
                "agent_id": BUILDER,
                "outcome": "an outcome",
                "signoff": "Timo",
                "forbidden": "nothing",
                "artifact_ref": None,
            },
            actor="swarm.agent.SwarmAgent",
        )

    @staticmethod
    def take(log, assignment):
        log.append(
            "swarm.agent.AssignmentTaken",
            {"agent_id": BUILDER, "ref": assignment, "msg": "taken"},
            actor="swarm.agent.SwarmAgent",
        )

    @staticmethod
    def done(log, assignment):
        log.append(
            "swarm.agent.AssignmentDone",
            {"assignment_id": assignment, "note": "done"},
            actor="swarm.agent.SwarmAgent",
        )

    def test_clause_four_is_met_by_a_finish_of_an_assignment_that_was_never_taken(self) -> None:
        """Two assignments, one agent. The one it took is open; the one it finished is another.

        The shape a swarm that outlives one unit of work has: `builder` finished assignment A on
        an earlier goal, then was posted assignment B and took it and stopped. `AssignmentTaken`
        carries `ref` = B and `AssignmentDone` carries `assignment_id` = A, so the record itself
        distinguishes them; the checker reads neither field and reports the work "finished by the
        agent that took it".
        """
        first, second = str(uuid.uuid4()), str(uuid.uuid4())
        log = self.log()
        self.post(log, first)
        self.take(log, first)
        self.done(log, first)
        self.post(log, second)
        self.take(log, second)
        # `second` is open — nothing finishes it. Only `first` was ever done, and clause 2 pairs
        # the latest post with the latest take, which is `second`.
        log.close()
        (self.root / "turns").mkdir(parents=True, exist_ok=True)
        clause = self.clauses(self.root)[4]
        post_seq = [
            e.seq for e in checker.read(self.root).named("swarm.agent.AssignmentPosted")
        ][-1]
        self.assertMet(clause, False)
        self.assertNotIn(
            "which took the assignment",
            clause.found,
            f"the last handover is the post at seq {post_seq} of assignment {second}, which "
            f"nothing finished; the only AssignmentDone names {first}",
        )

    def test_clause_four_is_met_by_a_finish_recorded_before_the_work_was_posted(self) -> None:
        """An end-of-work event that predates the posting cannot have finished that work.

        `clause_four` puts no ordering between the finish and the take — `hits` filters on agent
        membership in `takers` and nothing else — so a done at seq 5 satisfies a take at seq 9.
        """
        old, current = str(uuid.uuid4()), str(uuid.uuid4())
        log = self.log()
        self.post(log, old)
        self.take(log, old)
        self.done(log, old)
        log.close()
        done_seq = max(
            e.seq for e in checker.read(self.root).named("swarm.agent.AssignmentDone")
        )

        log = fixtures.EventLog.__new__(fixtures.EventLog)
        log.db = sqlite3.connect(self.root / "eventlog.sqlite3")
        log.version = done_seq
        self.post(log, current)
        self.take(log, current)
        log.close()

        records = checker.read(self.root)
        take_seq = max(e.seq for e in records.named("swarm.agent.AssignmentTaken"))
        clause = self.clauses(self.root)[4]
        self.assertMet(clause, False)
        self.assertNotIn(
            "which took the assignment",
            clause.found,
            f"the only AssignmentDone is at seq {done_seq}, before the assignment now open was "
            f"taken at seq {take_seq}; a finish cannot precede the work it finished",
        )


class ClauseTwoAndTheHandoverItNames(Case):
    """`handovers()` is now shared by clauses 2 and 4, and pairs every post with every later take
    by the same agent. The first pair — the one clause 2 prints as its evidence — is the OLDEST
    post, not the one the take answers."""

    def test_clause_two_names_a_posting_that_was_never_answered(self) -> None:
        """`builder` is posted A and never takes it; it is posted B and takes B.

        Clause 2 is met, correctly — B was handed over. Its `found` names the post of A, because
        `pairs[0]` is the first post crossed with the first later take. A re-run pointed at that
        seq is pointed at the assignment that was abandoned.
        """
        abandoned, taken = str(uuid.uuid4()), str(uuid.uuid4())
        helper = ClauseFourAndTheAssignmentItNeverChecks
        (self.root / "turns").mkdir(parents=True, exist_ok=True)
        log = fixtures.EventLog(self.root / "eventlog.sqlite3")
        log.append(
            "swarm.manager.SwarmCreated",
            {"swarm_id": fixtures.SWARM_ID, "display_name": "two", "home": str(self.root)},
        )
        log.append("swarm.manager.SwarmStarted", {"swarm_id": fixtures.SWARM_ID})
        for agent, role in ((COORDINATOR, "Coordinator"), (BUILDER, "Worker")):
            log.append(
                "swarm.agent.AgentSpawned",
                {
                    "agent_id": agent,
                    "swarm_id": fixtures.SWARM_ID,
                    "role": role,
                    "harness": "ClaudeCode",
                    "display_name": agent,
                    "host": {},
                },
            )
        helper.post(log, abandoned)
        helper.post(log, taken)
        helper.take(log, taken)
        log.close()
        (self.root / "turns").mkdir(parents=True, exist_ok=True)

        records = checker.read(self.root)
        posts = records.named("swarm.agent.AssignmentPosted")
        answered = posts[-1].seq
        clause = self.clauses(self.root)[2]
        self.assertMet(clause, True)
        self.assertIn(
            f"at seq {answered}",
            clause.found,
            f"the take answers the posting at seq {answered} (assignment {taken}); clause 2 "
            f"reports the posting at seq {posts[0].seq} (assignment {abandoned}), which nothing "
            "took",
        )


if __name__ == "__main__":
    unittest.main()

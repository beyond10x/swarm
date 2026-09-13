#!/usr/bin/env python3
"""Adversary, wave 2026-09-13b unit A, pass 2 — what the corrections left standing.

Run it:

    python3 -m unittest discover -s examples/two-agents

Three cases, each driving the implementation against a document the unit wrote about itself:

* `TheWindowThatRunsPastTheVerdict` — `unattended`'s own docstring quotes the demonstration
  story's wording, "between the swarm starting and a verdict being recorded", as the reason the
  opening `SwarmStarted` is excepted. The window it actually computes runs from the opener to the
  LAST event in the log, so an operator's command after the verdict is counted as a hand on the
  run. `data/swarms/two-agents-proof` is that shape: `swarm.goal.GoalReached` at seq 23 and an
  operator's `swarm.goal.GoalSet` at seq 24.

* `TheVocabularyGuardOnAStructVariant` — `TheIssuerVocabulary`'s rewritten docstring says the
  enumeration "starts at the variants, not at `label`'s arms" so that "a variant with no readable
  label is a MISSING label, reported as that variant", and that "a parser that fails open is not
  a guard". `issuer_variants` matches `^    (\\w+)[,(]`, so a struct variant — `Scheduler { at:
  String },` — is matched by neither `,` nor `(` and is not seen at all.

* `AMemberWhoseSlugBeginsWithAQuestionMark` — `store.rs`'s `UNNAMEABLE` const accepts one
  collision by name: "An agent whose id begins with `?` would collide with the prefix. That is
  accepted rather than guarded: ... the collision costs one reader one wrong attribution in a
  case nothing has ever produced." The cost is measured here instead of estimated, and it is not
  one wrong attribution: it is clause 2 NOT MET, which is exit 1 for the whole demonstration.

Nothing here edits an implementation file, and nothing here edits pass 1's files. The one
mutation is made on a synthetic source built in this file, never on `store.rs`.
"""

from __future__ import annotations

import tempfile
import textwrap
import unittest
from pathlib import Path

import fixtures
import test_checker
from fixtures import COORDINATOR

checker = test_checker.checker


class TheWindowThatRunsPastTheVerdict(test_checker.Case):
    """An operator's command AFTER the verdict, counted against a run nobody touched during it.

    `unattended`'s docstring gives the exception for the opening `SwarmStarted` its reason in the
    story's own words:

        the `SwarmStarted` that OPENS the window does not count against it. A person starts a
        swarm by construction, so counting it would make the condition unmeetable ... The story's
        own word is "between the swarm starting and a verdict being recorded"

    The window the function computes is not that. It is

        window = [event for event in records.events if event.seq >= opener.seq]

    — the opener to the end of the log, with no verdict in it anywhere. A person who starts a
    swarm, lets it run to `swarm.goal.GoalReached` untouched, and then does anything at all
    afterwards gets NOT MET for a run that met the condition the story states.

    **This is a shape already on disk.** `data/swarms/two-agents-proof`, the log the demonstration
    rests on, ends `swarm.goal.GoalReached` at seq 23 and `swarm.goal.GoalSet` at seq 24 with
    `swarm.goal.Operator` in the actor column — a person setting the next goal after the verdict
    landed. It answers NOT MET today for a different reason (it predates issuers), and the next
    run of the same shape answers NOT MET for this one. The evidence README says that run "will be
    better evidence than this one"; it will get the same two words.

    Closing the window at the verdict is the whole fix, and `records.named(...)` over the
    finish/verdict names the clause-4 machinery already knows is where to close it.
    """

    def a_run_nobody_touched_until_after_the_verdict(self) -> Path:
        """One log, by hand, in the shape `two-agents-proof` ends in.

        `fixtures.build`'s `operator_in_window` writes an operator row too, and it writes it in a
        position that reads both ways — after `AssignmentDone`, which clause 4 treats as the
        finish. This is unambiguous instead: the verdict is `swarm.goal.GoalReached` and the only
        operator command in the log is strictly after it.
        """
        self.root.mkdir(parents=True, exist_ok=True)
        log = fixtures.EventLog(self.root / "eventlog.sqlite3")
        log.append(
            "swarm.manager.SwarmCreated",
            {"swarm_id": fixtures.SWARM_ID, "display_name": "two agents", "home": str(self.root)},
            issuer=fixtures.OPERATOR,
        )
        log.append(
            "swarm.goal.GoalSet",
            {"goal_id": fixtures.GOAL_ID, "swarm_id": fixtures.SWARM_ID, "text": "hand it over"},
            issuer=fixtures.OPERATOR,
        )
        # The opener, which the rule excepts — a person starts a swarm by construction.
        log.append(
            "swarm.manager.SwarmStarted",
            {"swarm_id": fixtures.SWARM_ID},
            issuer=fixtures.OPERATOR,
        )
        log.append(
            "swarm.goal.GoalPursued",
            {"goal_id": fixtures.GOAL_ID, "swarm_id": fixtures.SWARM_ID},
            issuer=fixtures.RUNTIME,
        )
        # The verdict. Everything between it and the opener is the loop's own.
        log.append(
            "swarm.goal.GoalReached",
            {"goal_id": fixtures.GOAL_ID, "swarm_id": fixtures.SWARM_ID},
            issuer=fixtures.RUNTIME,
        )
        # And a person, afterwards. `two-agents-proof` seq 24, in shape.
        log.append(
            "swarm.goal.GoalSet",
            {"goal_id": fixtures.GOAL_ID, "swarm_id": fixtures.SWARM_ID, "text": "and the next"},
            issuer=fixtures.OPERATOR,
        )
        log.close()
        return self.root

    def test_an_operator_after_the_verdict_is_not_a_hand_on_the_run(self) -> None:
        root = self.a_run_nobody_touched_until_after_the_verdict()
        events = checker.read(root).events
        verdict = [event for event in events if event.name == "swarm.goal.GoalReached"]
        self.assertEqual(len(verdict), 1, "one verdict in the log")
        hands = [
            event
            for event in events
            if checker.issued_by(event) == checker.OPERATOR and event.seq > verdict[0].seq
        ]
        self.assertEqual(
            [event.seq for event in hands],
            [verdict[0].seq + 1],
            "the one operator command after the opener is after the verdict too",
        )

        unattended = checker.check(root).unattended
        self.assertTrue(
            unattended.met,
            "no operator input between the swarm starting and the verdict being recorded, which "
            "is the condition `unattended`'s own docstring quotes as its reason for excepting "
            f"the opener. found: {unattended.found}",
        )


class TheVocabularyGuardOnAStructVariant(unittest.TestCase):
    """The derivation starts at the variants, and a struct variant is not one of them.

    `TheIssuerVocabulary`'s docstring, rewritten for correction round 1 finding 2:

        **The enumeration starts at the variants, not at `label`'s arms** ... Starting at the
        variants turns that silence into a name: a variant with no readable label is a MISSING
        label, reported as that variant. The failure being guarded against is silent, so the guard
        has to be loud, and a parser that fails open is not a guard.

    `issuer_variants` finds them with `re.findall(r"^    (\\w+)[,(]", ...)`. A Rust enum variant
    has three shapes and that regex reads two of them: a unit variant ends in `,` and a tuple
    variant opens a `(`. A STRUCT variant — `Scheduler { at: String },` — has a space and a brace
    where the regex wants one of those two characters, so it is not a variant this guard has ever
    heard of.

    The consequence is the one the guard's own docstring spells out and is the reason it exists: a
    fifth value in `subject` that the checker does not name makes `issued_by` return `None`, so
    every log carrying it reads as one written before issuers existed, clause 2 reports it is
    coping and `unattended` says NOT MET, forever, with nothing red.

    Asked on a synthetic source, for the reason the unit itself gave when it rewrote pass 1's
    version of this case: a mutation anchored on the real `store.rs` stops testing anything the
    moment that source legitimately changes.
    """

    #: The four `Issuer` variants as they stand, spelled the way `store.rs` spells them.
    #:
    #: Synthetic rather than quoted, for the reason the unit itself gave when it rewrote pass 1's
    #: version of this case: a stand-in anchored on the real source stops asking anything the
    #: moment that source legitimately changes. `test_the_guard_is_green_against_the_four` is the
    #: control that this copy is a good one.
    FOUR = textwrap.dedent(
        """\
        pub enum Issuer {
            Runtime,
            Operator,
            Agent(String),
            UnnameableAgent(String),
        }

        const AGENT: &str = "agent:";

        const UNNAMEABLE: &str = "agent:?";

        impl Issuer {
            pub fn label(&self) -> String {
                match self {
                    Self::Runtime => "runtime".to_owned(),
                    Self::Operator => "operator".to_owned(),
                    Self::Agent(name) => format!("{AGENT}{name}"),
                    Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),
                }
            }
        }
        """
    )

    #: The same four, and a fifth declared as a STRUCT variant.
    #:
    #: Its arm is unreadable as well as differently shaped, so if the guard saw the variant at all
    #: it would have two reasons to fail by name. A green can only mean it never saw it.
    A_STRUCT_VARIANT = FOUR.replace(
        "    UnnameableAgent(String),\n}",
        "    UnnameableAgent(String),\n    Scheduler { at: String },\n}",
        1,
    ).replace(
        '            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),\n',
        '            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),\n'
        '            Self::Scheduler { at } => format!("sched:{at}"),\n',
        1,
    )

    def a_store_holding(self, source: str) -> Path:
        tmp = tempfile.TemporaryDirectory(prefix="adversary-pass-2-store-")
        self.addCleanup(tmp.cleanup)
        copy = Path(tmp.name) / "store.rs"
        copy.write_text(source)
        return copy

    def the_guards_reading_of(self, store: Path):
        """Run the guard's own derivation against a store of our choosing.

        `labels_from_the_runtime` reads the module global `STORE` when it is called, so pointing
        that at a copy is enough and `store.rs` is never touched.
        """
        original = test_checker.STORE
        test_checker.STORE = store
        try:
            case = test_checker.TheIssuerVocabulary(
                "test_the_checker_names_every_value_the_runtime_can_write"
            )
            result = unittest.TestResult()
            case.run(result)
        finally:
            test_checker.STORE = original
        return result

    def test_the_guard_is_green_against_the_four(self) -> None:
        """The control. A red below has to come from the fifth variant, not from a poor copy."""
        self.assertIn("Scheduler", self.A_STRUCT_VARIANT, "the stand-in has a fifth variant")
        self.assertNotIn("Scheduler", self.FOUR, "and the control does not")
        result = self.the_guards_reading_of(self.a_store_holding(self.FOUR))
        self.assertEqual(
            (result.failures, result.errors),
            ([], []),
            "the vocabulary case passes against a faithful copy of the four variants",
        )

    def test_a_struct_variant_is_a_missing_label_and_not_a_silence(self) -> None:
        result = self.the_guards_reading_of(self.a_store_holding(self.A_STRUCT_VARIANT))
        self.assertNotEqual(
            (result.failures, result.errors),
            ([], []),
            "a struct variant of `Issuer` is a fifth value the log's `subject` column can hold "
            "and the checker does not name, and the guard that exists to catch exactly that read "
            "straight past it: `^    (\\w+)[,(]` wants a `,` or a `(` and a struct variant "
            "carries a `{`. The guard's own docstring: \"a parser that fails open is not a "
            "guard\"",
        )
        self.assertIn(
            "Scheduler",
            "".join(traceback for _, traceback in result.failures),
            "and it names the variant it could not read",
        )


class AMemberWhoseSlugBeginsWithAQuestionMark(test_checker.Case):
    """The one collision `store.rs` accepts by name, costed rather than estimated.

        /// An agent whose id begins with `?` would collide with the prefix. That is accepted
        /// rather than guarded: the guard would be a second rule about names, and the collision
        /// costs one reader one wrong attribution in a case nothing has ever produced.

    `?` is `is_ascii_graphic`, it is neither `@` nor a space, and `swarm.agent.Spawn`'s `agent_id`
    is a bare `String` with no pattern in `src/core/domains/agent.yaml`. So `agent:?worker` is
    what `Issuer::agent("?worker")` writes — a NAMEABLE agent, through `Issuer::Agent`, not
    through `UnnameableAgent` at all. The `--agent` flag on `swarm-cli` is `global = true` and
    reaches the same place in one word.

    What the reader does with it is `agent_named_by`, which returns `None` for anything starting
    with `agent:?` because it takes the rest for a digest. `who_took_it` then answers that the
    taking "was issued by an agent whose name this log cannot hold, so it cannot be shown to be
    the member" — of a member that issued its own taking, under its own name, which the log held
    perfectly well.

    That is not one wrong attribution in a report. `clause_two`'s `met` is `met and issued and not
    forged`, so it is clause 2 NOT MET, `report.ok` False, and exit 1 on a swarm that did
    everything the clause asks. The judgement was that the collision was not worth a rule; what it
    was weighed against was the wrong cost.

    The rule that removes it is one character: spell `UnnameableAgent` with something `agent:` +
    an identity cannot start with. `validate_identity` refuses a space, so `agent: ` — or any of
    the bytes `validate_field` refuses — cannot be the first character of a nameable slug.
    """

    SLUG = "?worker"

    def test_a_member_that_took_its_own_work_is_not_read_as_a_digest(self) -> None:
        clause = self.clauses(
            roles=((COORDINATOR, "Coordinator"), (self.SLUG, "Worker")),
            posted_to=self.SLUG,
            taken_by=self.SLUG,
            taken_by_issuer=fixtures.by(self.SLUG),
            finished_by=self.SLUG,
        )[2]

        events = checker.read(self.root).events
        takes = [event for event in events if event.name == checker.TAKEN]
        self.assertEqual(len(takes), 1, "one taking in the log")
        self.assertEqual(
            takes[0].issuer,
            f"agent:{self.SLUG}",
            "the member issued it under its own name, which the identity column holds",
        )

        self.assertMet(clause, True)


if __name__ == "__main__":
    unittest.main()

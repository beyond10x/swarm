#!/usr/bin/env python3
"""Adversary, wave 2026-09-13b unit A, pass 1 — the checker's half of the issuer story.

Run it:

    python3 -m unittest discover -s examples/two-agents

Three cases, each driving the implementation against a document the unit wrote about itself:

* `TheIssuerVocabularyGuard` — `test_checker.TheIssuerVocabulary`'s docstring says "a FIFTH
  variant added to `Issuer` fails here rather than being quietly unread by the only thing that
  reads the column". A copy of `store.rs` with a fifth variant is the way to ask.
* `TheUnattendedConditionOnAnEmptyWindow` — the checker's own module docstring says "Nothing
  passes by finding nothing", and `unattended` is a `Clause` it renders as `met`.
* `TheExportedEvidence` — `evidence/2026-09-13-two-agents-proof/checker-output.txt` is cited by
  `README.md` and by `AGENTS.md` as this checker's verdict on the demonstration, and it records a
  verdict line for a log shape this checker still meets.

Nothing here edits an implementation file: the mutation in the first case is made on a COPY, in a
temporary directory, and the module under test is pointed at it for the length of one case.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import fixtures
import test_checker

checker = test_checker.checker
HERE = Path(__file__).resolve().parent
EVIDENCE = HERE / "evidence" / "2026-09-13-two-agents-proof" / "checker-output.txt"


class TheIssuerVocabularyGuard(unittest.TestCase):
    """The derivation is only worth what it catches, and a fifth variant is what it claims to catch.

    `TheIssuerVocabulary`'s docstring: "The unrecognised-arm failure is the load-bearing half: a
    FIFTH variant added to `Issuer` fails here rather than being quietly unread by the only thing
    that reads the column." This asks that directly, by handing the guard an `Issuer` with five.

    **Rewritten 2026-09-13, on the coordinator's instruction, by the unit this case was written
    against.** The case was correct and it found a real defect: the guard walked `Issuer::label`'s
    arms with a regex ending in `,\n`, and `rustfmt` writes any arm too long for one line as a
    BLOCK ending in `}`, so a fifth variant whose label is computed was not an unreadable arm —
    it was no arm at all, and the derivation reported the same four labels it always had.

    What could not survive was the way the case ASKED. It string-replaced two literals out of the
    real `store.rs`, and finding 1's fix removed both: two members whose names the log cannot hold
    must record different envelopes, the envelope is `label(&self)` and so a function of `self`
    alone, therefore `self` must differ between them, therefore `UnnameableAgent` cannot be a unit
    variant. The anchor was made unsatisfiable by the fix this same pass demanded — not by
    carelessness on either side.

    So it asks the same question of a source built here rather than quoted from there. Nothing in
    it can be invalidated by a legitimate change to `store.rs`, which is what a guard against a
    silent failure has to be true of, and the two cases below are the same two as before: a
    control that the derivation agrees with the checker, and the claim.
    """

    #: The four variants the runtime has, written the way `store.rs` writes them — including the
    #: digest arm finding 1 added. A stand-in has to carry all four, because a parser that skipped
    #: a fifth in silence would derive exactly these and agree with the checker: the case is only
    #: red if the fifth is CAUGHT, never if the stand-in is short.
    FOUR = """pub enum Issuer {
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

    #: A fifth variant, and a `label` arm for it written the way `rustfmt` writes a long one.
    FIFTH_VARIANT = """    UnnameableAgent(String),
    /// A fifth kind of issuer, added by a later story.
    Scheduler,
}"""
    FIFTH_ARM = """            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),
            Self::Scheduler => {
                let mut said = String::from("sched");
                said.push_str("uler");
                said
            }"""

    def a_store_with(self, source: str) -> Path:
        """One source, on disk, for the guard to read. Nothing real is written or read."""
        tmp = tempfile.TemporaryDirectory(prefix="adversary-store-")
        self.addCleanup(tmp.cleanup)
        copy = Path(tmp.name) / "store.rs"
        copy.write_text(source)
        return copy

    def a_store_with_a_fifth_issuer(self) -> Path:
        """The four, plus a fifth whose label is computed rather than returned."""
        source = self.FOUR.replace("    UnnameableAgent(String),\n}", self.FIFTH_VARIANT, 1)
        source = source.replace(
            '            Self::UnnameableAgent(claim) => format!("{UNNAMEABLE}{}", mark(claim)),',
            self.FIFTH_ARM,
            1,
        )
        self.assertIn("Self::Scheduler", source, "the fifth arm is in the stand-in")
        self.assertNotEqual(source, self.FOUR, "the stand-in has a fifth variant")
        return self.a_store_with(source)

    def outcome_of(self, name: str, store: Path):
        """Run one of `TheIssuerVocabulary`'s cases against a store of our choosing.

        The whole case, not its internals: what is being asked is whether the GUARD goes red, and
        a guard that derives correctly and then compares against the wrong thing is a guard that
        is green when it should not be.
        """
        original = test_checker.STORE
        test_checker.STORE = store
        try:
            case = test_checker.TheIssuerVocabulary(name)
            result = unittest.TestResult()
            case.run(result)
        finally:
            test_checker.STORE = original
        return result

    def test_the_guard_is_green_against_the_store_it_was_written_for(self) -> None:
        """The control, twice over.

        Against the real `store.rs`, so the derivation is known to agree with the checker's
        constants on the source that actually ships; and against the stand-in, so a red in the
        case below is known to come from the fifth variant and not from the stand-in being a poor
        copy of the four.
        """
        for what, store in (
            ("the real store", test_checker.STORE),
            ("the stand-in for its four variants", self.a_store_with(self.FOUR)),
        ):
            with self.subTest(store=what):
                result = self.outcome_of(
                    "test_the_checker_names_every_value_the_runtime_can_write", store
                )
                self.assertEqual(
                    (result.failures, result.errors),
                    ([], []),
                    f"the vocabulary case passes against {what}",
                )

    def test_a_fifth_variant_whose_label_arm_is_a_block_is_caught(self) -> None:
        """The claim, asked directly.

        A fifth `Issuer` variant means a fifth value in the log's `subject` column, and the
        checker names four. An event carrying the fifth reads as a row written before issuers
        existed — `issued_by` returns `None` — so clause 2 reports it is coping and `unattended`
        says NOT MET, silently and forever. That is the silent downgrade to "I cannot tell" the
        whole story was filed to end, and this guard is what is supposed to stand between.

        It has to fail BY NAME. A guard that goes red without saying which variant it could not
        read sends the next reader back to the regex instead of to the enum, and this failure is
        one nobody is looking for in the first place.
        """
        result = self.outcome_of(
            "test_the_checker_names_every_value_the_runtime_can_write",
            self.a_store_with_a_fifth_issuer(),
        )
        self.assertNotEqual(
            (result.failures, result.errors),
            ([], []),
            "a fifth `Issuer` variant must fail the vocabulary case, and it did not: if its "
            "label arm is read by a regex that wants a trailing comma, a block-bodied arm is not "
            "an unreadable arm but no arm at all, and the derivation reports the same four labels "
            "it reported before",
        )
        self.assertIn(
            "Scheduler",
            "".join(traceback for _, traceback in result.failures),
            "the guard names the variant it could not read",
        )


class TheUnattendedConditionOnAnEmptyWindow(test_checker.Case):
    """A verdict of `met` reached by finding nothing at all.

    `check-two-agents.py`'s own module docstring, unchanged by this unit: "**Nothing passes by
    finding nothing.** `swarm.agent.AssignmentTaken` has fired zero times in this repository's
    history, so an empty result is the default state of the world and never evidence of anything."

    `unattended` now excepts the `SwarmStarted` that opens the window — correctly, a person starts
    a swarm — and then asks whether any REMAINING event was issued by an operator. On a swarm that
    was started and then did nothing there are no remaining events, so the answer is "none", and
    the checker prints `unattended  met     no operator input in the window`. It is a positive
    claim about whose hands were on a swarm, derived from a window of one excepted event.

    A swarm started with no coordinator program configured leaves exactly this log; three of the
    eleven logs under `data/swarms/` are swarms abandoned at or before this point.
    """

    def test_a_window_holding_only_the_excepted_opener_is_not_unattended(self) -> None:
        report = self.report(
            roles=(),
            posted_to=None,
            taken_by=None,
            finished=None,
            ceiling=None,
            turns=(),
        )
        window = [
            event
            for event in checker.read(self.root).events
            if event.name == "swarm.manager.SwarmStarted"
        ]
        self.assertEqual(len(window), 1, "the log holds one `SwarmStarted`")
        self.assertEqual(len(checker.read(self.root).events), 3, "and nothing after it")
        self.assertFalse(
            report.unattended.met,
            "a window whose only event is the one the rule excepts is no evidence that nobody "
            f"touched this swarm; found: {report.unattended.found}",
        )


class TheExportedEvidence(test_checker.Case):
    """The committed evidence for `verification-report:two-agents-under-a-frame-2026-09-13`.

    `examples/two-agents/README.md` lists `checker-output.txt` as "`check-two-agents.py`'s verdict,
    seven clauses, exit 0", and `AGENTS.md` cites the same directory. The swarm it reports on,
    `data/swarms/two-agents-proof`, is a log written before issuers existed — which is the shape
    `fixtures.build(issuers=False)` writes — and the unit left the export untouched while changing
    what the checker prints for exactly that shape.

    So the evidence in the repository asserts a line this checker no longer produces for the log
    it was produced from, and a reader who re-runs the checker against the original swarm gets a
    different report from the one the verification report rests on.
    """

    def test_the_evidence_quotes_an_unattended_line_this_checker_still_prints(self) -> None:
        self.assertTrue(EVIDENCE.is_file(), f"the exported verdict is at {EVIDENCE}")
        exported = [
            line.rstrip()
            for line in EVIDENCE.read_text().splitlines()
            if line.startswith("unattended")
        ]
        self.assertEqual(len(exported), 1, f"one unattended line in {EVIDENCE}: {exported}")

        rendered = checker.render(self.report(issuers=False))
        printed = [
            line.rstrip() for line in rendered.splitlines() if line.startswith("unattended")
        ]
        self.assertEqual(len(printed), 1, f"one unattended line from the checker: {printed}")
        self.assertEqual(
            printed[0],
            exported[0],
            "the committed evidence and the checker disagree about the demonstration's own log",
        )

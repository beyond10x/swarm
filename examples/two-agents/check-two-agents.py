#!/usr/bin/env python3
"""Read a finished swarm's records and report each of the story's seven clauses, by number.

`story:a-recorded-run-shows-two-agents-working` asks for a demonstration that a swarm is more than
a swarm manager, and its acceptance is seven clauses. This is the instrument that reads them off
the record, so that a run which clears six names the one it missed and the re-run targets that
clause rather than starting again — at two concurrent `claude-opus-5` sessions, "it didn't work" is
not an affordable report.

What it reads, all of it under `data/swarms/<slug>/`:

    eventlog.sqlite3      table `swarm_events` — the record. State is a fold over this.
    turns/*.jsonl         the metaharness event records, one file per turn ATTEMPT
    turns/spend.jsonl     one JSON object per attempt, carrying the agent since 2026-09-12
    turns/capped.jsonl    if the runtime ever writes one — see clause 7

Usage:

    examples/two-agents/check-two-agents.py data/swarms/<slug>
    examples/two-agents/check-two-agents.py <slug> [--data data/swarms] [--json]

Exit 0 when every one of the seven is met; exit 1 otherwise. The story's "unattended"
condition is reported beside them and decides nothing, because this runtime's log cannot
settle it either way — see `unattended()`.

Two rules this obeys, both of them load-bearing:

**A clause it cannot evaluate is NOT MET, never skipped.** An absent event log, an unreadable turn
file, a missing spend record — each of those is a clause that has not been shown, and a report that
stayed silent about it would read as a clause that passed.

**No clause passes by finding nothing.** `swarm.agent.AssignmentTaken` has fired zero times in this
repository's history, so an empty result is the default state of the world and not evidence of
anything. Every clause below is met only by a positive count, and `test_nothing_passes_by_finding_
nothing` walks every clause over an empty log to keep it that way.

Testing it needs no swarm: `fixtures.py` writes the records a run would leave, and
`test_checker.py` exercises every clause both ways. See that file for the command.
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
import textwrap
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

COORDINATOR_ROLE = "Coordinator"

SPAWNED = "swarm.agent.AgentSpawned"
POSTED = "swarm.agent.AssignmentPosted"
RECORDED = "swarm.agent.AssignmentRecorded"
TAKEN = "swarm.agent.AssignmentTaken"
STARTED = "swarm.manager.SwarmStarted"

# What `swarm.agent.FinishAssignment` emits, and what `swarm.agent.ReportGate`'s `green` outcome
# emits. The command names are accepted too, for a log that ever records one.
FINISHED = (
    "swarm.agent.AssignmentDone",
    "swarm.agent.GateGreen",
    "swarm.agent.FinishAssignment",
)

# A decision word that means the call did not happen. `tool.decided` carries either
# `{"decision": {"decision": "deny", ...}}` or a bare string, depending on the adapter.
DENIALS = {"deny", "denied", "refuse", "refused", "block", "blocked", "reject", "rejected"}

# `Capped::why()` for `Bound::Agent` — `src/runtime/swarm-server/src/budget.rs`. The agent bound is
# the one a goal cannot get round, and this sentence is the runtime's own way of saying it fired.
AGENT_CEILING = re.compile(
    r"by the agent [`'\"]?(?P<agent>[^`'\"\s]+)[`'\"]? across every goal it works"
)

# `{turn:04}-{attempt:02}-{agent}-{unit-prefix}.jsonl`. The agent segment was added by the
# 2026-09-13a wave, in the same wave that wrote this checker and on the other branch of it, so the
# first real run of the demonstration reported clause 3 NOT MET with "0 file(s)" for every turn:
# this pattern parsed the old three-field name and matched nothing. It is optional because a log
# written before that wave has no agent in the name, and those runs must still be readable.
TURN_FILE = re.compile(
    r"^(?P<turn>\d{4})-(?P<attempt>\d{2})-"
    r"(?:(?P<agent>[A-Za-z0-9_-]+?)-)?"
    r"(?P<goal>[0-9a-fA-F]{6,})(?:-(?P<rest>.+))?\.jsonl$"
)
# Before 2026-09-12 the attempt was not in the name, which is how a retry came to overwrite the
# record of what went wrong. Read, so that such a run can still be reported on.
LEGACY_TURN_FILE = re.compile(r"^(?P<turn>\d{4})-(?P<goal>[0-9a-fA-F]{6,})\.jsonl$")

TITLES = {
    1: "a non-Coordinator agent exists",
    2: "work was handed over",
    3: "two transcripts, neither overwritten",
    4: "the work was finished, not merely started",
    5: "the confinement acted",
    6: "spend is attributed per agent",
    7: "the bound held with more than one agent to bound",
}


@dataclass
class Clause:
    """One acceptance clause, and the evidence for the answer.

    `determinable` is False for a condition the records cannot settle either way. Such a thing is
    never `met` — an answer nobody can check is not a clause that passed — but it is not a failed
    clause either, so it does not decide the exit status. Today the unattended condition is the
    only one, for the reason its own `found` gives.
    """

    number: int
    title: str
    met: bool
    looked_for: str
    found: str
    determinable: bool = True


@dataclass
class Event:
    seq: int
    name: str
    actor: str
    at: str
    data: dict


@dataclass
class Turn:
    path: Path
    turn: int | None
    attempt: int | None
    goal: str | None
    rest: str | None
    records: list
    unreadable: str | None = None

    @property
    def key(self):
        return None if self.turn is None or self.goal is None else (self.turn, self.goal)


@dataclass
class Report:
    swarm: Path
    clauses: list
    unattended: Clause

    @property
    def ok(self) -> bool:
        """The seven clauses, and nothing else.

        The unattended condition used to be folded in here, and that was wrong in the one
        direction that matters: it could not fail. `store.rs:192` writes `actor.unwrap_or("system")`
        and `http.rs:38` makes the actor optional, so an operator driving a swarm by hand through
        the HTTP surface is recorded exactly as the loop is. `dsfsdf` — the swarm the story names
        as hand-driven — passed it with 26 operator commands inside its window. A condition that
        cannot fail is not evidence, and a condition that cannot fail deciding an exit status is
        worse than not checking it, so it no longer does. See `unattended()`.
        """
        return all(clause.met for clause in self.clauses)


@dataclass
class Records:
    """Everything a finished swarm left on disk, read once."""

    root: Path
    events: list = field(default_factory=list)
    log_error: str | None = None
    turns: list = field(default_factory=list)
    spend: list = field(default_factory=list)
    spend_error: str | None = None
    capped: list = field(default_factory=list)

    def named(self, *names: str) -> list:
        return [event for event in self.events if event.name in names]

    @property
    def spawned(self) -> dict:
        roles: dict = {}
        for event in self.named(SPAWNED):
            agent = event.data.get("agent_id")
            if isinstance(agent, str) and agent:
                roles[agent] = event.data.get("role")
        return roles

    @property
    def workers(self) -> set:
        """The agents spawned into a role that is not Coordinator."""
        return {agent for agent, role in self.spawned.items() if role != COORDINATOR_ROLE}

    @property
    def spend_agents(self) -> Counter:
        named: Counter = Counter()
        for row in self.spend:
            agent = row.get("agent")
            if isinstance(agent, str) and agent:
                named[agent] += 1
        return named

    @property
    def unattributed_spend(self) -> int:
        return len(self.spend) - sum(self.spend_agents.values())


def read(root) -> Records:
    """Read the log, the turn records and the spend record. Never raises: an unreadable input is
    a clause that is not met, and the reason travels with it."""
    root = Path(root)
    records = Records(root=root)
    if not root.is_dir():
        records.log_error = f"there is no directory at {root}"
        records.spend_error = records.log_error
        return records

    log = root / "eventlog.sqlite3"
    if not log.exists():
        records.log_error = f"there is no event log at {log}"
    else:
        try:
            records.events = read_events(log)
        except sqlite3.Error as why:
            records.log_error = f"{log} could not be read: {why}"

    turns = root / "turns"
    if not turns.is_dir():
        records.spend_error = f"there is no turns directory at {turns}"
    else:
        for path in sorted(turns.glob("*.jsonl")):
            if path.name in ("spend.jsonl", "capped.jsonl"):
                continue
            records.turns.append(read_turn(path))
        records.spend, records.spend_error = read_jsonl(turns / "spend.jsonl")
        records.capped, _ = read_jsonl(turns / "capped.jsonl")
    return records


def read_events(log: Path) -> list:
    """The event log, oldest first.

    Opened read-only where the platform allows it — a checker must not checkpoint a WAL into the
    record it is reading — and read-write only if that is refused, which is what a log with a live
    `-wal` beside it does.
    """
    try:
        db = sqlite3.connect(f"file:{log}?mode=ro", uri=True)
        db.execute("select 1 from swarm_events limit 1").fetchall()
    except sqlite3.Error:
        db = sqlite3.connect(str(log))
    with db:
        rows = db.execute(
            "select global_seq, event_name, actor, occurred_at, data "
            "from swarm_events order by global_seq"
        ).fetchall()
    db.close()
    events = []
    for seq, name, actor, at, data in rows:
        try:
            payload = json.loads(data)
        except (TypeError, ValueError):
            payload = {}
        events.append(
            Event(
                seq=seq,
                name=name,
                actor=actor or "",
                at=at or "",
                data=payload if isinstance(payload, dict) else {},
            )
        )
    return events


def read_jsonl(path: Path):
    """Every object in a JSON-lines file, and why there are none if there are none."""
    if not path.exists():
        return [], f"there is no {path.name} at {path}"
    rows = []
    for line in path.read_text(errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict):
            rows.append(row)
    return rows, None if rows else f"{path} holds no readable rows"


def read_turn(path: Path) -> Turn:
    match = TURN_FILE.match(path.name) or LEGACY_TURN_FILE.match(path.name)
    groups = match.groupdict() if match else {}
    records, _ = read_jsonl(path)
    return Turn(
        path=path,
        turn=int(groups["turn"]) if groups.get("turn") else None,
        attempt=int(groups["attempt"]) if groups.get("attempt") else None,
        goal=groups.get("goal"),
        rest=groups.get("rest"),
        records=records,
        unreadable=None if records else "no readable records",
    )


def listed(items, limit: int = 6) -> str:
    items = list(items)
    shown = ", ".join(str(item) for item in items[:limit])
    return shown + (f", and {len(items) - limit} more" if len(items) > limit else "")


def clause_one(records: Records) -> Clause:
    looked = f"a `{SPAWNED}` whose `role` is not `{COORDINATOR_ROLE}`"
    spawned = records.spawned
    workers = [agent for agent, role in spawned.items() if role != COORDINATOR_ROLE]
    if records.log_error:
        found = records.log_error
    elif not spawned:
        found = f"no `{SPAWNED}` at all, in {len(records.events)} event(s)"
    elif not workers:
        found = (
            f"{len(spawned)} agent(s) spawned, every one of them a {COORDINATOR_ROLE}: "
            + listed(f"`{agent}` ({role})" for agent, role in spawned.items())
        )
    else:
        found = f"{len(workers)} of {len(spawned)} spawned agent(s): " + listed(
            f"`{agent}` ({spawned[agent]})" for agent in workers
        )
    return Clause(1, TITLES[1], bool(workers), looked, found)


@dataclass
class Handover:
    """One unit of work posted to an agent and taken by it, with the assignment it is about."""

    post: Event
    recorded: Event | None
    take: Event
    agent: str
    assignment: str | None

    def describe(self) -> str:
        about = (
            f"assignment {self.assignment}"
            if self.assignment
            else "an assignment the take does not name (`ref` absent)"
        )
        return (
            f"posted to `{self.agent}` at seq {self.post.seq} and taken by it at seq "
            f"{self.take.seq}, {about}"
        )


def handovers(records: Records):
    """Every `AssignmentPosted` answered by a later `AssignmentTaken` from the same agent.

    Pairing is on the **assignment**, not on the agent id, wherever the record carries one — and
    the record does, in two places and never on the post: `AssignmentRecorded` carries
    `assignment_id` beside `agent_id`, and `AssignmentTaken` carries it as `ref`.
    `AssignmentPosted` carries neither, so the post that a take answers is the latest one
    preceding that assignment's `AssignmentRecorded`.

    Pairing by agent alone crossed every post with every later take: on post A, post B, take B
    the pair reported was (A, B), so clause 2's evidence named the posting that was abandoned,
    and clause 4 accepted a finish of A as the finish of B. The verdicts were right often enough
    that only the citations gave it away.

    Ordered by the take, so `[-1]` is the handover of record — the work in flight when the swarm
    stopped, which is what a demonstration is about.
    """
    workers = records.workers
    posts = records.named(POSTED)
    recorded = records.named(RECORDED)
    out = []
    for take in records.named(TAKEN):
        agent = take.data.get("agent_id")
        if not isinstance(agent, str) or agent not in workers:
            continue
        assignment = take.data.get("ref")
        assignment = assignment if isinstance(assignment, str) and assignment else None
        match = None
        if assignment is not None:
            match = next(
                (
                    event
                    for event in recorded
                    if event.data.get("assignment_id") == assignment
                    and event.data.get("agent_id") == agent
                    and event.seq < take.seq
                ),
                None,
            )
        # A post is only a candidate if it precedes the take; among those, the one this
        # assignment was recorded from, else the most recent.
        before = [
            post for post in posts if post.data.get("agent_id") == agent and post.seq < take.seq
        ]
        if match is not None:
            earlier = [post for post in before if post.seq < match.seq]
            before = earlier or before
        if not before:
            continue
        out.append(Handover(before[-1], match, take, agent, assignment))
    return out


def clause_two(records: Records) -> Clause:
    looked = (
        f"a `{POSTED}` naming a non-Coordinator agent, and a later `{TAKEN}` whose `agent_id` "
        "is that same agent. This establishes that the RECORD SAYS that agent took it; it "
        "cannot establish who issued the command, because no actor in this log names a "
        "person or an agent — see the unattended note"
    )
    posts = records.named(POSTED)
    takes = records.named(TAKEN)
    pairs = handovers(records)
    if records.log_error:
        found = records.log_error
    else:
        parts = [
            f"{len(posts)} `{POSTED}`"
            + (
                " (to " + listed(f"`{p.data.get('agent_id')}`" for p in posts) + ")"
                if posts
                else ""
            ),
            f"{len(takes)} `{TAKEN}`"
            + (
                " (by " + listed(f"`{t.data.get('agent_id')}`" for t in takes) + ")"
                if takes
                else " — it has fired zero times in this repository's history"
            ),
        ]
        if pairs:
            # The LAST handover, not the first: `[0]` was the oldest post crossed with the
            # oldest later take, so a post-fault-repost-take sequence cited the posting that
            # was abandoned and sent a re-run to the wrong seq.
            parts.append(pairs[-1].describe())
            if len(pairs) > 1:
                parts.append(
                    f"{len(pairs)} handover(s) in all; this is the one in flight last"
                )
        elif posts and takes:
            parts.append(
                "no posting is answered by a later taking from the same non-Coordinator agent"
            )
        found = "; ".join(parts)
    return Clause(2, TITLES[2], bool(pairs), looked, found)


def agent_of(turn: Turn, records: Records):
    """Which agent a turn file belongs to, and how that was established.

    Four ways, in the order a reader would trust them. The last is the one that works today: the
    turn file's name carries the turn number and the goal's first eight characters
    (`coordinator.rs::turn_file`) and `spend.jsonl` carries the same pair beside the agent, which
    is the join `Swarm::turns` itself uses to put a verdict against a transcript.

    **Every one of the four checks the name against `AgentSpawned`, and that is the only
    definition of an agent in this file.** The first path did not until 2026-09-13: it returned
    whatever string a record carried, so two transcripts naming `alice` and `bob` in a swarm that
    spawned neither read as "two transcripts attributed to two different agents". A name a turn
    file or a spend row gives an agent is a claim; `swarm.agent.AgentSpawned` is what settles it,
    and a second, looser set of "known" names existed here long enough to give clause 6 and
    clause 7 opposite answers about the same string.
    """
    known = set(records.spawned)
    for record in turn.records:
        agent = record.get("agent")
        if isinstance(agent, str) and agent in known:
            return agent, "named in the record"
    if turn.rest and turn.rest in known:
        return turn.rest, "named in the file name"
    for record in turn.records:
        cwd = record.get("cwd")
        if isinstance(cwd, str):
            for part in Path(cwd).parts:
                if part in known:
                    return part, "the work directory it ran in"
    if turn.key is not None:
        for row in records.spend:
            goal = row.get("goal")
            agent = row.get("agent")
            if (
                isinstance(goal, str)
                and isinstance(agent, str)
                and agent
                and agent in known
                and row.get("iterations") == turn.turn
                and goal.startswith(turn.goal)
            ):
                return agent, "spend.jsonl"
    return None, "nothing in the file, its name or the spend record names a spawned agent"


def clause_three(records: Records) -> Clause:
    looked = (
        "two turn files under `turns/`, each attributable to an agent the log knows and the "
        "two of them to different agents, and a file for every attempt `spend.jsonl` claims "
        "— an attempt with no file of its own is a record that was overwritten or never kept"
    )
    attributed = [(turn, *agent_of(turn, records)) for turn in records.turns]
    agents = {agent for _, agent, _ in attributed if agent}

    # An attempt is a spend row; a transcript is a file. The keys are the SPEND ROWS' own, not the
    # files': keying on the files asks "which attempts left a file that is missing a sibling" and
    # silently passes a turn whose every attempt vanished. `data/swarms/e2e-1789212621/` is exactly
    # that shape — spend claims iterations 1, 2, 3, 4 and only 3 and 4 have a transcript — and the
    # earlier revision of this reported it met while promising, one sentence above, "a file for
    # every attempt the spend record claims".
    rows_per_key: Counter = Counter()
    for row in records.spend:
        goal, turn = row.get("goal"), row.get("iterations")
        if isinstance(goal, str) and isinstance(turn, int):
            rows_per_key[(turn, goal[:8])] += 1
    files_per_key = {
        key: sum(
            1
            for a_turn in records.turns
            if a_turn.turn == key[0] and a_turn.goal and key[1].startswith(a_turn.goal[:8])
        )
        for key in rows_per_key
    }
    missing = [
        f"turn {key[0]} of goal {key[1]} has {rows} attempt(s) in spend.jsonl and "
        f"{files_per_key[key]} file(s): "
        + ("no transcript at all" if not files_per_key[key] else "a record was overwritten")
        for key, rows in sorted(rows_per_key.items())
        if rows > files_per_key[key]
    ]

    met = len(records.turns) >= 2 and len(agents) >= 2 and not missing
    parts = []
    if not records.turns:
        parts.append(records.spend_error or f"no turn files under {records.root / 'turns'}")
    else:
        # Counted per agent rather than listed per file: a real swarm has 44 of them, and a report
        # that prints 44 file names has buried the one number a reader wants.
        per_agent: Counter = Counter(
            (agent, how) for _, agent, how in attributed if agent is not None
        )
        nameless = [turn.path.name for turn, agent, _ in attributed if agent is None]
        parts.append(
            f"{len(records.turns)} turn file(s), {len(records.turns) - len(nameless)} attributed: "
            + (
                listed(f"`{agent}` ×{count} ({how})" for (agent, how), count in per_agent.items())
                or "none"
            )
        )
        parts.append(f"{len(agents)} distinct agent(s): " + (listed(sorted(agents)) or "none"))
        if nameless:
            parts.append(f"{len(nameless)} file(s) no agent is named on: " + listed(nameless, 3))
        unreadable = [turn.path.name for turn in records.turns if turn.unreadable]
        if unreadable:
            parts.append(
                f"{len(unreadable)} file(s) hold no readable record: " + listed(unreadable)
            )
    parts.extend(missing)
    return Clause(3, TITLES[3], met, looked, "; ".join(parts))


def finisher_of(event: Event, records: Records):
    """Which agent an end-of-work event says finished the work.

    `GateGreen` carries `agent_id` itself. `AssignmentDone` carries only `assignment_id` — the
    event's subject is the Assignment, not the agent — so the owner is read from the
    `AssignmentRecorded` that created it, which is the only event carrying both.
    """
    if isinstance(event.data.get("agent_id"), str):
        return event.data["agent_id"]
    assignment = event.data.get("assignment_id")
    if assignment is None:
        return None
    for recorded in records.named(RECORDED):
        if recorded.data.get("assignment_id") == assignment:
            owner = recorded.data.get("agent_id")
            return owner if isinstance(owner, str) else None
    return None


def assignment_named_by(event: Event):
    """The assignment an end-of-work event is about: `assignment_id` on `AssignmentDone`, `ref`
    on `GateGreen`. Either may be absent — `ref` is `Optional<String>` — and absent is not a
    match, it is a thing that cannot be checked."""
    for key in ("assignment_id", "ref"):
        value = event.data.get(key)
        if isinstance(value, str) and value:
            return value
    return None


def clause_four(records: Records) -> Clause:
    """Finished by the agent that took it, for the assignment it took, after it took it.

    Three predicates, and each was absent in turn. The first rule was "any `AssignmentDone` or
    `GateGreen` anywhere in the log", which a one-coordinator swarm satisfies by reporting its own
    gate green. The second added the agent — and accepted a finish of assignment A as the finish
    of the take of B by the same agent, which is the shape of a swarm that outlives one unit of
    work, and accepted a finish recorded BEFORE the work was posted, which is the shape of
    nothing at all.

    Where the record does not carry the assignment on both sides, this says so rather than
    reporting a match it did not make — the same answer clause 2 and `unattended()` give.
    """
    looked = (
        "a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a "
        "`swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent "
        "that took it, and naming that same assignment. `AssignmentPosted` carries no assignment "
        "identity, so the handover's is read from `AssignmentTaken.ref` and the "
        "`AssignmentRecorded` behind it; where either side names none, the agent and the ordering "
        "are checked and the assignment is not, and the finding says which"
    )
    pairs = handovers(records)
    latest = pairs[-1] if pairs else None
    finished = [(event, finisher_of(event, records)) for event in records.named(*FINISHED)]

    def why_not(event: Event, agent):
        """Why this end-of-work event is not the finish of the handover of record."""
        if latest is None:
            return "nothing was taken, so there is nothing for it to be the finish of"
        if event.seq <= latest.take.seq:
            return (
                f"recorded at seq {event.seq}, at or before the take at seq {latest.take.seq} — "
                "a finish cannot precede the work it finished"
            )
        if agent != latest.agent:
            return (
                f"by `{agent}`" if agent else "by nobody the record names"
            ) + f", and the work was taken by `{latest.agent}`"
        named = assignment_named_by(event)
        if latest.assignment and named and named != latest.assignment:
            return f"for assignment {named}, and the work taken was {latest.assignment}"
        return None

    hits = [(event, agent) for event, agent in finished if why_not(event, agent) is None]
    others = [(event, agent, why_not(event, agent)) for event, agent in finished if
              why_not(event, agent) is not None]
    if records.log_error:
        found = records.log_error
    elif hits:
        parts = []
        for event, agent in hits:
            named = assignment_named_by(event)
            matched = bool(latest.assignment and named and named == latest.assignment)
            parts.append(
                f"`{event.name}` at seq {event.seq} by `{agent}`, which took the assignment at "
                f"seq {latest.take.seq}"
                + (
                    f", both naming {named}"
                    if matched
                    else " — but "
                    + (
                        "the take names no assignment (`ref` absent)"
                        if not latest.assignment
                        else "this event names no assignment"
                    )
                    + ", so the agent and the ordering are checked and the assignment is not"
                )
            )
        found = listed(parts)
    elif others:
        found = f"{len(others)} end-of-work event(s), none of them this finish: " + listed(
            f"`{event.name}` at seq {event.seq} {why}" for event, _, why in others
        ) + (
            "; clause 2 matched no handover"
            if latest is None
            else f"; the handover of record is {latest.describe()}"
        )
    else:
        found = (
            f"neither, in {len(records.events)} event(s); the agent events present are "
            + (
                listed(
                    sorted(
                        {e.name for e in records.events if e.name.startswith("swarm.agent.")}
                    )
                )
                or "none"
            )
        )
    return Clause(4, TITLES[4], bool(hits), looked, found)


def refusals(records: Records):
    """Every tool call a turn record says was refused, with who refused it."""
    out = []
    for turn in records.turns:
        requested = {
            record.get("call_id"): record
            for record in turn.records
            if record.get("event") == "tool.requested"
        }
        for record in turn.records:
            if record.get("event") != "tool.decided":
                continue
            decision = record.get("decision")
            if isinstance(decision, dict):
                word, reason = decision.get("decision"), decision.get("reason")
            else:
                word, reason = decision, None
            if not isinstance(word, str) or word.lower() not in DENIALS:
                continue
            asked = requested.get(record.get("call_id"), {})
            out.append(
                {
                    "file": turn.path.name,
                    "decider": record.get("decided_by"),
                    "tool": asked.get("name"),
                    "operations": asked.get("operations"),
                    "reason": reason,
                }
            )
    return out


def census_of(turn: Turn):
    """A session's own tally of its decisions, if it wrote one.

    **This is not evidence of a refusal and is never read as one.** `by_decider` counts decisions
    of every outcome, not denials: `data/swarms/dsfsdf/turns/0035-77fc1fcc.jsonl` carries
    `{"allowed": 5, "denied": 0, "by_decider": {"observe": 5}}`. So `by_decider["frame"]` is true
    of a session where the frame allowed five calls and denied none, and an earlier revision of
    this file read exactly that as a frame refusal — "a flag that was set", which is the thing
    clause 5's own sentence says it is not. The tally is reported to a reader and decides nothing.
    """
    return next(
        (
            record["census"]
            for record in turn.records
            if record.get("event") in ("session.ended", "session.summary")
            and isinstance(record.get("census"), dict)
        ),
        None,
    )


def clause_five(records: Records) -> Clause:
    looked = (
        "a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is "
        "`frame` — one call, named, that did not happen. A session census is not this: its "
        "`by_decider` counts decisions of every outcome, so it cannot say the frame denied anything"
    )
    denied = refusals(records)
    by_frame = [entry for entry in denied if entry["decider"] == "frame"]
    decided = sum(
        1
        for turn in records.turns
        for record in turn.records
        if record.get("event") == "tool.decided"
    )
    # Reported, never counted: a census claiming denials that no `tool.decided` records is a hole
    # in the transcript, and a reader should see it rather than have it silently resolved.
    claimed = sum(
        (census_of(turn) or {}).get("denied") or 0
        for turn in records.turns
        if isinstance((census_of(turn) or {}).get("denied"), int)
    )
    unaccounted = (
        f"; the session censuses claim {claimed} denied call(s) and {len(denied)} `tool.decided` "
        "record(s) name a denier"
        if claimed != len(denied)
        else ""
    )
    if not records.turns:
        found = records.spend_error or f"no turn records under {records.root / 'turns'}"
    elif by_frame:
        entry = by_frame[0]
        found = (
            f"{len(by_frame)} of {decided} decided call(s) refused by the frame: "
            f"`{entry['tool']}` ({listed(entry['operations'] or [])}) in {entry['file']}"
            + (f" — {entry['reason']}" if entry["reason"] else "")
        )
    elif denied:
        found = (
            f"{len(denied)} of {decided} decided call(s) denied, none of them by the frame: "
            + listed(f"`{entry['decider']}` denied `{entry['tool']}`" for entry in denied)
            + unaccounted
        )
    else:
        found = (
            f"{decided} tool call(s) decided in {len(records.turns)} turn record(s), "
            "none denied" + unaccounted
        )
    return Clause(5, TITLES[5], bool(by_frame), looked, found)


def clause_six(records: Records) -> Clause:
    """Two agents each naming themselves on their own rows.

    The agents have to be agents the log says were spawned, not two strings: a figure a ceiling is
    written against must name who ran it up, and a name nobody recognises names nobody. It does
    NOT ask which role they hold — whether a second agent exists at all is clause 1's question,
    and the story is explicit that per-agent recording and per-agent enforcement fail
    independently. Clause 6 asks only whether the record attributes.
    """
    looked = (
        "rows in `turns/spend.jsonl` naming two different agents, both of them agents the log "
        "says were spawned"
    )
    named = records.spend_agents
    spawned = set(records.spawned) & set(named)
    met = len(named) >= 2 and len(spawned) >= 2
    if not records.spend:
        found = records.spend_error or "no rows in the spend record"
    else:
        parts = [
            f"{len(records.spend)} row(s), {len(named)} agent(s) named: "
            + (listed(f"`{agent}` ×{count}" for agent, count in named.most_common()) or "none")
        ]
        if records.unattributed_spend:
            parts.append(f"{records.unattributed_spend} row(s) naming no agent")
        strangers = set(named) - set(records.spawned)
        if strangers:
            parts.append(
                f"named by no `{SPAWNED}`: "
                + listed(f"`{agent}`" for agent in sorted(strangers))
            )
        found = "; ".join(parts)
    return Clause(6, TITLES[6], met, looked, found)


def ceiling_refusals(records: Records):
    """Every recorded refusal by the AGENT bound, in each form a record could carry it.

    Three forms, because the runtime does not write one yet: as of 2026-09-13 the agent bound is
    reported to the watch stream and the tracing log only (`trigger.rs::report_capped`), so no
    finished swarm's records name it and this clause is not met by any run this runtime can
    currently produce. That is the honest verdict, and it is the reason the forms are enumerated
    here rather than guessed at later.
    """
    out = []

    def from_sentence(text, where):
        match = AGENT_CEILING.search(text) if isinstance(text, str) else None
        if match:
            out.append((match.group("agent"), where))

    for row in records.capped:
        bound = row.get("bound")
        agent = None
        if isinstance(bound, dict):
            agent = bound.get("agent")
        elif bound == "agent":
            agent = row.get("agent")
        if isinstance(agent, str) and agent:
            out.append((agent, "turns/capped.jsonl"))
        else:
            from_sentence(row.get("why"), "turns/capped.jsonl")

    for row in records.spend:
        for key in ("note", "why", "error"):
            from_sentence(row.get(key), "turns/spend.jsonl")

    for event in records.events:
        from_sentence(json.dumps(event.data), f"`{event.name}` at seq {event.seq}")

    return out


def clause_seven(records: Records) -> Clause:
    looked = (
        "a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the "
        "sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — "
        "naming one of at least two agents an `AgentSpawned` says the swarm had"
    )
    # `spawned`, which is now the only definition of an agent in this file. This used to read a
    # union of spawned and spend-named agents, so one set of records got two verdicts about who
    # existed: clause 6 refused `ghost` as named by no `AgentSpawned` while clause 7 counted it
    # as one of the "at least two agents the swarm had to bound".
    known = set(records.spawned)
    ceilings = ceiling_refusals(records)
    refused = [(agent, where) for agent, where in ceilings if agent in known]
    unknown = [(agent, where) for agent, where in ceilings if agent not in known]
    met = bool(refused) and len(known) >= 2
    parts = [f"{len(known)} spawned agent(s) to bound: " + (listed(sorted(known)) or "none")]
    if refused:
        parts.append(
            "refused: " + listed(f"`{agent}` ({where})" for agent, where in refused)
        )
    elif unknown:
        parts.append(
            "a ceiling refusal names an agent no `swarm.agent.AgentSpawned` named: "
            + listed(f"`{agent}` ({where})" for agent, where in unknown)
        )
    else:
        parts.append(
            "no refusal by the agent ceiling in the event log, in spend.jsonl or in capped.jsonl"
        )
    return Clause(7, TITLES[7], met, looked, "; ".join(parts))


def unattended(records: Records) -> Clause:
    """The story's unattended condition — **undeterminable from this runtime's log**.

    "No operator input between the swarm starting and a verdict being recorded" is a fact about
    whose hands were on the swarm, and this log does not record that. `store.rs:192` writes
    `actor.unwrap_or("system")` and `http.rs:38` makes `actor` an optional field of the command
    request body, so an operator who pauses a swarm through the HTTP surface leaves an event
    indistinguishable from one the loop wrote.

    Measured rather than argued: `data/swarms/dsfsdf` is the swarm the story itself names as
    hand-driven, and inside its window it carries 8 `SwarmPaused`, 4 `SwarmResumed`, 7
    `SwarmStopped` and 6 further `SwarmStarted`, **every one of them actor `system`**. An earlier
    revision of this function read that as unattended and let it decide the exit status.

    So the answer is not a better rule, it is an honest refusal: never met, never counted against
    the seven, and the actors are reported as information rather than as a verdict. When
    `story:an-event-cannot-say-which-agent-acted` closes, this becomes checkable and comes back.
    """
    looked = (
        "whether anybody's hands were on this swarm between its start and its last event — which "
        "this log cannot say: `store.rs:192` writes `actor.unwrap_or(\"system\")` and "
        "`http.rs:38` makes the actor optional, so an operator's command and the loop's own are "
        "recorded identically"
    )
    why = (
        "UNDETERMINABLE, and reported as such rather than guessed: no actor in this log "
        "distinguishes a person from the loop. This does not count for or against the seven"
    )
    if records.log_error:
        return Clause(0, "unattended", False, looked, f"{why}; {records.log_error}", False)
    starts = records.named(STARTED)
    window = (
        [event for event in records.events if event.seq >= starts[0].seq] if starts else []
    )
    if not starts:
        seen = f"no `{STARTED}` in {len(records.events)} event(s), so there is no window to read"
    else:
        # Operator-issued commands, if any were, are in here somewhere — this is what a reader
        # gets instead of an answer: the actors, and the manager events a person usually causes.
        by_hand = [
            event
            for event in window
            if event.name.startswith("swarm.manager.")
            and event.name != "swarm.manager.SwarmCreated"
        ]
        seen = (
            f"{len(window)} event(s) from seq {starts[0].seq}, actors: "
            + listed(sorted({event.actor for event in window}))
            + f"; `swarm.manager.*` lifecycle events in that window: {len(by_hand)} — an "
            "operator and the loop can each cause one and the record names neither"
        )
    return Clause(0, "unattended", False, looked, f"{why}. {seen}", False)


def check(root) -> Report:
    """The seven clauses and the unattended condition, for one swarm's records."""
    records = read(root)
    clauses = [
        clause_one(records),
        clause_two(records),
        clause_three(records),
        clause_four(records),
        clause_five(records),
        clause_six(records),
        clause_seven(records),
    ]
    assert [clause.number for clause in clauses] == [1, 2, 3, 4, 5, 6, 7]
    return Report(swarm=Path(root), clauses=clauses, unattended=unattended(records))


def wrapped(label: str, text: str) -> str:
    return textwrap.fill(
        f"{label} {text}",
        width=98,
        initial_indent=" " * 10,
        subsequent_indent=" " * 18,
    )


def render(report: Report) -> str:
    lines = [f"swarm: {report.swarm}", ""]
    for clause in report.clauses:
        lines.append(
            f"clause {clause.number}  {'met    ' if clause.met else 'NOT MET'}  {clause.title}"
        )
        if not clause.met:
            lines.append(wrapped("wanted:", clause.looked_for))
        lines.append(wrapped("found: ", clause.found))
    lines.append("")
    lines.append("unattended  UNDETERMINABLE  no operator input in the window")
    lines.append(wrapped("wanted:", report.unattended.looked_for))
    lines.append(wrapped("found: ", report.unattended.found))
    lines.append("")
    missed = [clause.number for clause in report.clauses if not clause.met]
    summary = f"7 clauses: {7 - len(missed)} met, {len(missed)} not met"
    if missed:
        summary += " — " + ", ".join(f"clause {number}" for number in missed)
    summary += "; the unattended condition is undeterminable and decides nothing"
    lines.append(summary)
    return "\n".join(lines)


def as_json(report: Report) -> str:
    return json.dumps(
        {
            "swarm": str(report.swarm),
            "clauses": [vars(clause) for clause in report.clauses],
            "unattended": vars(report.unattended),
            "met": sum(1 for clause in report.clauses if clause.met),
            "not_met": sum(1 for clause in report.clauses if not clause.met),
            "ok": report.ok,
        },
        indent=2,
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Report the seven acceptance clauses of "
        "story:a-recorded-run-shows-two-agents-working against a finished swarm's records.",
    )
    parser.add_argument("swarm", help="a path to a swarm's directory, or its slug")
    parser.add_argument(
        "--data",
        default="data/swarms",
        help="where swarms live, when the first argument is a slug (default: data/swarms)",
    )
    parser.add_argument("--json", action="store_true", help="the report as JSON")
    args = parser.parse_args(argv)

    root = Path(args.swarm)
    if not root.is_dir():
        by_slug = Path(args.data) / args.swarm
        if by_slug.is_dir():
            root = by_slug

    report = check(root)
    print(as_json(report) if args.json else render(report))
    return 0 if report.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())

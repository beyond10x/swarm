#!/usr/bin/env python3
"""Fixture swarms: the records a run leaves, written without a run.

A real two-agent run costs real money — one turn on `claude-opus-5[1m]` cost $0.070 on 2026-09-13,
and `--max-budget-usd` is not a pre-spend bound, so a probe capped at $0.01 ended at $0.071. A
checker nobody has seen fail is a checker that reports met, and seeing it fail seven ways at $0.07 a
look is not a way to develop one.

So this writes the same three things a finished swarm leaves behind, from nothing:

    <root>/eventlog.sqlite3     table `swarm_events`, the real schema, read out of a real log
    <root>/turns/*.jsonl        metaharness event records, one file per turn attempt
    <root>/turns/spend.jsonl    one JSON object per attempt, carrying the agent since 2026-09-12

`build(root)` with no overrides writes a swarm that meets every clause. Every keyword argument
takes exactly one clause the other way, so a test that wants clause 4 unmet says `finished=None`
and changes nothing else. That is the whole design: the negative fixtures are the positive one
minus one fact.

By hand:

    python3 examples/two-agents/fixtures.py <dir>              # meets all seven
    python3 examples/two-agents/fixtures.py <dir> --unmet 4    # meets six
    python3 examples/two-agents/check-two-agents.py <dir>
"""

from __future__ import annotations

import argparse
import json
import shutil
import sqlite3
import uuid
from pathlib import Path

# Verbatim from `data/swarms/dsfsdf/eventlog.sqlite3`, so the checker's SQL is exercised against
# the table it will actually meet rather than a convenient subset of it.
SCHEMA = """CREATE TABLE swarm_events (
                 global_seq INTEGER PRIMARY KEY AUTOINCREMENT,
                 committed_xid INTEGER NOT NULL DEFAULT 0,
                 tenant_id TEXT NOT NULL,
                 stream_type TEXT NOT NULL,
                 stream_id TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 event_id TEXT NOT NULL,
                 event_name TEXT NOT NULL,
                 event_schema_version INTEGER NOT NULL,
                 occurred_at TEXT NOT NULL,
                 recorded_at TEXT NOT NULL,
                 subject TEXT NOT NULL,
                 actor TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 trace_id TEXT NOT NULL,
                 causation_id TEXT,
                 causation_depth INTEGER NOT NULL DEFAULT 0,
                 redacted_at TEXT,
                 data TEXT NOT NULL,
                 UNIQUE (tenant_id, stream_type, stream_id, version)
             )"""

SWARM_ID = "083b1264-b000-4fd6-a656-c07181e79796"
GOAL_ID = "73a4ac74-c4b6-4833-a3af-f7a1b4a9be95"
GOAL8 = GOAL_ID[:8]
ASSIGNMENT_ID = "5f1d1f5e-2a1b-4c3d-8e9f-0a1b2c3d4e5f"

COORDINATOR = "coordinator"
BUILDER = "builder"

# `Capped::why()` for `Bound::Agent`, verbatim in shape — `src/runtime/swarm-server/src/budget.rs`.
AGENT_CEILING_SENTENCE = (
    "the turn cap was reached: 4 of 3, by the agent `{agent}` across every goal it works. "
    "This goal is within its own bounds; raise SWARM_MAX_TURNS, or stop one of the agent's "
    "other goals"
)


class EventLog:
    """The append side of `swarm_events`, with the bookkeeping columns filled the way the real
    store fills them. Nothing here reads; the checker does that."""

    def __init__(self, path: Path) -> None:
        self.db = sqlite3.connect(path)
        self.db.executescript(SCHEMA)
        self.version = 0

    def append(self, name: str, data: dict, actor: str = "system") -> None:
        self.version += 1
        at = f"2026-09-13T12:{self.version:02d}:00Z"
        self.db.execute(
            "insert into swarm_events (committed_xid, tenant_id, stream_type, stream_id, "
            "version, event_id, event_name, event_schema_version, occurred_at, recorded_at, "
            "subject, actor, request_id, trace_id, data) "
            "values (0, 'swarm', 'swarm', ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?)",
            (
                SWARM_ID,
                self.version,
                str(uuid.uuid4()),
                name,
                at,
                at,
                SWARM_ID,
                actor,
                str(uuid.uuid4()),
                str(uuid.uuid4()),
                json.dumps(data, sort_keys=True),
            ),
        )

    def close(self) -> None:
        self.db.commit()
        self.db.close()


def turn_record(agent: str, *, refusal: str | None, attributed: bool, cwd: str) -> list[dict]:
    """One turn's metaharness event records.

    The shapes are those of a real run: `tool.requested` carries `operations`, `tool.decided`
    carries `{"decision": {"decision": ...}, "decided_by": ...}`, and a refused call leaves a
    `tool.result` behind only when it was allowed.
    """
    started = {
        "format": "metaharness.event/1",
        "seq": 1,
        "event": "session.started",
        "adapter": "claude",
        "model": "claude-opus-5[1m]",
        "cwd": cwd,
        "hermetic": {"mode": "on", "decisions": "frame"},
    }
    if attributed:
        started["agent"] = agent
    records = [
        started,
        {
            "format": "metaharness.event/1",
            "seq": 2,
            "event": "tool.requested",
            "call_id": "toolu_allowed",
            "name": "Read",
            "input": {"file_path": f"{cwd}/NOTES.md"},
            "operations": ["file.read"],
        },
        {
            "format": "metaharness.event/1",
            "seq": 3,
            "event": "tool.decided",
            "call_id": "toolu_allowed",
            "decision": {"decision": "allow"},
            "decided_by": "frame",
        },
    ]
    if refusal is not None:
        records += [
            {
                "format": "metaharness.event/1",
                "seq": 4,
                "event": "tool.requested",
                "call_id": "toolu_denied",
                "name": "Bash",
                "input": {"command": "echo hello-from-inside"},
                "operations": ["shell"],
            },
            {
                "format": "metaharness.event/1",
                "seq": 5,
                "event": "tool.decided",
                "call_id": "toolu_denied",
                "decision": {
                    "decision": "deny",
                    "reason": "this step only admits `file.read` operations",
                },
                "decided_by": refusal,
            },
        ]
    # The census counts DECISIONS, not denials, and `by_decider` counts every decider that
    # decided anything — read off `data/swarms/dsfsdf/turns/0035-77fc1fcc.jsonl`, which carries
    # `{"allowed": 5, "denied": 0, "by_decider": {"observe": 5}}` for five allowed calls. An
    # earlier revision of this file wrote only the decider that DENIED, which was the checker's
    # own idea of the schema; it made a wrong fallback in the checker look right, and it was the
    # only thing keeping a negative case green.
    deciders: dict = {"frame": 1}  # the `Read` call above, allowed by the frame
    if refusal is not None:
        deciders[refusal] = deciders.get(refusal, 0) + 1
    records.append(
        {
            "format": "metaharness.event/1",
            "seq": 6,
            "event": "session.ended",
            "census": {
                "allowed": 1,
                "denied": 1 if refusal else 0,
                "replaced": 0,
                "abstained": 0,
                "by_seam": {"hook": 2 if refusal else 1},
                "by_decider": deciders,
            },
        }
    )
    return records


def build(
    root,
    *,
    roles=((COORDINATOR, "Coordinator"), (BUILDER, "Worker")),
    posted_to=BUILDER,
    taken_by=BUILDER,
    take_before_post=False,
    finished="assignment",
    finished_by=None,
    turns=((1, COORDINATOR), (2, BUILDER)),
    attribution="spend",
    extra_attempts=(),
    refusal="frame",
    spend=None,
    unattributed=0,
    ceiling="note",
    started=True,
    human_actor=None,
    eventlog=True,
) -> Path:
    """Write a swarm's records under `root`.

    With no overrides the swarm meets every clause. Each argument is one clause's fact:

      roles           clause 1 — `roles=((COORDINATOR, "Coordinator"),)` leaves no second agent
      posted_to,      clause 2 — `taken_by=None` is an assignment nobody took, which is the
      taken_by,                  default state of the world: `AssignmentTaken` has fired zero
      take_before_post           times in this repository's history
      turns,          clause 3 — `turns=((1, COORDINATOR),)` is one transcript; `extra_attempts`
      attribution,               adds spend rows with no file beside them, which is what an
      extra_attempts             overwritten record looks like after the fact
      finished,       clause 4 — "assignment" | "gate" | None; `finished_by` is who reports the
      finished_by                green gate, and a coordinator reporting its own is the shape
                                 the clause now refuses
      refusal         clause 5 — "frame" | "user" (a denial the frame did not make) | None
      spend,          clause 6 — `spend=[(1, COORDINATOR)]` names one agent; `unattributed`
      unattributed               adds rows that name none
      ceiling         clause 7 — "note" | "capped-file" | "spend-note" | None
      started,        the unattended condition — `human_actor="timo@example.com"` is an operator
      human_actor                reaching into the window
      eventlog        no log at all, which every clause must survive
    """
    root = Path(root)
    if root.exists():
        shutil.rmtree(root)
    (root / "turns").mkdir(parents=True)

    if eventlog:
        log = EventLog(root / "eventlog.sqlite3")
        log.append(
            "swarm.manager.SwarmCreated",
            {"swarm_id": SWARM_ID, "display_name": "Two agents", "home": str(root)},
        )
        log.append(
            "swarm.goal.GoalSet",
            {"goal_id": GOAL_ID, "swarm_id": SWARM_ID, "text": "hand one unit of work over"},
        )
        if started:
            log.append("swarm.manager.SwarmStarted", {"swarm_id": SWARM_ID})
        for agent, role in roles:
            log.append(
                "swarm.agent.AgentSpawned",
                {
                    "agent_id": agent,
                    "swarm_id": SWARM_ID,
                    "role": role,
                    "harness": "ClaudeCode",
                    "display_name": agent,
                    "host": {},
                },
            )

        def post():
            log.append(
                "swarm.agent.AssignmentPosted",
                {
                    "agent_id": posted_to,
                    "outcome": "the checker reports seven clauses",
                    "signoff": "the suite is green",
                    "forbidden": "editing another unit's files",
                    "artifact_ref": None,
                },
                actor="swarm.agent.SwarmAgent",
            )
            log.append(
                "swarm.agent.AssignmentRecorded",
                {
                    "assignment_id": ASSIGNMENT_ID,
                    "agent_id": posted_to,
                    "outcome": "the checker reports seven clauses",
                    "signoff": "the suite is green",
                    "forbidden": "editing another unit's files",
                    "artifact_ref": None,
                },
                actor="swarm.agent.SwarmAgent",
            )

        def take():
            log.append(
                "swarm.agent.AssignmentTaken",
                {"agent_id": taken_by, "ref": ASSIGNMENT_ID, "msg": "taken"},
                actor="swarm.agent.SwarmAgent",
            )

        if take_before_post and taken_by:
            take()
        if posted_to:
            post()
        if taken_by and not take_before_post:
            take()

        if finished == "assignment":
            log.append(
                "swarm.agent.AssignmentDone",
                {"assignment_id": ASSIGNMENT_ID, "note": "the seven are reported"},
                actor="swarm.agent.SwarmAgent",
            )
        elif finished == "gate":
            log.append(
                "swarm.agent.GateGreen",
                {"agent_id": finished_by or taken_by or BUILDER, "ref": ASSIGNMENT_ID,
                 "exit_code": 0, "msg": "python3 -m unittest: OK"},
                actor="swarm.agent.SwarmAgent",
            )

        if ceiling == "note":
            log.append(
                "swarm.agent.NoteEmitted",
                {
                    "agent_id": BUILDER,
                    "msg": AGENT_CEILING_SENTENCE.format(agent=BUILDER),
                },
                actor="swarm.agent.SwarmAgent",
            )
        if human_actor:
            log.append(
                "swarm.agent.NoteEmitted",
                {"agent_id": COORDINATOR, "msg": "the operator said carry on"},
                actor=human_actor,
            )
        log.close()

    for turn, agent in turns:
        name = f"{turn:04d}-01-{GOAL8}"
        if attribution == "name":
            name = f"{name}-{agent}"
        cwd = str(root / "work")
        if attribution == "cwd":
            cwd = str(root / "work" / agent)
        records = turn_record(
            agent,
            refusal=refusal if agent != COORDINATOR or len(turns) == 1 else None,
            attributed=attribution == "record",
            cwd=cwd,
        )
        (root / "turns" / f"{name}.jsonl").write_text(
            "".join(json.dumps(record) + "\n" for record in records)
        )

    rows = []
    for turn, agent in list(spend if spend is not None else turns) + list(extra_attempts):
        rows.append(
            {
                "at": f"2026-09-13T12:{turn:02d}:30Z",
                "agent": agent,
                "goal": GOAL_ID,
                "iterations": turn,
                "reached": False,
                "note": AGENT_CEILING_SENTENCE.format(agent=agent)
                if ceiling == "spend-note" and agent == BUILDER
                else None,
                "finished": True,
                "spent": {"cost_usd": 0.07, "model": "claude-opus-5", "requests": 3},
            }
        )
    for index in range(unattributed):
        rows.append(
            {
                "at": f"2026-09-13T12:5{index}:30Z",
                "agent": None,
                "goal": GOAL_ID,
                "iterations": 90 + index,
                "reached": None,
                "spent": {"cost_usd": 0.07},
            }
        )
    if rows:
        (root / "turns" / "spend.jsonl").write_text(
            "".join(json.dumps(row) + "\n" for row in rows)
        )

    if ceiling == "capped-file":
        (root / "turns" / "capped.jsonl").write_text(
            json.dumps(
                {
                    "at": "2026-09-13T12:40:00Z",
                    "bound": {"agent": BUILDER},
                    "goal": GOAL_ID,
                    "turns": 4,
                    "spent_usd": 0.28,
                    "why": AGENT_CEILING_SENTENCE.format(agent=BUILDER),
                }
            )
            + "\n"
        )

    return root


# One override per clause, each chosen so that it takes THAT clause the other way and leaves the
# rest standing. Two of them carry a second override for exactly that reason: clause 3's one
# transcript would otherwise take clause 6's second spend row with it, and clause 6's single spend
# row would otherwise take away the attribution clause 3 reads. `COUPLED` below names the one pair
# that cannot be separated.
UNMET = {
    1: {"roles": ((COORDINATOR, "Coordinator"), (BUILDER, "Coordinator"))},
    2: {"taken_by": None},
    3: {"turns": ((1, COORDINATOR),), "spend": [(1, COORDINATOR), (2, BUILDER)]},
    4: {"finished": None},
    5: {"refusal": None},
    6: {"spend": [(1, COORDINATOR)], "attribution": "record"},
    7: {"ceiling": None},
}

# Two couplings, both of them in the acceptance rather than in the code. Clause 2 wants an
# `AssignmentTaken` by the non-Coordinator agent of clause 1, so a swarm with no such agent
# cannot meet it. Clause 4 is now "finished BY THE AGENT THAT TOOK IT, for the assignment
# clause 2 matched", so a swarm where nothing was taken cannot meet that either. This map is
# what a test asserts against rather than a sentence in a docstring.
COUPLED = {1: {2, 4}, 2: {4}}


def main() -> int:
    parser = argparse.ArgumentParser(description="Write a fixture swarm without running one.")
    parser.add_argument("root", help="the directory to write the swarm's records into")
    parser.add_argument(
        "--unmet",
        type=int,
        choices=sorted(UNMET),
        action="append",
        default=[],
        help="leave this clause unmet; repeatable",
    )
    args = parser.parse_args()
    overrides: dict = {}
    for clause in args.unmet:
        overrides.update(UNMET[clause])
    root = build(args.root, **overrides)
    unmet = ", ".join(str(clause) for clause in sorted(args.unmet)) or "none"
    print(f"{root}: a fixture swarm, clauses left unmet: {unmet}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

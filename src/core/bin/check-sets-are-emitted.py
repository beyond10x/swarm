#!/usr/bin/env python3
"""Refuse a spec whose events cannot rebuild its state.

The runtime is event-sourced: an entity is a fold over the events its commands emitted, and a
projection is rebuilt by replaying them. So an outcome that writes a field without putting that
field on an event writes a fact the log cannot recover, and an event that does not carry its
subject's identity is a fact nobody can attribute.

Neither rule is one ess/1 enforces — ESS is a specification format and says nothing about how state
is stored. They are this system's rules, and they are not retrofittable: once a log exists, the
events in it are already missing whatever they were allowed to omit.

Two checks, both over `ess specify compile --format json`:

  1. every `sets:` target appears as a payload field on one of the outcome's emitted events
  2. every emitted event carries the subject's identity field

Exits 1 and names each violation as `command.outcome`.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

SPEC = Path(__file__).resolve().parent.parent


def compile_ir(path: Path) -> dict:
    done = subprocess.run(
        ["ess", "specify", "compile", "--path", str(path), "--format", "json"],
        capture_output=True,
        text=True,
    )
    if done.returncode != 0:
        sys.exit(f"ess specify compile refused {path}:\n{done.stderr}")
    return json.loads(done.stdout)


def identity_of(ir: dict, entity: str) -> str | None:
    declared = ir.get("entities", {}).get(entity)
    return declared["identity"]["name"] if declared else None


def check(ir: dict) -> list[str]:
    faults: list[str] = []

    for command in ir.get("commands", {}).values():
        for outcome in command.get("outcomes", []):
            where = f"{command['name']}.{outcome['name']}"

            # what each emitted event carries
            carried: dict[str, set[str]] = {
                entry["event"]: {field["target"] for field in entry.get("fields", [])}
                for entry in outcome.get("payload", [])
            }
            emitted = outcome.get("emits", [])
            anywhere: set[str] = set().union(*carried.values()) if carried else set()

            # 1. a field that is written and never emitted is unrecoverable
            for written in outcome.get("sets", []):
                target = written["target"]
                if target not in anywhere:
                    faults.append(
                        f"{where}: sets `{target}`, which no emitted event carries — "
                        f"a replay cannot recover it"
                    )

            # 2. an event that does not say whose it is cannot be attributed
            subject = outcome.get("subject")
            if not subject:
                continue
            identity = identity_of(ir, subject["entity"])
            if identity is None:
                continue
            for event in emitted:
                # `observed` publishes the new identity in this event by construction
                instance = subject.get("instance", {})
                if (
                    instance.get("from") == "observed"
                    and instance.get("event") == event
                ):
                    continue
                if identity not in carried.get(event, set()):
                    faults.append(
                        f"{where}: emits `{event}` without `{identity}`, so the event "
                        f"cannot be attributed to a {subject['entity']}"
                    )

    return faults


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else SPEC
    faults = check(compile_ir(path))
    if not faults:
        print(f"{path}: every written field is emitted, every event is attributable")
        return 0
    print(f"{path}: {len(faults)} violation(s)\n", file=sys.stderr)
    for fault in faults:
        print(f"  {fault}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())

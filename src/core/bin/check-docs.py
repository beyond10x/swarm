#!/usr/bin/env python3
"""Validate a swarm's instance documents against the JSON Schemas the specification projects.

Data does not live in the specification tree. It lives in `data/swarms/<slug>/`, one directory per
swarm, and each document conforms to the schema `ess generate --kind schema` projects from the
entity it instantiates. This walks every swarm and checks every document.

An empty store is valid: a swarm that has not been created yet has nothing to conform.

Usage:
    bin/check-docs.py [--data DIR] [--schema-dir DIR]

Regenerate the schemas first:
    ess generate --path src/core --kind schema --out src/core/generated/schema
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import jsonschema
import yaml

SPEC = Path(__file__).resolve().parent.parent
REPO = SPEC.parent.parent

# where a document sits inside one swarm's directory -> the entity it instantiates
LAYOUT = {
    "swarm.yaml": "swarm.manager.Swarm",
    "goal.yaml": "swarm.goal.Goal",
    "config.yaml": "swarm.config.Config",
    "agents/*.yaml": "swarm.agent.Agent",
    "boxes/*.yaml": "swarm.blackbox.Box",
    "connections/*.yaml": "swarm.blackbox.Connection",
    "schemas/*.yaml": "swarm.blackbox.Schema",
}


def validator_for(schema_dir: Path, entity: str):
    path = schema_dir / "entities" / f"{entity}.schema.json"
    if not path.exists():
        return None, f"no schema for {entity} at {path} — run `ess generate --kind schema` first"
    schema = json.loads(path.read_text())
    return jsonschema.validators.validator_for(schema)(schema), None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", type=Path, default=REPO / "data")
    ap.add_argument("--schema-dir", type=Path, default=SPEC / "generated" / "schema" / "schema")
    args = ap.parse_args()

    swarms = sorted((args.data / "swarms").glob("*/")) if (args.data / "swarms").is_dir() else []
    if not swarms:
        print(f"{args.data}: no swarms yet — nothing to conform")
        return 0

    cache: dict[str, object] = {}
    checked = failed = 0

    for swarm in swarms:
        for pattern, entity in LAYOUT.items():
            for doc in sorted(swarm.glob(pattern)):
                if entity not in cache:
                    made, why = validator_for(args.schema_dir, entity)
                    if made is None:
                        print(why, file=sys.stderr)
                        return 1
                    cache[entity] = made
                rel = doc.relative_to(args.data)
                errors = list(cache[entity].iter_errors(yaml.safe_load(doc.read_text())))
                checked += 1
                if not errors:
                    print(f"{rel}: valid against {entity}")
                    continue
                failed += 1
                print(f"{rel}: {len(errors)} error(s) against {entity}", file=sys.stderr)
                for e in errors[:10]:
                    where = "/".join(str(p) for p in e.path) or "<root>"
                    print(f"  - {where}: {e.message[:180]}", file=sys.stderr)

    print(f"\n{checked} document(s), {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Validate swarm instance documents (config/*.yaml, flows/*.yaml) against the JSON Schemas
`ess generate --kind schema` projects from the specification. Zero model tokens; exit 1 on any error.
Usage: bin/check-docs.py [--schema-dir generated/schema/schema]"""
import glob, json, os, sys
import yaml, jsonschema
here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
schema_dir = os.path.join(here, "generated", "schema", "schema")
if len(sys.argv) > 2 and sys.argv[1] == "--schema-dir":
    schema_dir = sys.argv[2]
targets = {"config": "swarm.config.Config", "flows": "swarm.flow.Flow"}
failed = 0
for sub, entity in targets.items():
    sf = os.path.join(schema_dir, "entities", f"{entity}.schema.json")
    if not os.path.exists(sf):
        print(f"no schema for {entity}: {sf}"); failed += 1; continue
    schema = json.load(open(sf))
    validator = jsonschema.validators.validator_for(schema)(schema)
    for doc in sorted(glob.glob(os.path.join(here, sub, "*.yaml"))):
        data = yaml.safe_load(open(doc))
        errs = list(validator.iter_errors(data))
        rel = os.path.relpath(doc, here)
        if errs:
            failed += 1
            print(f"{rel}: {len(errs)} error(s) against {entity}")
            for e in errs[:10]:
                print(f"  - {'/'.join(str(p) for p in e.path) or '<root>'}: {e.message[:180]}")
        else:
            print(f"{rel}: valid against {entity}")
sys.exit(1 if failed else 0)

# data

Instance documents, one directory per swarm. Nothing here is specification — the specification is in
`../src/core/`, and every document here conforms to a schema projected from it.

```
data/
  swarms/
    <slug>/
      eventlog.sqlite3     the record. State is a fold over this; everything else is a projection.
      swarm.yaml           swarm.manager.Swarm
      goal.yaml            swarm.goal.Goal
      config.yaml          swarm.config.Config
      agents/*.yaml        swarm.agent.Agent
      boxes/*.yaml         swarm.blackbox.Box
      connections/*.yaml   swarm.blackbox.Connection
      schemas/*.yaml       swarm.blackbox.Schema
  generated/               projections; gitignored
```

Check every document against the specification:

```
ess generate --path src/core --kind schema --out src/core/generated/schema
src/core/bin/check-docs.py
```

The event log is the authority. The YAML documents beside it are a readable projection of the same
facts, not a second source — if the two disagree, the log is right and the projection is stale.

A swarm starts with nothing here but its own directory. It is not seeded with boxes, flows or
agents; it is created, given a goal, and started, and what it becomes it builds.

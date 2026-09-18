# swarm

A working agent swarm. Its behaviour is specified in ESS and the specification is the source of
truth; a small Rust runtime serves it, and a Vue canvas UI shows it.

| part | where | what it is |
|---|---|---|
| the specification | `src/core/` | an Executable System Specification (`ess/4`, validated with `ess 0.23.0`) of a swarm manager: 6 domains, 10 entities, 53 commands, 56 events, 27 views |
| the runtime | `src/runtime/` | `ess-runtime`, an interpreter for any ESS specification; `swarm-server`, which serves this one over HTTP; and `swarm`, the command line a swarm's own agents use to reach it |
| the UI | `src/web/` | a Vue canvas over the served swarm |
| instance data | `data/` | one directory per swarm; the event log is the authority and the YAML beside it is a projection |

A swarm starts bare — one coordinator, no data, one goal — and builds what it needs. `src/core/` is
the kernel: only what a swarm cannot build for itself. Everything else is a **Box** the swarm draws
and a **Connection** it wires. A tool is a box, an MCP server is a box, a UI panel is a box. Two
boxes connect only when their ports cite the same published schema.

That drawing describes; it does not decide. Boxes and connections are declared, created and
recorded, and no part of the runtime reads one: no message travels a connection and no box runs a
program. What a coordinator may reach is settled per call by the sealed frame its turn launches
under, which never consults the canvas. Wiring as the grant is the model's intent, and is not built
yet.

## Read next

- [The specified kernel](https://github.com/beyond10x/swarm/blob/main/src/core/README.md) — what is
  in the kernel and why nothing else is, the declared loop, and the two rules this system adds to
  `ess/4`.
- [What ESS accepts and refuses](https://github.com/beyond10x/swarm/blob/main/src/core/docs/ess-authoring-rules.md),
  as probed while authoring this specification.
- [Instance data layout](https://github.com/beyond10x/swarm/blob/main/data/README.md) — the
  documents one swarm keeps, and how to check them against the specification.

## Status

Version 0.1.0, unreleased. The repository is private and this surface is declared `planned` and
`internal`: it is recorded in the organization catalog and is not collected or published by the
Website.

# Swarm

**A swarm manager whose kernel is written down rather than programmed.**

Most software describes its rules twice: once in a design document nobody updates, and once in the
code that actually runs. Swarm keeps one copy. The rules live in a set of YAML files, and the
runtime reads those files when it starts and executes them directly. There is no step in between —
nothing is generated, nothing is scaffolded, nothing is hand-written per feature. Add a command to
the specification, restart, and it is there: over HTTP, in the command line tool, and in the web
canvas, without anyone having written a line of code for it.

That is the whole idea, and it is worth being precise about, because it is easy to mistake for
something more ordinary. This is not a code generator. There is no generated output anywhere in the
repository. The runtime is an **interpreter**: it holds the compiled specification in memory and
works against it the way a browser works against a page. Which means the usual problem with
generated systems — the moment the generated code and its source drift apart — cannot happen, because
there is nothing generated to drift.

## What it manages

A *swarm* is an AI coordinator pursuing a goal, over and over, until the goal is reached.

It starts with almost nothing: a loop, a coordinator, and a goal. Every thirty seconds the loop
turns. The coordinator wakes up, works, and reports back one of two things — "reached" or "not yet".
If it says "not yet", the goal goes back to waiting and the loop turns again. If it says "reached",
the loop stops.

While a turn is running you can watch it. The coordinator's output streams to disk and to every
open browser at once, line by line — every tool it calls, every token it spends, what it costs, as
it happens rather than afterwards. A turn that dies halfway leaves its goal visibly waiting, because
the runtime will not fill in a verdict on the coordinator's behalf. Software that reports work
nobody did is worse than software that stops.

Everything a swarm needs beyond that bare start, it draws for itself. A tool is a box. A data source
is a box. A panel in the UI is a box. Boxes connect when their ports agree on a shape. Giving a
coordinator access to something is drawing an edge to it; taking it away is deleting the edge.

## What actually runs today

Honestly, and in that order:

**The manager works.** You can create swarms, give them goals, start and pause and stop them, watch
turns run, and read the whole history back. There are real recorded coordinator runs sitting in this
repository. The server, the command line tool and the web UI all work.

**The specification is genuinely the source of truth.** The server compiles it at boot and refuses
to start if it does not resolve — it will not limp along with a broken model. The web UI asks the
server what entities exist rather than knowing in advance, so adding one to the YAML makes it appear
on the canvas.

**The multi-agent part is written down but not yet demonstrated.** This is the honest caveat and it
is a large one. The specification describes spawning agents, assigning them work, and a full
mailbox system for them to talk to each other — and none of that has been shown running. What works
today is *one* coordinator per swarm, pursuing *one* goal. Getting a second agent spawned is an open
piece of work, not a finished one. If you came here for a working multi-agent system, this is not
that yet.

**Nothing sandboxes the coordinator.** It runs on your machine with the same access you have. It is
launched in a controlled, fully recorded way, which is not the same as being contained. Run it
somewhere you would be comfortable letting an AI agent run.

## The shape of it, in numbers

Six domains. Fifty-three commands. Twenty-seven views. About 4,800 lines of YAML describing what the
system is — against roughly 6,100 lines of runtime that knows nothing about swarms in particular and
would happily execute a different specification.

That ratio is the point of the project. The interesting part of the system is the part you can read.

## Trying it

You need Rust and Node. The short version:

```sh
cargo run -p swarm-server          # the server, on :5000
cd src/web && npm install && npm run dev   # the canvas, in a browser
```

Then create a swarm, give it a goal, and start it. Thirty seconds later the loop turns for the first
time and you can watch a coordinator think.

The full instructions — building, testing, how the pieces fit, and every check that has to pass
before a change lands — are in [`AGENTS.md`](AGENTS.md), which is written for the people and agents
working on the code.

## Where to look first

Start with [`src/core/`](src/core/). It is nine files of YAML, and it is the system. Everything else
in this repository exists to run it.

More at [swarm.beyond10x.dev](https://swarm.beyond10x.dev).

---

Licensed under Apache-2.0.

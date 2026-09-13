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

**The multi-agent part runs, at depth one and breadth two.** Until 2026-09-13 this paragraph said it
was written down and not demonstrated, which was true. What changed is a recorded run, not an
opinion: a coordinator was given a goal it could not meet alone, spawned a second agent, posted it an
assignment, and waited while the runtime ran that agent as its own session with its own work
directory and its own transcript. The run's evidence is in the repository at
`examples/two-agents/evidence/2026-09-13-two-agents-proof/` — the event log as CSV, the spend rows,
the ceiling refusal and the sealed frames the turns ran under. The log itself is runtime state and
is gitignored, which that directory says. Run
`cargo run -p swarm-check -- data/swarms/two-agents-proof` to check the retained runtime records.
The checker reports each of seven clauses independently —
`AssignmentTaken`, `AssignmentDone` and `GoalReached` among them, each of which had fired zero times
here before that run.

Be precise about the size of it: **one coordinator, one worker, one assignment.** Agents that spawn
agents, more than two at once, and a swarm that extends its own specification are all still
undemonstrated. If you came here for a swarm that grows itself, this is one step of that and not the
whole of it.

**A turn is narrowed, and the coordinator is still not sandboxed.** Both halves matter. Every turn now
launches under a sealed frame that names which operations it admits and which directory it may write,
so a tool call outside that set is refused when the model attempts it — measured in the run above: 1
of 41 decided calls refused by the frame. A write outside the agent's own directory is refused too,
and the rule is derived by the runtime rather than read from configuration, because a config an agent
writes for itself could widen its own boundary.

That is refusal at a decision seam. It is **not** containment: there is no namespace, no cgroup, no
network isolation, and the harness's own attestation says so. The process runs on your machine with
your access, and a refused call is a call the harness declined to make rather than one the kernel
stopped. Run it somewhere you would be comfortable letting an AI agent run.

## The shape of it, in numbers

Six domains. Fifty-three commands. Twenty-seven views. About 4,800 lines of YAML describing what the
system is — against roughly 10,800 lines of runtime in the interpreter and swarm host. The interpreter
knows nothing about swarms in particular and can execute a different specification.

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

More at [beyond10x.github.io/swarm](https://beyond10x.github.io/swarm/).

---

Licensed under Apache-2.0.

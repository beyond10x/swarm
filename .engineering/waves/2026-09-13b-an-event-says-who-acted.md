# Wave proposal — 2026-09-13b — an event says who acted

Coordinator: this session. Skill `aep-drive:wave` **0.9.1**. `aep` **0.55.0**.
Base `main` at `df71cba`. **Approved**: the operator approved the plan at
`~/.claude/plans/sprightly-twirling-gosling.md`, which names this story as step 2, and then said
to keep going.

## One unit, and why

`story:an-event-cannot-say-which-agent-acted`. Everything lands in `ess-runtime` and the two callers
that feed it; there is no disjoint second surface, so a second unit would be two agents in one crate.
A wave of one is a wave.

`aep plan artifact waves --kind story --status draft` now puts this story in **wave 1**, which it
could not do before the board was reconciled — the previous wave 1 was a story shipped in v0.2.0.

## Why this story, immediately after v0.2.0

v0.2.0's headline is that a swarm ran two agents, and its evidence is an event log. This is the seam
where that evidence stops being evidence: `permitted` (`ess-runtime/src/apply.rs:174-196`) matches
the actor string against the specification's actor **types**, and `TakeAssignment`'s `agent_id` is
caller-supplied input. So `AssignmentTaken` naming `worker` is indistinguishable in the record from
the coordinator, or a curl, issuing it on that agent's behalf. **A single-agent swarm can write a log
a reader cannot tell from a two-agent one.**

It is also the story that makes the demonstration's `unattended` clause answerable. It reports
UNDETERMINABLE today and decides nothing, by design.

## What the coordinator measured before writing the brief

- **`swarm-cli/src/main.rs:502` — `fn actor_for(command: &str, _agent: &str)`.** The agent slug is
  passed in and **discarded**; the function returns an actor *type* or `None`. The identity exists at
  the call site and is thrown away one line before the wire.
- **`ess-runtime/src/store.rs:192-193`** writes `actor.unwrap_or("system")` into **both** `subject`
  and `actor`. Two columns, one string — so the row already has somewhere to put a second fact.
- **`swarm-server/src/http.rs:38`** makes the request body's `actor` optional, which is why every
  operator command and every runtime command both record `system`.
- `permitted` skips the check entirely when a domain declares no matching actor — `actor_for`'s own
  doc calls that "the runtime's own behaviour, not a widening invented here".

## The decision the story left open, taken by the coordinator

The story's acceptance clause 2 asks whether a command issued by something that is not the named
agent is **refused** or **recorded distinguishably**.

**Recorded distinguishably. Not refused.** Three reasons, the first decisive:

1. **Refusing is authorisation, and the story's own Out of Scope excludes it** — *"This is about a
   record being able to say who acted, not about proving the claim against a credential."* To refuse,
   the runtime would have to know which instance may act as which type, which is a policy nobody has
   designed and which this story does not ask for.
2. **The coordinator legitimately acts on the swarm's behalf.** It issues `Spawn` and `Assign` for
   agents that are not itself; a refusal rule would have to carve that out, and carving it out is how
   the hole comes back.
3. **Distinguishability is what the open clauses need.** The demonstration's `unattended` condition
   needs to tell an operator's command from the loop's — not to forbid either.

So `permitted`'s actor-type check **stays exactly as it is**, and an acting identity is carried
beside it and recorded.

## Pre-flight

| check | result |
|---|---|
| working tree | clean on `main` at `df71cba`, pushed to `origin/main` |
| worktrees | none beyond the primary — the six from the previous two waves were removed after the push supplied recovery proof |
| free disk | **45G** |
| model budget | **79% of the weekly allowance used, resetting Sep 17.** This is the reason for one unit rather than two, and for no third attack pass under any circumstances |
| `AGENTS.md` | read. Its 11-step gate governs; `aep plan artifact move` is not run by this wave |

## The commits this authorises

The opening commit, one unit commit, one merge into the integration branch, the closing store commit,
and the merge into `main` once the gate is green. Not a push, not a tag, not a release.

## Stage

| unit | stage | branch head |
|---|---|---|
| A | not started | — |

---

## The close

**Closed 2026-09-13. Unit A merged at `77044a8`; all thirteen gate steps green, each exit status read
separately.**

| step | exit | |
|---|---|---|
| fmt · clippy · `cargo test` | 0 · 0 · 0 | **158 cases** across 30 lanes |
| `ess validate` · `check-sets` · `aep validate` | 0 · 0 · 0 | |
| checker suite · demonstration | 0 · 0 | **83 cases**; 7 of 7 |
| `npm test` · `vue-tsc` · `vite build` | 0 · 0 · 0 | 71 web cases |
| website build · `spec:check` | 0 · 0 | README and AGENTS.md agree |

Rust 145 → **158**. Checker 62 → **83**.

**Steps 7–9 were red on the first run and were not a defect**: `src/web/node_modules` was absent from
the integration tree, so `npm test` died on `ERR_MODULE_NOT_FOUND` for `@vue/compiler-sfc`. Staged and
re-run, all three exit 0. A missing dependency and a broken build are the same exit code, which is
why `AGENTS.md` says to read what a step printed rather than only what it returned.

### The ledger

| pass | findings | verdict |
|---|---|---|
| 1 | 8 — 6 fixed, 2 INFEASIBLE | acceptance clause 1 negated |
| 2 | 12 — 8 fixed, rest doc lines | acceptance clause 3 negated at the CLI |

Twelve findings across two passes, zero carried. **Three separately negated a clause the unit had
already reported met**, which is the honest summary of this wave: a story about making a log
trustworthy was itself reported trustworthy three times before it was.

### What the passes bought beyond the fixes

- **Clause 1 was negated by a reachable state, not a constructed one.** `StreamId` validates a
  *field* and the envelope validates an *identity*; a member whose id carries a space or an `@` passes
  the first and fails the second, so two such members recorded one envelope. Both spawns were
  accepted over the socket and both commands answered 200.
- **Clause 3 was negated at the door the repository documents.** `swarm --swarm <slug> do <command>`
  from any directory with no `.swarm/config.json` issued as `agent:coordinator`, reaching all
  fifty-three commands. The case that had covered that clause built a `Settings` value `run` cannot
  produce.
- **A guard that failed open.** The vocabulary check derived labels with a regex wanting a trailing
  comma, and `rustfmt` writes any long arm as a block — so a fifth variant was skipped in silence,
  which is the exact silent downgrade the guard's own docstring claims to prevent. It derives from
  the enum's variants now.
- **The adversary's own suggested fix was refused with a proof.** "Spell the unnameable form with a
  character `validate_identity` refuses" cannot work: the label goes *into* the identity column, so
  it must pass the validation whose gap it would exploit.

### The pattern, in the unit's words

> Each time I estimated the frequency of a reachable state and never computed the consequence — and
> the consequence was the same every time, because everything in this system feeds one checker.

The fix generalises rather than patching the third instance: `agent_named_by` asks
`swarm.agent.AgentSpawned` instead of parsing a prefix, so an agent is what the log says it is and
the next unanticipated form is not a new incident.

### What a person may do without an agent was not invented here

`src/core/domains/mailbox.yaml:609-616` already decided it — an Operator may post and may not read or
ack, because *"an ack the recipient did not perform is the one lie this domain exists to prevent"*.
So `read`, `ack` and `mailbox` refuse and name the flag; posting works and the sender is `you`, the
word `MailPanel.vue` was already using for a person.

### Coordinator debts, all cleared at integration

The evidence re-export (twice — the second after the unit's window change moved the block), its CRLF,
the derived line count 10,400 → **10,800**, and the sentence in
`verification-report:two-agents-under-a-frame-2026-09-13` calling the unattended condition
UNDETERMINABLE *by design*, which this wave made false. **The report's seven clauses are unchanged at
seven met**; what moved is that a later run can earn the condition instead of abstaining.

### Left open, in writing

- **`src/web` still renders `· by ${event.actor}`**, so the canvas shows two members as one string
  while the stream now distinguishes them. Patch at
  `~/.cache/swarm-wave-2026-09-13b/a/canvas-shows-the-issuer.patch`, unapplied — it is a surface no
  story in this wave owned.
- **`examples/two-agents/` is 3,650 lines of Python** against the 204 that were in the repository
  before it. The operator's standing preference is against Python; the port to a fourth crate is the
  next piece of work and is deliberately after this wave, so that it translates code pinned by 83
  passing cases rather than code carrying four known defects.

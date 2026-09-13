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

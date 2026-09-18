# Two workers at once, and the ceiling over them — 2026-09-18

What this directory holds is the **recorded run** behind `story:run-two-workers-at-once`: two
members of one swarm taking their turns at the same moment, a per-swarm ceiling on how many turns
may be in flight, and a swarm-wide spend cap refusing a swarm whose members are each inside their
own bounds.

## What was run, and what it is not

Every session below is a `Launch::Program` — the stdin→stdout contract
`examples/coordinator-manual.sh` satisfies — resolved from `SWARM_COORDINATOR`. **No vendor was
contacted and nothing was spent.** That is a real limit on what this run demonstrates, and it is the
same limit `a_second_agent_runs.rs` carries: what a real `metaharness run claude` does under a frame
is measured in `examples/two-agents/evidence/2026-09-13-two-agents-proof/`, for $0.224.

What IS the runtime's own path here: which turns are admitted, how many at once, which bound refuses
a turn before it launches, and what the swarm writes down about it.

    cargo test -p swarm-server --test two_workers_run_at_once -- --nocapture
    cargo test -p swarm-server --lib bounds::a_swarm_over_its_total -- --nocapture

Anyone can re-derive both. Neither needs a network, a key or a dollar.

## Two workers, at the same moment

    running 3 tests
    test one_swarms_ceiling_is_not_another_swarms_business ... ok
    two workers at once: 2 turns in flight, overlapping for 206 ms, intervals (ns) [(1789689520810142279, 1789689521017114363), (1789689520810170521, 1789689521017598396)]
    test two_workers_take_their_turns_at_the_same_moment ... ok
    test a_swarm_at_its_ceiling_starts_no_second_turn ... ok

    test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s

Three independent readings of the same fact, because two turns that both happened is not two turns
that happened together:

- **the runtime's count.** `Server::turns_in_flight_for("two-workers")` is `2` at the instant
  `work_the_assignments` returns — both turns claimed, neither finished;
- **the sessions' clocks.** The two intervals above start 28 µs apart and overlap for 206 ms. A
  runtime that ran them one after the other produces two intervals that do not intersect, and the
  case asserts the intersection is not empty;
- **the members' own verdicts.** Each session waits for a second session to register before it
  answers, so `builder` and `checker` both report `two turns were in flight at once`, both
  assignments reach `Done`, and `AssignmentTaken` fires once per member.

Set the default breadth to one — `const IN_FLIGHT: u64 = 1` in `budget.rs` — and this case goes red
at the first of the three: `left: 1, right: 2`.

## The ceiling

`a_swarm_at_its_ceiling_starts_no_second_turn` runs the same two assignments with
`SWARM_MAX_IN_FLIGHT=1`. One turn starts; three further passes of the same arrow start nothing while
it is still running; nothing is published, nothing is written to `turns/capped.jsonl`, and the turn
that waited runs at the next period and finishes. **Being full is not a refusal** — it costs nothing
and changes nothing in the model.

Remove the ceiling from `trigger.rs` and the case goes red: two turns in flight under a ceiling of
one.

## The swarm-wide cap, and the bound named in the record

Four members at $2.00 each, none of them over the $5.00 per-agent cap, none of them the coordinator,
against `SWARM_MAX_TOTAL_SPEND_USD=5`. The row the swarm retained, verbatim:

    the bound that fired, from turns/capped.jsonl: {"agent":"coordinator","at":"2026-09-17T23:58:43.406477082Z","bound":"swarm","cap":"SWARM_MAX_TOTAL_SPEND_USD","spent_usd":8.0,"turns":4,"unit":"3bd32ffd-729b-47e3-9ee5-9901318e383e","why":"the spend cap was reached: $8.00 of $5.00, by this swarm across every agent in it. This unit and the agent working it are each within their own bounds; raise SWARM_MAX_TOTAL_SPEND_USD, or stop the swarm"}

    test trigger::bounds::a_swarm_over_its_total_is_stopped_with_the_bound_that_fired_named ... ok

Three things in that row are the point of it:

- `"bound":"swarm"` — which fold was measured. `"unit"` and `"agent"` were the only two before;
- `"cap":"SWARM_MAX_TOTAL_SPEND_USD"` — the variable that moves THIS bound. Both the per-agent cap
  and this one reach a `Reached::Spend`, and a record that named the per-agent one would send its
  reader to raise $5.00 while $20.00 was what refused them. Make `report_capped` publish
  `bound.reached.variable()` instead of `bound.variable()` and the case goes red on exactly that
  string;
- **the refusal cost $0.00.** The check precedes the launch, so the swarm's total after three
  periods is the $8.00 it was planted at, the goal took no turn, and the coordinator never ran.

## What this run does NOT show

- no worker spawned another worker, and no swarm extended its own specification. Depth is still one;
- the two sessions are programs, not model turns. Breadth two under a real coordinator has not been
  paid for;
- `turns/spend.jsonl` is not concurrency-safe. `Swarm::record_spend` appends a row with
  `writeln!(file, "{line}")`, which is many small writes, and two members recording a turn at the
  same moment interleave inside a row — captured during this work:

      {"agent{":""agentbuilder"":,""atchecker"":,""at"2026-09-17T23:50:15.408022265Z:"", …

  `Swarm::spend_where` skips a row it cannot parse, so what that costs is two turns and their dollars
  missing from every fold a cap is measured on. The defect is `swarm.rs`'s and predates this wave; it
  was unreachable until a swarm ran two turns at once, which is what this wave makes it do. The cases
  here therefore read verdicts from the event log rather than from that file.

---
format: aep.planning-md/1
id: story:spend-rows-do-not-interleave
kind: story
status: implemented
title: Two members' spend rows shred each other
summary: writeln! is several writes, so two concurrent turns interleave inside a row and spend_where drops it.
relations:
- decomposes: epic:trustworthy-swarm-control
- serves: vision:swarm-builds-itself
- depends_on: story:run-two-workers-at-once
scope:
- confidence: cited
  path: src/runtime/swarm-server/src/swarm.rs
revision: 5
---
## What

`Swarm::record_turn` and `Swarm::record_capped` each appended their row with
`writeln!(file, "{line}")`. `writeln!` is not one write: `std::io::Write::write_fmt` drives the
formatter, which issues a small write per fragment, and the newline is its own. Under `O_APPEND`
each of those writes is atomic on its own — the row is not.

So two members of one swarm recording a turn at the same moment interleave *inside* a row.
`spend_where` reads the file back with

    let Ok(row) = serde_json::from_str::<Json>(line) else { continue };

and drops the shredded line without a word. The turn and its dollars are then missing from every
fold that reads the file — `spend_on`, `spend_by_agent`, and the swarm-wide fold. A cap is not
enforced against spend it cannot see, and the failure is silent in the direction that costs money.

## Why it was not reachable before, and is now

One writer cannot race itself. Until `SWARM_MAX_IN_FLIGHT` there was one turn of a swarm in flight
at a time, so the two append sites had one caller. `story:run-two-workers-at-once` removes that,
which is what turns a latent shape into a defect, and is why the two land together.

## Acceptance

Each row reaches `turns/spend.jsonl` and `turns/capped.jsonl` as a single `write_all` of the row
and its newline, so `O_APPEND` makes the row itself atomic, and no reader has to tolerate a
half-row. The full suite is green with the breadth ceiling in place.

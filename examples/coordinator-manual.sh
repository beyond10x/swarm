#!/usr/bin/env bash
# A coordinator that never claims success.
#
# The point of this one is to make the loop visible without spending model budget or pretending to
# have done work. It reads the goal, reports "not yet", and lets the loop come back round — so a
# person can watch the back-edge being taken, turn after turn, and stop it by hand.
#
#   SWARM_COORDINATOR=examples/coordinator-manual.sh cargo run -p swarm-server
#
# The contract is the whole of what a coordinator must satisfy:
#
#   in   {"goal": "...", "iterations": 3, "swarm": "demo"}   on stdin
#   out  {"reached": false, "note": "..."}                   on stdout
#
# A `metaharness`-launched agent is another program satisfying the same contract. It decides the
# verdict by doing the work; this one decides it by never being finished.
set -euo pipefail

asked=$(cat)
goal=$(printf '%s' "$asked" | python3 -c 'import json,sys; print(json.load(sys.stdin)["goal"])')
turn=$(printf '%s' "$asked" | python3 -c 'import json,sys; print(json.load(sys.stdin)["iterations"])')

# Reported, not decided: a coordinator that answered `true` here would be claiming work nobody did.
printf '{"reached": false, "note": "turn %s on %s — a real coordinator would do the work here"}\n' \
  "$turn" "$(printf '%s' "$goal" | head -c 60)"

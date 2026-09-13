// Acceptance language preserved from the original checker.
pub const LANGUAGE: [(&str, &str); 8] = [
    (
        "unattended",
        "at least one event between the first `swarm.manager.SwarmStarted` and the last verdict other than the `swarm.manager.SwarmStarted` itself, every event in that window naming who issued it, and every one of those the runtime or an agent this log spawned — the command that STARTS a swarm excepted, because a person issues that one by construction",
    ),
    (
        "a non-Coordinator agent exists",
        "a `swarm.agent.AgentSpawned` whose `role` is not `Coordinator`",
    ),
    (
        "work was handed over",
        "a `swarm.agent.AssignmentPosted` naming a non-Coordinator agent, a later `swarm.agent.AssignmentTaken` whose `agent_id` is that same agent, and a record of WHO ISSUED that taking that is not an operator's hand and not some other agent acting in its name",
    ),
    (
        "two transcripts, neither overwritten",
        "two turn files under `turns/`, each attributable to an agent the log knows and the two of them to different agents, and a file for every attempt `spend.jsonl` claims — an attempt with no file of its own is a record that was overwritten or never kept",
    ),
    (
        "the work was finished, not merely started",
        "a `swarm.agent.AssignmentDone` — what `swarm.agent.FinishAssignment` emits — or a `swarm.agent.GateGreen`, recorded AFTER the handover of record was taken, by the agent that took it, and naming that same assignment. `AssignmentPosted` carries no assignment identity, so the handover's is read from `AssignmentTaken.ref` and the `AssignmentRecorded` behind it; where either side names none, the agent and the ordering are checked and the assignment is not, and the finding says which",
    ),
    (
        "the confinement acted",
        "a `tool.decided` in a turn record whose decision is a denial and whose `decided_by` is `frame` — one call, named, that did not happen. A session census is not this: its `by_decider` counts decisions of every outcome, so it cannot say the frame denied anything",
    ),
    (
        "spend is attributed per agent",
        "rows in `turns/spend.jsonl` naming two different agents, both of them agents the log says were spawned",
    ),
    (
        "the bound held with more than one agent to bound",
        "a recorded refusal by the AGENT ceiling — `bound: agent` in `turns/capped.jsonl`, or the sentence `Capped::why()` writes for `Bound::Agent`, in the event log or on a spend row — naming one of at least two agents an `AgentSpawned` says the swarm had",
    ),
];

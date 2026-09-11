// What the orchestrator bootstraps into every swarm when it starts: the orchestrator agent, the
// four boxes every swarm has, their schemas, and the four standing flows. Pure functions, no Vue —
// the store calls them on `startSwarm`, and `public/data/seed.json` was generated from them so the
// seed swarm and a freshly started swarm carry the same boxes and flows.
//
// Step names and kinds come from the swarm's own documents:
//   orchestrator.check-in     swarm/AGENTS.md §1
//   orchestrator.memory-tick  swarm/coordinator/CHARTER.md §5
//   agent.spawn               swarm/AGENTS.md §4
//   agent.wake-pass           swarm/wake.d/improver.conf WAKE_PROMPT
// `command` = a script does it; `llm` = a judgement the agent makes.
import type { Agent, Box, Edge, Flow, Port, Schema, Step, StepKind } from '../model'

export const SWARM_HOME = '~/beyond10x/harness-builder'

export const SCHEMA = {
  messagePosted: 'swarm.board.MessagePosted/1',
  agentEvent: 'swarm.agent.AgentEvent/1',
  gateFired: 'swarm.schedule.GateFired/1',
  decisionRaised: 'swarm.decision.DecisionRaised/1',
  decisionAnswered: 'swarm.decision.DecisionAnswered/1',
  assignmentPosted: 'swarm.agent.AssignmentPosted/1',
} as const

/** `b10x forward 2026-09-11` → `b10x-forward-2026-09-11`. */
export function slugify(name: string): string {
  return name
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    || 'swarm'
}

export function orchestratorAgent(): Agent {
  return { agentId: 'orchestrator', role: 'orchestrator', harness: 'claude', state: 'Spawned' }
}

const port = (name: string, schemaId: string, required = false): Port =>
  required ? { name, schemaId, required } : { name, schemaId }

const schema = (schemaId: string, name: string, sourceFile: string, sourceName: string): Schema => ({
  schemaId,
  name,
  version: 1,
  jsonSchema: { $comment: `projection of ${sourceFile} ${sourceName}` },
})

export function bootstrapSchemas(): Schema[] {
  return [
    schema(SCHEMA.messagePosted, 'MessagePosted', 'swarm/domains/board.yaml', 'swarm.board.MessagePosted'),
    schema(SCHEMA.agentEvent, 'AgentEvent', 'swarm/bin/eventlog.sh', 'the seven kinds of WORKING-RULES.md §10'),
    schema(SCHEMA.gateFired, 'GateFired', 'swarm/domains/schedule.yaml', 'swarm.schedule.GateFired'),
    schema(SCHEMA.decisionRaised, 'DecisionRaised', 'swarm/domains/decision.yaml', 'swarm.decision.DecisionRaised'),
    schema(SCHEMA.decisionAnswered, 'DecisionAnswered', 'swarm/domains/decision.yaml', 'swarm.decision.DecisionAnswered'),
    schema(SCHEMA.assignmentPosted, 'AssignmentPosted', 'swarm/domains/agent.yaml', 'swarm.agent.AssignmentPosted'),
  ]
}

/** The four boxes every swarm has, laid out in a row. */
export function bootstrapBoxes(): Box[] {
  const row = (i: number) => ({ x: 40 + i * 280, y: 40 })
  return [
    {
      boxId: 'board', name: 'board', kind: 'store', version: 1, position: row(0),
      summary: 'swarm/bin/board.sh — inboxes; the default channel (AGENTS.md §3)',
      inputs: [], outputs: [port('message.posted', SCHEMA.messagePosted)],
    },
    {
      boxId: 'event-log', name: 'event-log', kind: 'store', version: 1, position: row(1),
      summary: 'swarm/bin/eventlog.sh — a state channel, seven kinds (WORKING-RULES.md §10)',
      inputs: [port('event', SCHEMA.agentEvent, true)], outputs: [],
    },
    {
      boxId: 'wake-gate', name: 'wake-gate', kind: 'tool', version: 1, position: row(2),
      summary: 'swarm/bin/wakegate.sh — wakes a window on a signal instead of a timer (swarm/wake.d/)',
      inputs: [port('event', SCHEMA.messagePosted)], outputs: [port('gate.fired', SCHEMA.gateFired)],
    },
    {
      boxId: 'decision-console', name: 'decision-console', kind: 'service', version: 1, position: row(3),
      summary: 'decisions.md rows, answered like `1y 2n 3?` (WORKING-RULES.md §9)',
      inputs: [port('decision.raised', SCHEMA.decisionRaised, true)],
      outputs: [port('decision.answered', SCHEMA.decisionAnswered)],
    },
  ]
}

interface StepSpec {
  id: string
  name: string
  kind: StepKind
  run?: string
  prompt?: string
  inputs?: Port[]
  outputs?: Port[]
  context?: Step['context']
  evidenceKind?: string
}

function flow(flowId: string, binds: Flow['binds'], specs: StepSpec[]): Flow {
  const steps: Step[] = specs.map((s) => {
    const step: Step = { stepId: s.id, name: s.name, kind: s.kind, inputs: s.inputs ?? [], outputs: s.outputs ?? [] }
    if (s.run) step.run = s.run
    if (s.prompt) step.prompt = s.prompt
    if (s.context) step.context = s.context
    if (s.evidenceKind) step.evidenceKind = s.evidenceKind
    return step
  })
  const edges: Edge[] = steps.slice(1).map((s, i) => ({ from: steps[i]!.stepId, to: s.stepId }))
  return { flowId, name: flowId, binds, inputs: [], outputs: [], steps, edges }
}

export function bootstrapFlows(): Flow[] {
  const generation = { entity: 'swarm.orchestrator.Generation', state: 'Ticking' }
  return [
    // swarm/AGENTS.md §1 — the ingestion cycle, every tick, in this order
    flow('orchestrator.check-in', generation, [
      { id: 'take-inbox', name: 'take the inbox', kind: 'command',
        run: 'swarm/bin/board.sh read --agent coord',
        outputs: [port('messages', SCHEMA.messagePosted)] },
      { id: 'take-ledger', name: 'take the ledger', kind: 'command',
        run: 'aep plan artifact list --kind story --status draft && aep plan artifact waves' },
      { id: 'take-pane-state', name: 'take the pane state', kind: 'command',
        run: 'swarm/bin/status.sh && swarm/bin/report.sh && swarm/bin/swarm-state.sh --json' },
      { id: 'classify', name: 'classify each window', kind: 'llm',
        prompt: 'Four verdicts only: continue, idle is correct, decision, fault (WORKING-RULES.md §8). The classification is the work; do not relay.' },
      { id: 'route', name: 'route', kind: 'llm',
        prompt: 'Keep, dispatch, or park. The test is blocking, not size: if Timo could type a sentence while it runs and the coordinator would not see it, it is not for the main thread.' },
      { id: 'report', name: 'report', kind: 'llm',
        prompt: 'Verdict first, deviations only, decisions by number (WORKING-RULES.md §6, §9). A tick where everything is idle-is-correct produces no message.' },
    ]),
    // swarm/coordinator/CHARTER.md §5 — the 15-minute cycle
    flow('orchestrator.memory-tick', generation, [
      { id: 'defer-check', name: 'defer check', kind: 'llm',
        prompt: 'Is a question in flight to Timo (section `## 2 In flight to Timo` non-empty)? If yes, go to defer. coord-restart.sh refuses to restart while that section has content.' },
      { id: 'refresh-derived', name: 'refresh the derived section', kind: 'command',
        run: 'swarm/bin/coord-windows.sh --write' },
      { id: 'rewrite-sections', name: 'rewrite sections', kind: 'llm',
        prompt: 'Rewrite sections 0, 1, 2, 4, 5 of coordinator.memory.md in place. Drop per each section\'s eviction rule first, then write. Never append.',
        context: { files: ['swarm/runs/<run>/coordinator.memory.md'], budgetTokens: 8192 } },
      { id: 'gate', name: 'gate', kind: 'command',
        run: 'swarm/bin/coord-budget.sh', evidenceKind: 'exit-code' },
      { id: 'single-coordinator-check', name: 'single-coordinator check', kind: 'command',
        run: '.claude/hooks/coordinator-seed.sh  # scans /proc for another `claude` with CC_COORD=1' },
      { id: 'restart', name: 'restart', kind: 'command',
        run: 'swarm/bin/coord-restart.sh' },
      { id: 'defer', name: 'defer', kind: 'llm',
        prompt: 'Record `defers: N` in `## 0 Generation`. Hard cap 3 consecutive defers; on the 4th tick write the in-flight question into decisions.md as a numbered binary row with its default-on-silence, then restart from step 2.' },
    ]),
    // swarm/AGENTS.md §4 — spawning a swarm member
    flow('agent.spawn', { entity: 'swarm.agent.Agent', state: 'Spawned' }, [
      { id: 'pick-role', name: 'pick a role', kind: 'llm',
        prompt: 'Pick a definition from .agents/ — one file per role. Do not hand-write a new role when one fits; if none fits, write the definition first, then spawn.',
        context: { files: ['.agents/README.md'] } },
      { id: 'guarantee-loadout', name: 'guarantee the loadout', kind: 'llm',
        prompt: 'Every member loads aep, ess, worktree, plus whatever the role adds, and .skills/swarm-inbox.',
        context: { skills: ['aep-plan:planning', 'aep-drive:*', 'ess-specify:specify', 'workspace-hygiene:worktree', 'swarm-inbox'] } },
      { id: 'create-inbox', name: 'create its inbox', kind: 'command',
        run: 'swarm/bin/board.sh create-inbox --agent <slug> --name main' },
      { id: 'post-assignment', name: 'post its assignment', kind: 'command',
        run: 'swarm/bin/board.sh post --to <slug> --inbox main --from coord --body "<outcome, sign-off verbatim, what it may not do>"',
        outputs: [port('assignment', SCHEMA.assignmentPosted)] },
      { id: 'tell-provenance', name: 'tell it the provenance rule', kind: 'llm',
        prompt: 'The orchestrator commands, everyone else informs (AGENTS.md §2); communication goes through the inbox (§3).' },
    ]),
    // swarm/wake.d/improver.conf — WAKE_PROMPT
    flow('agent.wake-pass', { entity: 'swarm.agent.Agent', state: 'Working' }, [
      { id: 'drain-inbox', name: 'drain inbox', kind: 'command',
        run: 'swarm/bin/board.sh read --agent <slug>  # it MARKS READ, so act on what it returns',
        inputs: [port('gate.fired', SCHEMA.gateFired)] },
      { id: 're-read-role', name: 're-read role', kind: 'llm',
        prompt: 'Re-read .agents/<role>.md.', context: { files: ['.agents/<role>.md'] } },
      { id: 'one-round', name: 'one round', kind: 'llm',
        prompt: 'Do one round, then stop.' },
      { id: 'arm-gate', name: 'arm the gate', kind: 'command',
        run: 'swarm/bin/wakegate.sh <slug> --arm <files you wrote>' },
    ]),
  ]
}

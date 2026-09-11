// The UI's data model. Mirrors the ESS specification under ../swarm2 (domains: manager, config,
// agent, board, decision, schedule, orchestrator, work, fault, flow, blackbox), camelCased.
// Owned by the integrator. Owners of views/components import from here and never edit it.

export type SwarmState = 'Created' | 'Running' | 'Paused' | 'Stopped' | 'Deleted'

/** What the create-swarm form collects. Maps to swarm2.config.Budgets and swarm2.manager.Swarm. */
export interface Limits {
  budgetTokens?: number
  maxAgents?: number
  diskFloorGb?: number
  memoryFileBytes?: number
  maxDefers?: number
}

export interface Swarm {
  swarmId: string
  displayName: string
  objective: string
  tmuxSession: string
  home: string
  state: SwarmState
  limits: Limits
  createdAt: string
  startedAt?: string
  agents: Agent[]
  boxes: Box[]
  connections: Connection[]
  flows: Flow[]
  schemas: Schema[]
  decisions: Decision[]
}

export type AgentState = 'Spawned' | 'Assigned' | 'Working' | 'Blocked' | 'Parked' | 'Idle' | 'Faulted' | 'Retired'
export type Harness = 'claude' | 'codex' | 'b10x'

export interface Host {
  tmuxWindowId?: string
  windowIndex?: number
  sessionId?: string
  cwd?: string
}

export interface AgentEvent {
  ts: string
  kind: 'assignment_taken' | 'step_done' | 'gate' | 'blocked' | 'decision' | 'decision_cleared' | 'idle' | 'fault' | 'note'
  ref?: string
  exit?: number
  msg: string
}

export interface Agent {
  agentId: string        // role slug: `improver`, `cv2-aep`, `orchestrator`
  role: string           // .agents/<role>.md
  harness: Harness
  state: AgentState
  host?: Host
  assignment?: string
  blockedOn?: string
  lastEvent?: AgentEvent
  unread?: number
}

/** A typed port on a blackbox or a step. `schemaId` names a Schema known to everyone. */
export interface Port {
  name: string
  schemaId: string
  required?: boolean
}

export type BoxKind = 'flow' | 'service' | 'tool' | 'store' | 'agent' | 'swarm'

/** A blackbox: only inputs and outputs are visible; other agents connect to them. */
export interface Box {
  boxId: string
  name: string
  kind: BoxKind
  ownerAgentId?: string
  inputs: Port[]
  outputs: Port[]
  version: number
  flowId?: string        // when kind === 'flow', the Flow that is its internals
  summary?: string
  position?: { x: number; y: number }
}

export interface Connection {
  connectionId: string
  from: { boxId: string; output: string }
  to: { boxId: string; input: string }
  delivery?: 'at_least_once' | 'at_most_once'
}

export interface Schema {
  schemaId: string
  name: string
  version: number
  jsonSchema: unknown
}

export type StepKind = 'command' | 'tool' | 'llm' | 'subflow'

export interface StepContext {
  skills?: string[]
  tools?: string[]
  files?: string[]
  writeScope?: string[]
  budgetTokens?: number
  cwd?: string
}

export interface Step {
  stepId: string
  name: string
  kind: StepKind
  inputs: Port[]
  outputs: Port[]
  run?: string
  tool?: string
  prompt?: string
  subflowId?: string
  context?: StepContext
  evidenceKind?: string
}

export interface Wire { fromOutput: string; toInput: string }

export interface Edge {
  from: string           // stepId
  to: string             // stepId
  when?: string
  wires?: Wire[]
}

/** A DAG that runs inside one workflow state. */
export interface Flow {
  flowId: string
  name: string
  binds?: { entity: string; state: string }
  inputs: Port[]
  outputs: Port[]
  context?: StepContext
  steps: Step[]
  edges: Edge[]
}

export type DecisionState = 'Open' | 'Answered' | 'Held' | 'Moot' | 'Withdrawn' | 'Taken'

export interface Decision {
  number: number
  question: string
  defaultOnSilence: string
  state: DecisionState
  answer?: 'y' | 'n'
  source?: string
}

/** The seed document under public/data/seed.json. */
export interface Seed {
  swarms: Swarm[]
}

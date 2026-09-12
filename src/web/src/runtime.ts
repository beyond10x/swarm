// The runtime API.
//
// Everything the canvas knows comes from here. There are no fixtures and no localStorage: a swarm
// is what the server's log says it is, and the only way to change one is to issue a command the
// specification declares.
//
// Three things the server offers, because the specification describes three kinds of thing:
// issue a command, read a view, watch what happens. This file is one function per kind plus the
// shapes they return.

const BASE = import.meta.env.VITE_RUNTIME_URL ?? "http://localhost:5000";

/** One entity instance, exactly as the interpreter holds it. */
export interface Instance {
  entity: string;
  id: string;
  identity_field: string;
  state: string;
  /** Every declared field. `null` means nothing has written it — not the same as absent. */
  fields: Record<string, unknown>;
  revision: number;
}

/** Everything on one swarm's canvas, keyed by entity name. */
export type Canvas = Record<string, Instance[]>;

/** What the specification says this system is. The UI reads the shape rather than hard-coding it. */
export interface Shape {
  system: string;
  entities: {
    name: string;
    identity: string;
    states: string[];
    /** The states that are ends. A canvas draws a finished instance differently. */
    terminal: string[];
    fields: string[];
    /** What this entity points at, and through which field. These are the canvas's edges. */
    relations: { name: string; target: string; via: string; owns: boolean }[];
  }[];
  commands: { name: string; input: { name: string; optional: boolean }[] }[];
  views: string[];
}

/** What issuing a command produced. */
export interface Issued {
  outcome: string;
  error: string | null;
  events: { name: string; fields: Record<string, unknown> }[];
}

/** What a coordinator turn cost, summed from what the vendor reported. */
export interface Spent {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number | null;
  requests: number;
  tool_calls: number;
  model: string | null;
  duration_ms: number | null;
  thinking_tokens: number | null;
}

/** One `metaharness.event/1` record, as the coordinator's run emitted it. */
export interface AgentEvent {
  seq?: number;
  at?: string | null;
  event: string;
  [field: string]: unknown;
}

/** One recorded coordinator run. */
export interface TurnRecord {
  name: string;
  iterations: number;
  goal: string;
  bytes: number;
  reached: boolean | null;
  note: string | null;
  ended_at: string | null;
}

/** Something that happened, as the stream reports it. Every change carries when the server saw it. */
export type Change = { at: string } & (
  | {
      kind: "applied";
      command: string;
      outcome: string;
      actor: string | null;
      events: { name: string; fields: Record<string, unknown> }[];
      /** The instance as it now stands, when the outcome acted on one. */
      instance: Instance | null;
    }
  | {
      kind: "routed";
      binding: string;
      command: string;
      outcome: string;
      instance: Instance | null;
    }
  | {
      kind: "ticked";
      binding: string;
      command: string;
      goal: string;
      iterations: number;
      instance: Instance | null;
    }
  | {
      kind: "turn";
      goal: string;
      iterations: number;
      phase: "asking" | "answered" | "unfinished";
      reached: boolean | null;
      note: string | null;
      error: string | null;
      took_ms: number | null;
      spent: Spent | null;
    }
  | {
      kind: "agent";
      goal: string;
      iterations: number;
      seq: number;
      spent: Spent;
      event: AgentEvent;
    }
  | { kind: "reloaded" }
);

/** One event as the log holds it. */
export interface Recorded {
  seq: number;
  at: string;
  entity: string;
  id: string;
  version: number;
  name: string;
  actor: string;
  request: string;
  fields: Record<string, unknown> | null;
}

/** Where one swarm is, in one row. */
export interface SwarmStatus {
  slug: string;
  display_name: string | null;
  state: string | null;
  goal: { id: string; state: string; text: string; iterations: number } | null;
  instances: Record<string, number>;
  watchers: number;
  events: number;
  spent: Spent;
  turns_recorded: number;
}

/** Where the runtime is. */
export interface Status {
  system: string;
  spec_files: number;
  started_at: string;
  now: string;
  periodic: { binding: string; every_s: number }[];
  next_tick_at: string | null;
  ticks: number;
  coordinator: { configured: boolean; program: string | null; in_flight: number };
  swarms: SwarmStatus[];
}

/** A refusal, carrying which kind it was so a caller can tell a bad request from a broken store. */
export class Refused extends Error {
  constructor(
    message: string,
    readonly kind: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "Refused";
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
  });
  if (!response.ok) {
    const problem = (await response.json().catch(() => null)) as {
      error: string;
      kind: string;
    } | null;
    throw new Refused(
      problem?.error ?? `${response.status} ${response.statusText}`,
      problem?.kind ?? "unknown",
      response.status,
    );
  }
  return response.status === 204
    ? (undefined as T)
    : ((await response.json()) as T);
}

/** What the specification declares. Read once at boot. */
export function shape(): Promise<Shape> {
  return request<Shape>("/spec");
}

/** Every swarm the server holds. */
export function swarms(): Promise<string[]> {
  return request<string[]>("/swarms");
}

/**
 * Opens a swarm's log, creating its directory the first time.
 *
 * This makes a PLACE for a swarm, not a swarm — the record itself is made by issuing CreateSwarm,
 * like every other fact in the system.
 */
export function open(slug: string): Promise<{ slug: string }> {
  return request("/swarms", { method: "POST", body: JSON.stringify({ slug }) });
}

/** Everything on one swarm's canvas. */
export function canvas(slug: string): Promise<Canvas> {
  return request<Canvas>(`/swarms/${encodeURIComponent(slug)}`);
}

/** One view, computed by the server against its own world. */
export function view(
  slug: string,
  name: string,
): Promise<Record<string, unknown>[]> {
  return request(
    `/swarms/${encodeURIComponent(slug)}/views/${encodeURIComponent(name)}`,
  );
}

/**
 * Issues one command.
 *
 * `request` is an idempotency key. Sending the same one twice with the same input is a retry and
 * writes nothing the second time, which is what makes a dropped response safe to resend.
 */
export function issue(
  slug: string,
  command: string,
  input: Record<string, unknown>,
  options: { actor?: string; request?: string } = {},
): Promise<Issued> {
  return request<Issued>(
    `/swarms/${encodeURIComponent(slug)}/commands/${encodeURIComponent(command)}`,
    { method: "POST", body: JSON.stringify({ input, ...options }) },
  );
}

/**
 * Watches one swarm. Returns a function that stops watching.
 *
 * The server drops a reader that falls too far behind rather than feeding it a gap it cannot see,
 * so `onDropped` is where a caller reloads the canvas — which is cheap — instead of carrying on
 * with a picture that is quietly wrong. The browser reconnects on its own; `onOpen` fires each time
 * it does, so a caller can show that it is listening again.
 */
export function watch(
  slug: string,
  handlers: {
    onChange: (change: Change) => void;
    onOpen?: () => void;
    onDropped?: () => void;
  },
): () => void {
  const source = new EventSource(
    `${BASE}/swarms/${encodeURIComponent(slug)}/events`,
  );

  source.onopen = () => handlers.onOpen?.();
  source.onmessage = (message) => {
    try {
      handlers.onChange(JSON.parse(message.data) as Change);
    } catch {
      // A message this client cannot read is a message it must not act on.
    }
  };
  source.onerror = () => handlers.onDropped?.();

  return () => source.close();
}

/** The most recent events in a swarm's log, oldest first. */
export function log(slug: string, limit = 200): Promise<Recorded[]> {
  return request<Recorded[]>(
    `/swarms/${encodeURIComponent(slug)}/log?limit=${limit}`,
  );
}

/** Every recorded coordinator run, newest first. */
export function turns(slug: string): Promise<TurnRecord[]> {
  return request<TurnRecord[]>(`/swarms/${encodeURIComponent(slug)}/turns`);
}

/** One recorded coordinator run, every event. */
export function turn(slug: string, name: string): Promise<AgentEvent[]> {
  return request<AgentEvent[]>(
    `/swarms/${encodeURIComponent(slug)}/turns/${encodeURIComponent(name)}`,
  );
}

/** Where the runtime is right now. */
export function status(): Promise<Status> {
  return request<Status>("/status");
}

/** Throws the server's in-memory world away and rebuilds it from the log. */
export function reload(slug: string): Promise<void> {
  return request<void>(`/swarms/${encodeURIComponent(slug)}/reload`, {
    method: "POST",
  });
}

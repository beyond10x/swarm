// The runtime API.
//
// Everything the canvas knows comes from here. There are no fixtures and no localStorage: a swarm
// is what the server's log says it is, and the only way to change one is to issue a command the
// specification declares.
//
// Three things the server offers, because the specification describes three kinds of thing:
// issue a command, read a view, watch what happens. This file is one function per kind plus the
// shapes they return.

const BASE = import.meta.env.VITE_RUNTIME_URL ?? 'http://localhost:7777'

/** One entity instance, exactly as the interpreter holds it. */
export interface Instance {
  entity: string
  id: string
  identity_field: string
  state: string
  /** Every declared field. `null` means nothing has written it — not the same as absent. */
  fields: Record<string, unknown>
  revision: number
}

/** Everything on one swarm's canvas, keyed by entity name. */
export type Canvas = Record<string, Instance[]>

/** What the specification says this system is. The UI reads the shape rather than hard-coding it. */
export interface Shape {
  system: string
  entities: {
    name: string
    identity: string
    states: string[]
    /** The states that are ends. A canvas draws a finished instance differently. */
    terminal: string[]
    fields: string[]
    /** What this entity points at, and through which field. These are the canvas's edges. */
    relations: { name: string; target: string; via: string; owns: boolean }[]
  }[]
  commands: { name: string; input: { name: string; optional: boolean }[] }[]
  views: string[]
}

/** What issuing a command produced. */
export interface Issued {
  outcome: string
  error: string | null
  events: { name: string; fields: Record<string, unknown> }[]
}

/** Something that happened, as the stream reports it. */
export type Change =
  | { kind: 'applied'; command: string; outcome: string; events: { name: string }[] }
  | { kind: 'routed'; binding: string; command: string }
  | { kind: 'ticked'; binding: string; command: string }

/** A refusal, carrying which kind it was so a caller can tell a bad request from a broken store. */
export class Refused extends Error {
  constructor(
    message: string,
    readonly kind: string,
    readonly status: number,
  ) {
    super(message)
    this.name = 'Refused'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
  })
  if (!response.ok) {
    const problem = (await response.json().catch(() => null)) as
      | { error: string; kind: string }
      | null
    throw new Refused(
      problem?.error ?? `${response.status} ${response.statusText}`,
      problem?.kind ?? 'unknown',
      response.status,
    )
  }
  return response.status === 204 ? (undefined as T) : ((await response.json()) as T)
}

/** What the specification declares. Read once at boot. */
export function shape(): Promise<Shape> {
  return request<Shape>('/spec')
}

/** Every swarm the server holds. */
export function swarms(): Promise<string[]> {
  return request<string[]>('/swarms')
}

/**
 * Opens a swarm's log, creating its directory the first time.
 *
 * This makes a PLACE for a swarm, not a swarm — the record itself is made by issuing CreateSwarm,
 * like every other fact in the system.
 */
export function open(slug: string): Promise<{ slug: string }> {
  return request('/swarms', { method: 'POST', body: JSON.stringify({ slug }) })
}

/** Everything on one swarm's canvas. */
export function canvas(slug: string): Promise<Canvas> {
  return request<Canvas>(`/swarms/${encodeURIComponent(slug)}`)
}

/** One view, computed by the server against its own world. */
export function view(slug: string, name: string): Promise<Record<string, unknown>[]> {
  return request(`/swarms/${encodeURIComponent(slug)}/views/${encodeURIComponent(name)}`)
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
    { method: 'POST', body: JSON.stringify({ input, ...options }) },
  )
}

/**
 * Watches one swarm. Returns a function that stops watching.
 *
 * The server drops a reader that falls too far behind rather than feeding it a gap it cannot see,
 * so `onDropped` is where a caller reloads the canvas — which is cheap — instead of carrying on
 * with a picture that is quietly wrong.
 */
export function watch(
  slug: string,
  onChange: (change: Change) => void,
  onDropped?: () => void,
): () => void {
  const source = new EventSource(`${BASE}/swarms/${encodeURIComponent(slug)}/events`)

  source.onmessage = (message) => {
    try {
      onChange(JSON.parse(message.data) as Change)
    } catch {
      // A message this client cannot read is a message it must not act on.
    }
  }
  source.onerror = () => onDropped?.()

  return () => source.close()
}

/** Throws the server's in-memory world away and rebuilds it from the log. */
export function reload(slug: string): Promise<void> {
  return request<void>(`/swarms/${encodeURIComponent(slug)}/reload`, { method: 'POST' })
}

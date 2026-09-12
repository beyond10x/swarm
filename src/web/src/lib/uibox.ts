// What a `swarm.blackbox.Box{kind: Ui}` renders, decided from the box alone.
//
// WHERE A UI BOX'S COMPONENT AND PROPS LIVE — the question the story left open, settled here.
//
// The component name is `Box.ref_id`. Not a new field: `blackbox.yaml` already declares ref_id as
// "what this box stands for, when its kind stands for something: a `goal_id` for a Goal, a
// component name for a Ui, a server name for an Mcp". A second field for the same sentence would
// be a second answer to a question the model already answers.
//
// The props are `Box.props`, a `Map<String, String>` added to the entity for this story. They
// could not go anywhere that already exists: `summary` is prose an agent writes for a person, and
// a port is a typed wire between boxes, not a value a panel is drawn with. Map<String, String> and
// not a struct because the 21 components in `components/ui` take 21 different prop shapes and
// ess/1 has no union over them; a value that is not a scalar is written as JSON text and decoded
// here, which keeps the kernel's type honest about what it actually stores — a string.
//
// This module is deliberately free of Vue and of `@/runtime`: the decision is a pure function so
// it can be tested by `npm test` without a component runner, and `@/runtime` reads `import.meta.env`
// at module scope, which node cannot load.

/** As much of an instance as this decision needs. A `runtime.Instance` satisfies it. */
export interface BoxLike {
  entity: string
  fields: Record<string, unknown>
}

/** How a prop's text is read, as `components/ui/index.ts` declares the prop's type. */
export type PropType = 'string' | 'number' | 'boolean' | 'json'

/** Every component's declared props, by name: `uiPropTypes` from the contract. */
export type Declared = Readonly<Record<string, Readonly<Record<string, PropType>>>>

/** What to render for one Ui box. */
export interface UiBoxPanel {
  kind: 'panel'
  /** A component exported by `@/components/ui`, by name. */
  component: string
  /** What to pass it, each value read as the type the contract declares. */
  props: Record<string, unknown>
  /** The runtime view whose rows feed the component, when the box names one. */
  view: string | undefined
}

/**
 * A Ui box naming a component the library does not have.
 *
 * Distinct from `undefined`, which means "not a Ui box at all". A caller handed the same value for
 * both cannot draw them differently, and the difference is the whole point: an agent wrote that
 * name, and nobody learns it was wrong from a node that quietly lists `ref_id` as a field.
 */
export interface UiBoxUnknown {
  kind: 'unknown-component'
  named: string
}

export type UiBoxSpec = UiBoxPanel | UiBoxUnknown

const BOX = 'swarm.blackbox.Box'

/**
 * `view` is read by the renderer, not passed to the component.
 *
 * A table on the canvas is fed from a view the server computes, and the view's name is a fact
 * about where the rows come from rather than a prop any component in the library declares.
 */
const FEED = 'view'

/**
 * The component and props for a Ui box, or undefined when this instance is not one.
 *
 * `known` is the set of component names the library actually exports, passed in rather than
 * imported so this file stays free of `.vue`. A name the library does not have returns undefined:
 * a box may name a component that does not exist — an agent wrote it — and drawing nothing for it
 * is a choice, where guessing a nearest match would be an invention.
 */
export function uiBoxSpec(
  instance: BoxLike,
  known: ReadonlySet<string>,
  declared?: Declared,
): UiBoxSpec | undefined {
  if (instance.entity !== BOX) return undefined
  if (instance.fields.kind !== 'Ui') return undefined

  const component = instance.fields.ref_id
  // A Ui box that names nothing has refused nothing: there is no component to report as missing.
  if (typeof component !== 'string' || !component) return undefined
  if (!known.has(component)) return { kind: 'unknown-component', named: component }

  const types = declared?.[component]

  const written = instance.fields.props
  // `null` is ESS for "nothing has written this", and anything that is not a map of strings is a
  // value this decoder does not understand — half-reading one would pass a component props it
  // never declared.
  const entries =
    written && typeof written === 'object' && !Array.isArray(written)
      ? Object.entries(written as Record<string, unknown>)
      : []

  const props: Record<string, unknown> = {}
  let view: string | undefined
  for (const [name, value] of entries) {
    if (typeof value !== 'string') continue
    if (name === FEED) {
      view = value
      continue
    }
    props[name] = decode(value, types?.[name])
  }
  return { kind: 'panel', component, props, view }
}

/**
 * One prop value, read as the type the contract declares for it.
 *
 * The map's values are strings because that is the only map ess/1 has, so something has to say
 * what a string means. That something is `components/ui/index.ts` — the same table a person reads
 * before writing the box — and never the text itself: `text*: string` means `404` is the three
 * characters an author typed, and a rule that guessed from the digits would make the obvious thing
 * the wrong thing.
 *
 * A value that does not read as its declared type is left as written. A `rows: number` given
 * `many` is a mistake worth seeing on the panel, and `NaN` would hide which mistake it was.
 *
 * A prop the table does not type — `label`, `placeholder`, `min` and the other bare names — is
 * text, unless it is bracketed: `[` or `{` opens a value no scalar prop could want, and that is as
 * far as a guess may go where the contract declares nothing.
 */
function decode(value: string, type: PropType | undefined): unknown {
  switch (type) {
    case 'string':
      return value
    case 'boolean':
      return value === 'true' ? true : value === 'false' ? false : value
    case 'number': {
      const number = Number(value)
      return value.trim() !== '' && Number.isFinite(number) ? number : value
    }
    case 'json':
      return parsed(value) ?? value
    default:
      return /^\s*[[{]/.test(value) ? (parsed(value) ?? value) : value
  }
}

/** The JSON a string holds, or undefined when it holds none. */
function parsed(value: string): unknown {
  try {
    return JSON.parse(value) as unknown
  } catch {
    return undefined
  }
}

/**
 * Columns for a table fed from a view whose box named none.
 *
 * A `UiTable` with rows and no `columns` draws an empty table, which looks like a view with no
 * rows and is the wrong thing to show. The view's own field names are what the server called them,
 * so they are a default rather than an invention — a box that wants better labels says so in its
 * `columns` prop and this is never reached.
 */
export function columnsFrom(rows: readonly Record<string, unknown>[]): { key: string; label: string }[] {
  const first = rows[0]
  if (!first) return []
  return Object.keys(first).map((key) => ({ key, label: key }))
}

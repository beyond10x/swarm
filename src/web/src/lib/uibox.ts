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

/** What to render for one Ui box. */
export interface UiBoxSpec {
  /** A component exported by `@/components/ui`, by name. */
  component: string
  /** What to pass it, decoded from the box's `props`. */
  props: Record<string, unknown>
  /** The runtime view whose rows feed the component, when the box names one. */
  view: string | undefined
}

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
export function uiBoxSpec(instance: BoxLike, known: ReadonlySet<string>): UiBoxSpec | undefined {
  if (instance.entity !== BOX) return undefined
  if (instance.fields.kind !== 'Ui') return undefined

  const component = instance.fields.ref_id
  if (typeof component !== 'string' || !known.has(component)) return undefined

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
    props[name] = decode(value)
  }
  return { component, props, view }
}

/**
 * One prop value, as the component wants it.
 *
 * The map's values are strings because that is the only map ess/1 has. A value that parses as JSON
 * is that JSON — so `columns` arrives as an array and `dense` as a boolean — and a value that does
 * not is the string itself. A prop that must stay the string "12" is written with its quotes,
 * `"12"`, which is what makes the rule one rule instead of a table of special cases.
 */
function decode(value: string): unknown {
  try {
    return JSON.parse(value) as unknown
  } catch {
    return value
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

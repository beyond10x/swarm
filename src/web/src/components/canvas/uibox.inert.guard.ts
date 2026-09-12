// The canvas's read-only claim, decided by VALUE rather than by tokens.
//
// This file declares no cases of its own. It exists because two files check the same claim —
// `uibox.inert.test.ts` against the shipped `UiBox.vue`, and `uibox.inert.mutants.test.ts` against
// mutated copies of it — and a predicate copied into both drifts from the one that ships, which
// makes the mutation harness prove something about a copy.
//
// It is NOT named `*.test.ts`: node counts a test file with no cases as one passing test, which is
// a green asserting nothing, and a module that registers cases cannot be imported from inside a
// running case — node cancels the nested test. So it imports no `node:` module and throws plain
// `Error`s, which is also what keeps it inside the app's typecheck (`tsconfig.json` excludes tests
// to keep node's globals out of reach of browser code) without pulling node's types in.
//
// What it checks, and why by value:
//
// An earlier version asked whether the word `inert` OCCURRED in the binding, in the note's `v-if`
// and in the computed. Three edits satisfy that while doing nothing — `:inert="inert && undefined"`
// is `undefined` for every input, `v-if="!inert && …"` shows the note for the complement of the
// inert set, and `uiEmitters.has(…) && false` is false for every box. Under the first and third
// every emitter panel is interactive again; under the second a live panel claims to be display
// only. Asking about the shape of an expression is the same failure as asking about its characters,
// one level up.
//
// So each expression is EVALUATED, over the inputs that decide it, and compared with the truth
// table it must have. An expression that mentions something this file does not supply throws, which
// is a failure and not a pass: the guard fails closed.

interface Node {
  type: number
  tag?: string
  content?: string
  props?: { type: number; name?: string; arg?: { content?: string }; exp?: { content?: string } }[]
  children?: Node[]
}

/** The note an inert panel shows. The sentence is the contract with the person reading the canvas. */
export const NOTE = 'Display only: nothing can keep what this component emits.'

/** The guard's own failure: a plain `Error`, so this module needs nothing from `node:`. */
function ok(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message)
}

/** `ok`, for the two values a truth table compares. */
function equal(actual: unknown, expected: unknown, message: string): void {
  ok(actual === expected, message)
}

/** Evaluate a template or script expression against a supplied scope. */
function value(expression: string, scope: Record<string, unknown>): unknown {
  const keys = Object.keys(scope)
  const fn = new Function(...keys, `return (${expression})`) as (...args: unknown[]) => unknown
  return fn(...keys.map((key) => scope[key]))
}

/** The text of `computed(...)`'s argument, paren-balanced from `const <name> = computed(`. */
function computedBody(script: string, name: string): string | undefined {
  const at = script.indexOf(`const ${name} = computed(`)
  if (at < 0) return undefined
  const open = script.indexOf('(', at + `const ${name} = `.length)
  let depth = 0
  for (let i = open; i < script.length; i += 1) {
    if (script[i] === '(') depth += 1
    else if (script[i] === ')') {
      depth -= 1
      if (depth === 0) return script.slice(open + 1, i).replace(/^\s*\(\s*\)\s*=>/, '')
    }
  }
  return undefined
}

/**
 * Assert that `source` — the text of a `UiBox.vue` — renders an emitting component inert and says
 * so. Throws `AssertionError` when it does not.
 */
export async function assertInertGuard(source: string): Promise<void> {
  const { parse } = await import('@vue/compiler-sfc')
  const box = parse(source, { filename: 'UiBox.vue' })
  const template = box.descriptor.template
  ok(template?.ast, 'UiBox.vue has a template')

  const elements: Node[] = []
  ;(function walk(node: Node) {
    if (node.type === 1) elements.push(node)
    for (const child of node.children ?? []) walk(child as Node)
  })(template!.ast as unknown as Node)

  /** A directive on an element: `v-if` is `{name: 'if'}`, `:inert` is `{name: 'bind', arg: 'inert'}`. */
  function directive(node: Node, name: string, arg?: string) {
    return (node.props ?? []).find(
      (prop) => prop.type === 7 && prop.name === name && (arg === undefined || prop.arg?.content === arg),
    )
  }

  function textIn(node: Node): string {
    if (node.type === 2 || node.type === 5) return node.content ?? ''
    return (node.children ?? []).map((child) => textIn(child as Node)).join('')
  }

  // 1. The element that mounts the named component carries an `inert` binding whose VALUE is set
  //    for an inert box and unset for one that is not. Vue renders the attribute for a truthy
  //    value and omits it otherwise, so that is the property asserted.
  const mount = elements.find((node) =>
    (node.children ?? []).some(
      (child) =>
        (child as Node).type === 1 &&
        (child as Node).tag === 'component' &&
        !!directive(child as Node, 'bind', 'is'),
    ),
  )
  ok(mount, 'UiBox.vue mounts the named component with `<component :is>`')
  const bound = directive(mount!, 'bind', 'inert')
  ok(
    bound,
    'the element wrapping `<component :is>` carries no `inert` binding: every emitter panel — UiTable among them — would take input the canvas then discards',
  )
  const expression = bound!.exp?.content ?? ''
  ok(
    value(expression, { inert: true }),
    `\`:inert="${expression}"\` is not set for a box that IS inert, so every emitter panel stays interactive while the canvas discards what it emits`,
  )
  ok(
    !value(expression, { inert: false }),
    `\`:inert="${expression}"\` is set for a box that is NOT inert, which would freeze a panel nothing is wrong with`,
  )

  // 2. The note is shown for exactly the boxes that are inert — not for their complement, and not
  //    for none of them. Checked over every input, because "mentions `inert`" is satisfied by the
  //    inversion, which is the most misleading state available: a live panel calling itself display
  //    only.
  const note = elements.find(
    (node) =>
      (node.children ?? [])
        .map((child) => textIn(child as Node))
        .join('')
        .includes(NOTE) && !(node.children ?? []).some((child) => (child as Node).type === 1),
  )
  ok(note, 'an inert panel says why it does not respond')
  const shown = directive(note!, 'if') ?? directive(note!, 'show')
  ok(shown, 'the note is rendered unconditionally, so it would claim display-only for panels that are not')
  const condition = shown!.exp?.content ?? ''
  for (const inert of [true, false]) {
    for (const refusal of [undefined, 'refused']) {
      for (const feed of [undefined, 'no such view']) {
        const wanted = inert && !refusal && !feed
        equal(
          !!value(condition, { inert, refusal, feed }),
          wanted,
          `the note is shown for the wrong set: \`v-if="${condition}"\` is ${!wanted} for ` +
            `inert=${inert}, refusal=${JSON.stringify(refusal)}, feed=${JSON.stringify(feed)} — it must be shown for exactly the boxes that are inert and draw a component`,
        )
      }
    }
  }

  // 3. The `inert` the template binds is the contract's answer, for both answers. Evaluated against
  //    a stub `uiEmitters`, because matching the CALL accepts `uiEmitters.has(…) && false`, which
  //    is false for every box ever drawn.
  const body = computedBody(box.descriptor.scriptSetup?.content ?? '', 'inert')
  ok(body, 'UiBox.vue computes `inert`')
  const decide = (component?: string) =>
    value(body!, {
      spec: { value: component ? { component } : undefined },
      uiEmitters: new Set(['AnEmitter']),
    })
  equal(
    !!decide('AnEmitter'),
    true,
    '`inert` is false for a component the contract lists as an emitter, so the panel takes input the canvas discards',
  )
  equal(
    !!decide('NotAnEmitter'),
    false,
    '`inert` is true for a component that emits nothing, freezing a panel for no reason',
  )
  equal(!!decide(undefined), false, 'a box with no spec inerts nothing')
}


// Which components leave their own box — decided by what a component is ALLOWED to reach, not by
// a list of the ways one was once caught reaching.
//
// `inert` is an attribute on a DOM SUBTREE, so `UiBox.vue` can only make a panel read-only for a
// component that stays inside its own markup. The contract's `uiRefused` names the ones that do
// not. What decided that set was
//
//   /<Teleport\b|document\.|window\.addEventListener|window\.matchMedia/
//
// four literal spellings of the three things `UiModal` happens to do. `globalThis.document`, a
// `const doc = document` alias, `navigator.clipboard`, `window.top`, and a composable that does any
// of it one call away are all invisible to it, and invisible reads as SAFE: the component is
// rendered inert on the canvas instead of refused, which is the `UiModal` defect respelled. A
// denylist of escapes cannot fail closed, because the thing it misses is the thing it is for.
//
// So the question is turned around. A component may reach:
//
//   * its own locals, parameters and imports — whatever it declares, by any name;
//   * the compiler macros (`defineProps`, `defineEmits`, …), which are not values at all;
//   * the ECMAScript intrinsics, which are `Math`, `JSON`, `Date` and their kind — the globals that
//     compute and own nothing outside this process;
//   * `vue`, by NAMED import, less the three exports that mount markup somewhere else;
//   * a sibling file in this directory, WHOSE OWN REACH IS THEN THE COMPONENT'S — the reach is
//     closed transitively, so a helper that escapes takes everything importing it with it.
//
// Every other free identifier is an escape, whatever it is called, because it is a name this file
// resolves to nothing it was allowed to reach. `globalThis`, `navigator`, `localStorage`, `top`,
// `parent`, `fetch`, a global a browser adds next year: none of them are listed anywhere below,
// and that is exactly why they are caught.
//
// THE RULE THIS FILE HAD TO LEARN TWICE. An allowlist of names only fails closed for constructs the
// walk actually reaches. A construct it does not reach yields no reason, and no reason reads as
// SAFE — the same fail-open shape as the four literals, one level down. The first version of this
// file read `ImportDeclaration` nodes and free identifiers and nothing else, so `import('x')`, a
// `<script setup src="…">` whose content is another file, and `import * as vue from 'vue'` each
// produced silence; the first of those was demonstrated against THIS FILE, which reaches
// `@vue/compiler-sfc` by `await import` and was reported clean by its own guard. So the invariant
// below is not "name every escape" but: **where this file cannot resolve something, it says so.**
// Every module-naming construct an ES or TS module has is routed through `reachModule`, and an
// argument that is not a literal, a block whose `src` was not supplied, a namespace binding, an
// SFC that will not parse and a script that will not parse are each a reason rather than a gap.
//
// WHAT IT STILL DOES NOT CATCH, enumerated, because a guard whose edge is unstated is worse than a
// narrow one. This list said THREE once and there were six; that it presented itself as the edge
// and was not is the same defect the four literals had, so the list is the part of this file to
// distrust first. Routes 4 to 6 are measured, not suspected: each is pinned by a passing case in
// `escapes.guard.adversary2.test.ts`, which asserts the guard says NOTHING for it.
//
//   1. A DOM ELEMENT HANDLE reaches the whole document through its own properties —
//      `el.ownerDocument`, `el.getRootNode()`, `$event.target.ownerDocument`. Five shipping
//      components legitimately hold a handle (`UiTextInput`, `UiTextArea`, `UiNumberInput` from
//      `$event.target`; `UiFileList`, `UiTabs` from a template ref), so refusing handles refuses
//      most of the library. Closing it needs the properties reachable on a DOM-typed value
//      allowlisted FROM `lib.dom.d.ts` rather than named by hand, which is not this change.
//   2. PROPERTY ACCESS off an allowed value, beyond the two doors named in `PROTOTYPE_DOORS`.
//      `Object.constructor` is `Function` and one call from every withheld global, so it and
//      `__proto__` are reasons; `Object.getPrototypeOf(x).constructor` is caught by the same
//      property name, but a computed access whose key is only known at runtime —
//      `Object[someName]` — is not, and computed access cannot be refused outright because seven
//      shipping components index objects and arrays that way. Same closure condition as 1: the
//      members of an intrinsic have to come from the ECMAScript lib declarations, not a list here.
//   3. A component that escapes THROUGH A VALUE HANDED TO IT — a prop or a slot carrying a
//      function that reaches out. `UiBox.vue` binds props from `Box.props`, which are strings, so
//      nothing on the canvas can pass one today; a caller inside the app could.
//
// The next three are the SFC parts this file does not read as code, and they are one story:
// `story:the-guard-reads-no-template-or-style`. All three are `origin: pre-existing` — the four
// literal spellings this guard replaced missed every one of them too.
//
//   4. A TELEPORT SPELLED `is="vue:Teleport"`. The template walk below asks for two tag names,
//      `teleport` and `component`. `<div is="vue:Teleport" to="body">` is a native element that
//      `resolveComponentType` rewrites to the real `Teleport` before the built-in lookup, so the
//      shipped render function teleports and this file sees a `div`. Closing it means asking the
//      COMPILED render function what it mounts rather than asking the source what it is called.
//   5. TEMPLATE EXPRESSIONS. Not one is parsed: the walk reads tags, never `@click`, `:prop` or an
//      interpolation. `$root`, `$parent` and every other public instance property are available in
//      a template without being declared, imported or named in any script, so they are never
//      offered to the free-identifier walk. Reaching `$root` is outside this component's subtree by
//      definition. Closing it means compiling the template and running the same allowlist over the
//      generated code's free names.
//   6. `<style>` BLOCKS. `descriptor.styles` is never read. An SFC style without `scoped` emits its
//      selectors globally, so `body { overflow: hidden }` is UiModal's scroll lock performed in CSS
//      — outside the subtree `inert` covers, and outside the subtree any attribute covers. Every
//      shipping component writes `scoped` today and nothing requires it. Closing it means reading
//      each style block and refusing an unscoped one, or one whose selectors leave the component.
//
// Not named `*.test.ts`: node counts a test file with no cases as one passing test, and this file
// is imported by three — `index.test.ts`, `canvas/uibox.inert.test.ts` and
// `escapes.guard.adversary.test.ts` — which is the same reason `canvas/uibox.inert.guard.ts` gives.
// It imports no `node:` module and reads no file; the caller supplies the directory as text.

/** The compiler macros. Not values, not reachable at runtime, and never imported. */
const MACROS = new Set([
  'defineProps',
  'defineEmits',
  'defineSlots',
  'defineOptions',
  'defineExpose',
  'defineModel',
  'withDefaults',
])

/**
 * The globals that compute and own nothing outside this process.
 *
 * `globalThis` is deliberately absent: it is the door to every other global, and the first spelling
 * the four literals missed. So are `console`, `fetch`, `setTimeout` and every other host global — a
 * component needing one is a component whose reach has to be argued for.
 */
const INTRINSICS = new Set([
  'undefined',
  'NaN',
  'Infinity',
  'Math',
  'JSON',
  'Date',
  'Number',
  'String',
  'Boolean',
  'Array',
  'Object',
  'Set',
  'Map',
  'WeakMap',
  'WeakSet',
  'RegExp',
  'Error',
  'TypeError',
  'RangeError',
  'Symbol',
  'Promise',
  'BigInt',
  'Intl',
  'parseInt',
  'parseFloat',
  'isNaN',
  'isFinite',
  'encodeURIComponent',
  'decodeURIComponent',
])

/**
 * The two property names that turn any permitted value back into the globals the allowlist
 * withholds. `x.constructor` on anything at all is a function constructor —
 * `Object.constructor('return globalThis')()` hands back the global object — and `__proto__` walks
 * to it one hop further round. Neither is used by any component in this directory, and both are
 * fixed names in the language rather than a list that grows: this is not a denylist of escapes, it
 * is the ALLOWLIST's own leak, plugged where the language puts it.
 */
const PROTOTYPE_DOORS = new Set(['constructor', '__proto__'])

/** `vue` exports that put markup or a whole app somewhere this component's subtree is not. */
const VUE_ESCAPES = new Set(['Teleport', 'createApp', 'render'])

/** The module every component may import: the framework it is written in. */
const FRAMEWORK = 'vue'

interface Babel {
  type: string
  id?: unknown
  body?: Babel[]
  declaration?: Babel
  declarations?: { id: unknown }[]
  specifiers?: { type: string; local: { name: string }; imported?: { name?: string; value?: string } }[]
  source?: { value: string }
  moduleReference?: { type: string; expression?: { value?: string } }
}

interface TemplateNode {
  type: number
  tag?: string
  props?: {
    type: number
    name?: string
    arg?: { content?: string }
    exp?: { content?: string }
    value?: { content?: string }
  }[]
  children?: TemplateNode[]
}

/** Every file name a relative specifier could mean: `./icons` is `icons.ts`. */
function candidates(specifier: string): string[] {
  const bare = specifier.replace(/^\.\//, '')
  if (/\.(vue|ts|js|css)$/.test(bare)) return [bare]
  return [`${bare}.ts`, `${bare}.vue`, `${bare}.js`, `${bare}/index.ts`]
}

/**
 * Every node of a babel AST, in no particular order.
 *
 * `walkIdentifiers` answers about IDENTIFIERS, and the three constructs that broke this guard —
 * `import()`, `import.meta`, a namespace binding — contain none that mean what they do. So the
 * tree is walked whole, and anything with a `type` is offered to `visit`.
 */
function walkNodes(node: unknown, visit: (node: Record<string, unknown>) => void): void {
  if (!node || typeof node !== 'object') return
  if (Array.isArray(node)) {
    for (const child of node) walkNodes(child, visit)
    return
  }
  const shaped = node as Record<string, unknown>
  if (typeof shaped.type === 'string') visit(shaped)
  for (const key of Object.keys(shaped)) {
    if (key === 'loc' || key === 'extra' || key === 'leadingComments' || key === 'trailingComments') continue
    walkNodes(shaped[key], visit)
  }
}

/**
 * The reasons `file` leaves its own subtree, ignoring what it imports — that is closed afterwards.
 *
 * Anything this function cannot read is a reason, not a silence.
 */
async function directReach(
  file: string,
  source: string,
  supplied: ReadonlySet<string>,
): Promise<{ reasons: string[]; imports: string[] }> {
  const { parse, babelParse, extractIdentifiers, walkIdentifiers } = await import('@vue/compiler-sfc')
  const reasons: string[] = []
  const imports: string[] = []
  const note = (why: string) => {
    if (!reasons.includes(why)) reasons.push(why)
  }

  /**
   * A module this file names, however it names it.
   *
   * `whole` is for a binding that carries the module entire — a namespace import, a re-export, a
   * dynamic import — because the three `vue` exports this guard withholds are withheld BY NAME and
   * a whole-module binding restores every one of them.
   */
  const reachModule = (specifier: string | undefined, how: string, whole: boolean) => {
    if (specifier === undefined) {
      note(`${how} a module whose name is not a literal, so this guard cannot resolve what it reaches`)
      return
    }
    if (specifier === FRAMEWORK) {
      if (whole) {
        note(
          `${how} all of \`vue\`, which binds ${[...VUE_ESCAPES].join(', ')} — the exports that mount markup outside this component`,
        )
      }
      return
    }
    if (specifier.startsWith('./')) {
      const resolved = candidates(specifier).find((name) => supplied.has(name))
      if (resolved) imports.push(resolved)
      else note(`${how} \`${specifier}\`, whose source this guard was not given`)
      return
    }
    note(`${how} \`${specifier}\`, which is outside this directory and not the framework`)
  }

  // 1. The SFC blocks. A block with a `src` has its content in ANOTHER FILE, and the descriptor's
  //    `content` for it is the empty string — which used to walk to no script, no reasons, and a
  //    component reported as declaring nothing and reaching nothing.
  let script = source
  if (file.endsWith('.vue')) {
    const parsed = parse(source, { filename: file })
    if (parsed.errors.length) {
      note(`has an SFC this guard cannot parse: ${String(parsed.errors[0])}`)
    }
    const { descriptor } = parsed
    for (const [what, block] of [
      ['takes its script from', descriptor.script],
      ['takes its setup script from', descriptor.scriptSetup],
      ['takes its template from', descriptor.template],
    ] as const) {
      if (block?.src) reachModule(block.src.startsWith('.') ? block.src : `./${block.src}`, what, true)
    }
    script = [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n;\n')

    // 2. The template. A teleport renders elsewhere by definition; a computed `:is` mounts
    //    something whose identity is not in this file.
    if (descriptor.template && !descriptor.template.ast && !descriptor.template.src) {
      note('has a template this guard could not read, so what it renders is unknown')
    }
    if (descriptor.template?.ast) {
      const walk = (node: TemplateNode) => {
        if (node.type === 1) {
          if (/^teleport$/i.test(node.tag ?? '')) {
            note('renders a <Teleport>, whose content is a child of another element')
          }
          if (/^component$/i.test(node.tag ?? '')) {
            const is = (node.props ?? []).find(
              (prop) =>
                (prop.type === 7 && prop.name === 'bind' && prop.arg?.content === 'is') || prop.name === 'is',
            )
            const literal = /^'[^']*'$|^"[^"]*"$/.test((is?.exp?.content ?? is?.value?.content ?? '').trim())
            if (!literal) {
              note('mounts <component :is> on an expression, so what it renders is not decidable here')
            }
          }
        }
        for (const child of node.children ?? []) walk(child as TemplateNode)
      }
      walk(descriptor.template.ast as unknown as TemplateNode)
    }
  }

  if (!script.trim()) return { reasons, imports }

  let program: Babel
  try {
    program = babelParse(script, { sourceType: 'module', plugins: ['typescript'] }).program as unknown as Babel
  } catch (error) {
    // Fail closed. A script this guard cannot read is a script whose reach it does not know, and
    // "unknown" is the state the four literals were in about `globalThis`.
    return { reasons: [...reasons, `has a script this guard cannot parse: ${(error as Error).message}`], imports }
  }

  // 3. The module-level declarations, and every construct that names another module. An ES or TS
  //    module has five of them and all five are here: `import`, `export … from`, `export * from`,
  //    `import x = require(…)`, and the dynamic `import()` picked up by the walk below.
  const declared = new Set<string>()
  const bindings = (node: Babel) => {
    if (node.type === 'VariableDeclaration') {
      for (const one of node.declarations ?? []) {
        for (const id of extractIdentifiers(one.id as never)) declared.add(id.name)
      }
    } else if (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') {
      const id = node.id as { name?: string } | undefined
      if (id?.name) declared.add(id.name)
    }
  }

  for (const node of program.body ?? []) {
    if (node.type === 'ImportDeclaration') {
      const specifier = node.source?.value ?? ''
      for (const one of node.specifiers ?? []) declared.add(one.local.name)
      const named = (node.specifiers ?? []).every((one) => one.type === 'ImportSpecifier')
      if (specifier === FRAMEWORK && named) {
        for (const one of node.specifiers ?? []) {
          const imported = one.imported?.name ?? one.imported?.value
          if (imported && VUE_ESCAPES.has(imported)) {
            note(`imports \`${imported}\` from vue, which mounts markup outside this component`)
          }
        }
      }
      reachModule(specifier, 'imports', !named)
    } else if (node.type === 'ExportAllDeclaration') {
      reachModule(node.source?.value, 're-exports all of', true)
    } else if (node.type === 'ExportNamedDeclaration') {
      if (node.source) reachModule(node.source.value, 're-exports from', true)
      else if (node.declaration) bindings(node.declaration)
    } else if (node.type === 'ExportDefaultDeclaration') {
      if (node.declaration) bindings(node.declaration)
    } else if (node.type === 'TSImportEqualsDeclaration') {
      const id = node.id as { name?: string } | undefined
      if (id?.name) declared.add(id.name)
      reachModule(node.moduleReference?.expression?.value, 'requires', true)
    } else {
      bindings(node)
    }
  }

  // 4. The constructs that reach without naming an identifier: `import()`, `import.meta`, and the
  //    two property names that hand a permitted value back as `Function`.
  walkNodes(program, (node) => {
    if (node.type === 'CallExpression' || node.type === 'OptionalCallExpression') {
      const callee = node.callee as { type?: string } | undefined
      if (callee?.type === 'Import') {
        const argument = (node.arguments as { type?: string; value?: unknown }[] | undefined)?.[0]
        const literal = argument?.type === 'StringLiteral' ? String(argument.value) : undefined
        reachModule(literal, 'imports at runtime', true)
      }
    }
    if (node.type === 'ImportExpression') {
      const argument = node.source as { type?: string; value?: unknown } | undefined
      const literal = argument?.type === 'StringLiteral' ? String(argument.value) : undefined
      reachModule(literal, 'imports at runtime', true)
    }
    if (node.type === 'MetaProperty' && (node.meta as { name?: string } | undefined)?.name === 'import') {
      note('reads `import.meta`, which carries the build environment and a loader for every module in the tree')
    }
    if (node.type === 'MemberExpression' || node.type === 'OptionalMemberExpression') {
      const property = node.property as { type?: string; name?: string; value?: unknown } | undefined
      const name = node.computed
        ? property?.type === 'StringLiteral'
          ? String(property.value)
          : undefined
        : property?.name
      if (name && PROTOTYPE_DOORS.has(name)) {
        note(`reads \`.${name}\`, which returns Function or the prototype chain and reaches every global from any value`)
      }
    }
  })

  // 5. The free identifiers: referenced, declared nowhere in the file, and not a macro or an
  //    intrinsic — so a name that resolves outside this component.
  const free = new Set<string>()
  walkIdentifiers(
    program as never,
    (node, _parent, _stack, isReferenced, isLocal) => {
      const name = (node as { name: string }).name
      if (!isReferenced || isLocal) return
      if (declared.has(name) || MACROS.has(name) || INTRINSICS.has(name)) return
      free.add(name)
    },
    true,
  )
  for (const name of [...free].sort()) {
    note(`reads \`${name}\`, which it neither declares nor imports, so it is a global`)
  }

  return { reasons, imports }
}

/**
 * For every file supplied, why it leaves its own subtree — an empty list meaning it does not.
 *
 * `sources` is a directory as text: `{'UiModal.vue': '<script setup…', 'icons.ts': '…'}`. Reach is
 * closed over the relative imports between them, so a component escapes if it escapes itself OR if
 * anything it imports, at any depth, does.
 */
export async function escapingComponents(sources: Record<string, string>): Promise<Record<string, string[]>> {
  const supplied = new Set(Object.keys(sources))
  const direct: Record<string, { reasons: string[]; imports: string[] }> = {}
  for (const [file, source] of Object.entries(sources)) {
    // A stylesheet declares no names and calls nothing; every other kind of file is read.
    direct[file] = file.endsWith('.css') ? { reasons: [], imports: [] } : await directReach(file, source, supplied)
  }

  const closed: Record<string, string[]> = {}
  for (const file of Object.keys(sources)) {
    const reasons: string[] = [...direct[file].reasons]
    const seen = new Set([file])
    const queue = [...direct[file].imports]
    while (queue.length) {
      const next = queue.shift()!
      if (seen.has(next)) continue
      seen.add(next)
      const reached = direct[next]
      if (!reached) {
        // Unreachable as long as an edge only ever names a supplied file, which is what
        // `reachModule` guarantees — and said here anyway, because the defect this guard was
        // corrected for was a missing branch reading as "nothing to report".
        reasons.push(`through \`${next}\`, whose reach this guard never computed`)
        continue
      }
      for (const reason of reached.reasons) reasons.push(`through \`${next}\`, which ${reason}`)
      queue.push(...reached.imports)
    }
    closed[file] = reasons
  }
  return closed
}

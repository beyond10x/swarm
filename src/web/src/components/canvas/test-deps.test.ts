// Adversarial, pass 2: `npm test` now imports a package the manifest does not declare.
//
// Before this unit, every file the suite ran imported only `node:` builtins and read `.vue` and
// `.ts` files off disk — `npm test` was runnable against a bare checkout. `uibox.inert.test.ts`
// now does `await import('@vue/compiler-sfc')`, and `@vue/compiler-sfc` appears in neither
// `dependencies` nor `devDependencies` of `src/web/package.json`. It resolves today only because
// npm hoists it out of `vue`'s own dependency tree into the flat `node_modules` root.
//
// That is a resolution accident, not a contract: a package manager with a strict store (pnpm,
// yarn PnP), or a `vue` release that stops depending on it, or an `npm install --omit=dev` tree,
// turns the gate command into ERR_MODULE_NOT_FOUND. The fix is one line in `package.json`
// devDependencies, and the version has to be pinned to the `vue` version anyway — the compiler and
// the runtime are released in lockstep and mismatching them is its own failure.
//
// So the rule is asserted for the whole suite rather than for the one import: a test file may
// import a `node:` builtin, a relative path, the `@/` alias, or a package the manifest declares.
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url)).replace(/\/$/, '')

const manifest = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')) as {
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
}
const declared = new Set([
  ...Object.keys(manifest.dependencies ?? {}),
  ...Object.keys(manifest.devDependencies ?? {}),
])

// Widened after the same defect reappeared one file away: the fix for the undeclared import moved
// the guard into `uibox.inert.guard.ts`, which is NOT a `*.test.ts` and imports the same package.
// The rule is about anything the gate loads, so every `.ts` and `.vue` under `src/` is read —
// `npx vite build` and `npx vue-tsc --noEmit` resolve those, and an undeclared import breaks them
// in the same way and for the same reason.
function sourceFiles(dir: string): string[] {
  const found: string[] = []
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name)
    if (entry.isDirectory()) found.push(...sourceFiles(path))
    else if (entry.name.endsWith('.ts') || entry.name.endsWith('.vue')) found.push(path)
  }
  return found
}

/** Comments out, so a specifier QUOTED in prose is not read as one the file imports. */
function code(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^[ \t]*\/\/.*$/gm, '')
}

/** The bare package specifiers a source imports, static and dynamic: `@vue/compiler-sfc`. */
function packagesIn(text: string): string[] {
  const source = code(text)
  const specifiers = [
    ...source.matchAll(/(?:^|[^\w.])import\s+[^'"]*?from\s*['"]([^'"]+)['"]/gm),
    ...source.matchAll(/\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g),
  ].map((match) => match[1])
  return specifiers
    .filter((name) => !name.startsWith('node:') && !name.startsWith('.') && !name.startsWith('@/'))
    .map((name) => (name.startsWith('@') ? name.split('/').slice(0, 2).join('/') : name.split('/')[0]))
}

test('every package the suite and the build import is declared in package.json', () => {
  const undeclared = sourceFiles(join(ROOT, 'src'))
    .flatMap((file) =>
      [...new Set(packagesIn(readFileSync(file, 'utf8')))]
        .filter((name) => !declared.has(name))
        .map((name) => `${file.slice(ROOT.length + 1)} imports ${name}`),
    )
    .sort()
  assert.deepEqual(
    undeclared,
    [],
    '`npm test` is the package gate; a package it imports and the manifest does not declare is ' +
      'resolved only by npm hoisting it out of another package, and is not there under a strict ' +
      'store, under `--omit=dev`, or after the package that pulled it in drops it',
  )
})

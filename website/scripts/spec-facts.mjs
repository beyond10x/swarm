#!/usr/bin/env node
// Derive every number the website states from the tree it describes.
//
// The failure this exists to prevent already happened: the page shipped "5 domains, 8 entities, 48
// commands, 22 views" and was wrong about all four from the moment a mailbox domain landed, because
// the numbers were typed. Nothing noticed for the page's entire published life. So no count on this
// site is written by hand any more — this script reads them, and the page imports what it wrote.
//
// It derives three classes of fact:
//
//   1. specification element counts, from `ess specify compile --path ../src/core --format json`
//   2. hand-written sizes, from counting lines in the tree
//   3. the two surfaces the runtime hand-writes, from the files that declare them
//
// Class 3 is parsed out of Rust source, which is the fragile part, so every anchor it looks for is
// required: a missing `pub fn routes` or `enum Verb` is a hard failure, never a zero. A derivation
// that silently returns the wrong number is worse than one that was typed, because it looks fresh.
//
// Usage:
//   node scripts/spec-facts.mjs            regenerate src/data/spec-facts.json
//   node scripts/spec-facts.mjs --check    fail if the committed file is stale; write nothing
//
// If `ess` is not on PATH the plain form warns loudly and leaves the committed file in place, so
// the site still builds from a bare checkout with no Rust toolchain (which is what GitHub Pages
// has). `--check` refuses in that case rather than reporting a green it did not verify.

import {execFileSync} from 'node:child_process';
import {readFileSync, writeFileSync, existsSync, readdirSync, statSync} from 'node:fs';
import {dirname, join, resolve, extname} from 'node:path';
import {fileURLToPath} from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const WEBSITE = resolve(HERE, '..');
const REPO = resolve(WEBSITE, '..');
const SPEC = join(REPO, 'src', 'core');
const OUT = join(WEBSITE, 'src', 'data', 'spec-facts.json');

const check = process.argv.includes('--check');

function fail(message) {
  console.error(`spec-facts: ${message}`);
  process.exit(1);
}

/** Every file under `dir` whose extension is in `exts`, recursively. */
function walk(dir, exts, found = []) {
  if (!existsSync(dir)) fail(`no such directory: ${dir}`);
  for (const entry of readdirSync(dir, {withFileTypes: true})) {
    const at = join(dir, entry.name);
    if (entry.isDirectory()) walk(at, exts, found);
    else if (exts.includes(extname(entry.name))) found.push(at);
  }
  return found;
}

/** `wc -l` semantics: how many newline-terminated lines these files hold in total. */
function lines(files) {
  if (files.length === 0) fail('counted zero files; the tree is not where this script expects it');
  let total = 0;
  for (const file of files) {
    const text = readFileSync(file, 'utf8');
    total += text.length === 0 ? 0 : text.split('\n').length - (text.endsWith('\n') ? 1 : 0);
  }
  return total;
}

/** Read `file`, or fail naming it — an absent source file must never become a zero. */
function source(rel) {
  const at = join(REPO, rel);
  if (!existsSync(at)) fail(`expected ${rel} to exist; it does not`);
  return readFileSync(at, 'utf8');
}

/** How many routes the server declares, counted inside its own route table. */
function httpRoutes() {
  const rel = 'src/runtime/swarm-server/src/http.rs';
  const text = source(rel);
  const start = text.indexOf('pub fn routes(');
  if (start === -1) fail(`${rel} no longer declares \`pub fn routes(\`; teach this script the new shape`);
  const end = text.indexOf('\n}', start);
  const body = text.slice(start, end === -1 ? text.length : end);
  const count = (body.match(/\.route\(/g) || []).length;
  if (count === 0) fail(`${rel} declares \`pub fn routes\` with no routes in it`);
  return count;
}

/** How many verbs the CLI offers, counted inside its own `Verb` enum. */
function cliVerbs() {
  const rel = 'src/runtime/swarm-cli/src/main.rs';
  const text = source(rel);
  const start = text.indexOf('enum Verb {');
  if (start === -1) fail(`${rel} no longer declares \`enum Verb {\`; teach this script the new shape`);
  const end = text.indexOf('\n}', start);
  const body = text.slice(start, end === -1 ? text.length : end);
  // A variant is a CamelCase identifier at one level of indentation, opening a block or ending bare.
  const count = (body.match(/^ {4}[A-Z][A-Za-z0-9]*\s*(\{|,)\s*$/gm) || []).length;
  if (count === 0) fail(`${rel} declares \`enum Verb\` with no variants in it`);
  return count;
}

/** The ESS format the specification declares, e.g. `ess/4`, read from the system document. */
function specFormat() {
  const rel = 'src/core/system.yaml';
  const found = source(rel).match(/^format:\s*(ess\/\d+)\s*$/m);
  if (!found) fail(`${rel} does not declare a \`format: ess/N\`; it must, and the site quotes it`);
  return found[1];
}

/** The specification, compiled — or `null` when `ess` is not installed. */
function compileSpec() {
  try {
    const json = execFileSync('ess', ['specify', 'compile', '--path', SPEC, '--format', 'json'], {
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    });
    const version = execFileSync('ess', ['--version'], {encoding: 'utf8'}).trim();
    const validate = execFileSync('ess', ['specify', 'validate', '--path', SPEC], {encoding: 'utf8'}).trim();
    return {ir: JSON.parse(json), version, validate};
  } catch (error) {
    if (error && (error.code === 'ENOENT' || error.code === 'EACCES')) return null;
    fail(`\`ess specify\` failed against ${SPEC}:\n${error.stderr || error.message}`);
  }
}

function derive() {
  const compiled = compileSpec();
  if (!compiled) return null;
  const {ir, version, validate} = compiled;

  const size = (value) => (Array.isArray(value) ? value.length : Object.keys(value || {}).length);
  const short = (qualified) => qualified.slice(qualified.lastIndexOf('.') + 1);

  // `9 file(s), valid` — the file count is the specification's own, not a guess from the tree.
  const files = Number((validate.match(/(\d+) file\(s\)/) || [])[1]);
  if (!Number.isFinite(files) || files === 0) {
    fail(`could not read a file count out of \`ess specify validate\`: ${validate}`);
  }

  const counts = {};
  for (const key of ['domains', 'entities', 'types', 'commands', 'events', 'errors', 'views', 'actors', 'bindings']) {
    counts[key] = size(ir[key]);
    if (counts[key] === 0) fail(`the compiled specification holds zero ${key}`);
  }

  const domains = Object.values(ir.domains)
    .map((domain) => ({
      name: domain.name,
      display: domain.naming?.display ?? domain.name,
      summary: domain.naming?.summary ?? '',
      entities: (domain.entities || []).map(short),
      commands: size(domain.commands),
      events: size(domain.events),
      views: size(domain.views),
    }))
    .sort((a, b) => b.commands - a.commands || a.name.localeCompare(b.name));

  const rs = ['.rs'];
  const runtime = {
    essRuntime: lines(walk(join(REPO, 'src/runtime/ess-runtime/src'), rs)),
    swarmServer: lines(walk(join(REPO, 'src/runtime/swarm-server/src'), rs)),
    swarmCli: lines(walk(join(REPO, 'src/runtime/swarm-cli/src'), rs)),
  };
  const testFiles = ['ess-runtime', 'swarm-server', 'swarm-cli']
    .map((crate) => join(REPO, 'src/runtime', crate, 'tests'))
    .filter((at) => existsSync(at) && statSync(at).isDirectory())
    .flatMap((at) => walk(at, rs));

  return {
    _comment:
      'Generated by website/scripts/spec-facts.mjs. Do not edit: `npm run spec:facts` rewrites it, ' +
      'and `npm run spec:check` fails when it is stale. Every number the site publishes lives here.',
    system: ir.system,
    specVersion: ir.version,
    specFormat: specFormat(),
    summary: ir.summary,
    essVersion: version,
    specFiles: files,
    validates: /valid/.test(validate),
    counts,
    domains,
    surfaces: {httpRoutes: httpRoutes(), cliVerbs: cliVerbs()},
    lines: {
      ...runtime,
      runtimeTotal: runtime.essRuntime + runtime.swarmServer + runtime.swarmCli,
      integrationTests: lines(testFiles),
      integrationTestFiles: testFiles.length,
      web: lines(walk(join(REPO, 'src/web/src'), ['.vue', '.ts', '.css'])),
      specYaml: lines(walk(SPEC, ['.yaml'])),
    },
  };
}

// ---------------------------------------------------------------------------
// The page imports its numbers, so it cannot go stale. Prose outside the page
// cannot import anything, and the root README states figures in words. Those are
// the same defect class as the four wrong counts this script exists to prevent,
// so they are checked too rather than trusted.

const ONES = [
  'zero', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten',
  'eleven', 'twelve', 'thirteen', 'fourteen', 'fifteen', 'sixteen', 'seventeen', 'eighteen',
  'nineteen',
];
const TENS = ['', '', 'twenty', 'thirty', 'forty', 'fifty', 'sixty', 'seventy', 'eighty', 'ninety'];

/** English for 0–99, e.g. 53 -> `fifty-three`. Beyond that, digits only. */
function words(value) {
  if (value < 20) return ONES[value];
  if (value > 99) return null;
  const tens = TENS[Math.floor(value / 10)];
  const ones = value % 10;
  return ones === 0 ? tens : `${tens}-${ONES[ones]}`;
}

/** Every written form of `value` the prose is allowed to use. */
function forms(value) {
  const out = new Set([String(value), value.toLocaleString('en-US')]);
  const word = words(value);
  if (word) {
    out.add(word);
    out.add(word[0].toUpperCase() + word.slice(1));
  }
  return [...out];
}

/** Fail unless `file` states `value` in one of its accepted forms. */
function assertProse(rel, label, value) {
  const text = source(rel);
  const accepted = forms(value);
  const hit = accepted.some((form) => new RegExp(`(^|[^\\w,.-])${form}([^\\w]|$)`, 'i').test(text));
  if (!hit) {
    fail(
      `${rel} no longer states the right ${label}.\n` +
        `        derived: ${value} — expected one of: ${accepted.join(', ')}\n` +
        `        Update the prose, or teach this script the new wording.`,
    );
  }
}

/** Round to the nearest hundred — how the README states approximate sizes. */
const hundreds = (value) => Math.round(value / 100) * 100;

function checkProse(f) {
  assertProse('README.md', 'domain count', f.counts.domains);
  assertProse('README.md', 'command count', f.counts.commands);
  assertProse('README.md', 'view count', f.counts.views);
  assertProse('README.md', 'specification file count', f.specFiles);
  assertProse('README.md', 'specification size (nearest 100 lines)', hundreds(f.lines.specYaml));
  assertProse('README.md', 'runtime size (nearest 100 lines)', hundreds(f.lines.runtimeTotal));
  assertProse('AGENTS.md', 'specification file count', f.specFiles);
}

const fresh = derive();
const committed = existsSync(OUT) ? readFileSync(OUT, 'utf8') : null;

if (!fresh) {
  const banner = '='.repeat(78);
  if (check) {
    fail(
      `\n${banner}\n\`ess\` is not on PATH, so the committed spec facts cannot be checked.\n` +
        `Install it (https://github.com/beyond10x/ess) and re-run \`npm run spec:check\`.\n${banner}`,
    );
  }
  if (!committed) {
    fail('`ess` is not on PATH and there is no committed src/data/spec-facts.json to fall back to');
  }
  console.warn(
    `\n${banner}\nspec-facts: \`ess\` is not on PATH. Building from the COMMITTED numbers in\n` +
      `  src/data/spec-facts.json\nThey are only as fresh as the last \`npm run spec:facts\`. ` +
      `The gate runs \`npm run spec:check\`,\nwhich fails when they drift.\n${banner}\n`,
  );
  process.exit(0);
}

const rendered = `${JSON.stringify(fresh, null, 2)}\n`;

if (check) {
  if (committed !== rendered) {
    fail(
      'src/data/spec-facts.json is stale — the specification or the tree moved under it.\n' +
        '        Run `npm run spec:facts` and commit the result.',
    );
  }
  checkProse(fresh);
  console.log(
    `spec-facts: current — ${fresh.counts.domains} domains, ${fresh.counts.commands} commands, ` +
      `${fresh.counts.views} views, from ${fresh.specFiles} specification file(s). ` +
      `README.md and AGENTS.md agree.`,
  );
  process.exit(0);
}

if (committed === rendered) {
  console.log('spec-facts: src/data/spec-facts.json already current.');
} else {
  writeFileSync(OUT, rendered);
  console.log(
    `spec-facts: wrote src/data/spec-facts.json — ${fresh.counts.domains} domains, ` +
      `${fresh.counts.commands} commands, ${fresh.counts.views} views, ` +
      `${fresh.lines.runtimeTotal} lines of runtime.`,
  );
}

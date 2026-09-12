# website

The public site at [beyond10x.github.io/swarm](https://beyond10x.github.io/swarm/). Docusaurus 3.10.2, React 19,
`future: {v4: true}`, Node >= 20. `docs` and `blog` are both off: this is a single page, and the page
is `src/pages/index.js`.

## Running it

```console
npm install
npm run start      # dev server, hot reload
npm run build      # production build into build/
npm run serve      # serve what build/ holds
```

`onBrokenLinks: 'throw'` is on, so a link that does not resolve fails the build rather than shipping.

## Every number on the page is derived

This is the part to read before editing anything.

`scripts/spec-facts.mjs` runs before `start` and before `build`, shells
`ess specify compile --path ../src/core --format json`, counts lines in the runtime and the
specification, parses the route table and the CLI verb enum, and writes `src/data/spec-facts.json`.
The page imports that file. **No count is typed into the page, and none may be.**

```console
npm run spec:facts   # rederive and rewrite src/data/spec-facts.json
npm run spec:check   # fail if the committed file is stale; write nothing
```

The reason is specific. The page this replaced published "5 domains, 8 entities, 48 commands, 22
views" and every one of those four was wrong from the moment a mailbox domain landed, minutes after
somebody typed them. Nothing noticed for the page's entire published life. Typed numbers are a
defect class, not a mistake, so the fix is the derivation and the check, not a correction.

**`ess` on `PATH` is optional at build time, deliberately.** When it is missing the script prints a
loud banner and builds from the committed `src/data/spec-facts.json`, so the site still publishes
from a bare checkout with no Rust toolchain — which is what the GitHub Pages runner has. `--check`
refuses in that case rather than reporting a green it could not verify, and it is the gate step that
catches drift where `ess` does exist. `src/data/spec-facts.json` is committed on purpose; treat it as
generated output and never hand-edit it.

If a new fact needs to reach the page, teach the script to derive it. Every anchor the script parses
out of Rust source is required — a missing `pub fn routes` or `enum Verb` is a hard failure, never a
silent zero, because a derivation that returns the wrong number is worse than one that was typed: it
looks fresh.

## Assets

`static/img/` holds only what is used: `favicon.ico`, `logo.svg`, and `social-card.png`, the Open
Graph image `docusaurus.config.js` points at. The five stock Docusaurus assets the scaffold shipped
were unreferenced and are gone.

The social card is generated from a source kept outside `static/` so it is not also served:

```console
rsvg-convert -w 1200 -h 630 -o static/img/social-card.png assets/social-card.svg
```

## Honesty

The site makes claims about a real system and was audited against the tree. The rules that audit
produced live in the repository's [`AGENTS.md`](../AGENTS.md) under "Honesty rules for anything
published" — read them before writing a sentence about what Swarm does. The short version: it is
interpreted, not code-generated; it is a swarm *manager*, and multi-agent operation is specified but
not demonstrated; nothing confines a coordinator.

## Deploying

GitHub Pages, from `build/`, by this repository's own `.github/workflows/deploy-website.yml`.
`static/.nojekyll` is present. `url` is `https://beyond10x.github.io` and `baseUrl` is `/swarm/`,
which is the address the site is actually served at. There is no custom domain: the one the
scaffold import named had no DNS record and no delegation, and every reference to it is gone.

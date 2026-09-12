# What the 2026-09-12b bug wave left behind

The wave page is `../2026-09-12-open-bugs.md`. This directory holds the things that would otherwise
have lived only in a worktree or a scratch directory, and would have gone with them.

| file | what it is |
|---|---|
| `unit-a-brief.md`, `unit-b-brief.md`, `unit-c-brief.md` | the brief each implementor was dispatched with, byte for byte. It is the only record of what an agent was *told*, as against what it did |
| `slug-validation-cases.rs` | the two adversary cases that measured `story:slug-is-not-validated`. Unit B lifted them out of `swarm-server`'s suite rather than weakening them; they need the `kernel()` and `server()` helpers from `tests/open_is_one_swarm_under_contention.rs` and `use std::sync::Arc;` to compile |

Everything else the wave's scratch directories held was reproducible from the tree and was removed:
mutation copies of `src/web/src`, `.orig` backups, per-run test and build logs, and one probe
script. The numbers those logs carried are in the wave page, and the gate re-runs on `main`.

`~/.cache/swarm-wave-2026-09-12/` — the **previous** wave's scratch — is deliberately untouched. Its
own page says the logs its agents wrote live there and survive the trees, and removing them would
make that page cite nothing.

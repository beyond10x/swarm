# Every commit in this repository was rewritten on 2026-09-12

## What happened

All 59 commits were rewritten with `git filter-repo` so that the author and committer of every one
is `b10x-bot[bot] <316511680+b10x-bot[bot]@users.noreply.github.com>` — the identity
`~/beyond10x/atlas` commits under. Nothing else changed: no tree, no message, no parent structure.
The commit count is 59 before and after.

**Every commit hash changed.** That is what a rewrite is, and it is why this page exists: the two
wave pages, the planning store's `test_result` evidence and several commit messages cite hashes
that no longer resolve.

`rewrite-commit-map.txt` in `2026-09-12b-evidence/` is `filter-repo`'s own map, old on the left and
new on the right, one line per commit. Every citation written before 2026-09-12 can be resolved
through it.

The ones a reader is most likely to hit:

| cited as | now |
|---|---|
| `b66ef29` — `main` before the bug wave | `7f07f11` |
| `0c24ac0` — the bug wave's opening commit, the base every unit forked from | `9365d85` |
| `e314c1d`, `61a196b`, `51905ea` — unit A's three commits | `51d2978`, `e8caac8`, `55277d2` |
| `8cabdd7`, `8224c88`, `50e425b` — unit B's three | `f05e1fc`, `cb1c347`, `42bed7f` |
| `ad7be5f`, `123210c`, `4b42470` — unit C's three | `7b85b51`, `9b6d6c7`, `9409c24` |
| `c0b4160` — the commit the whole gate was green on | `2f89d2d` |

A pre-rewrite bundle of every ref is at `~/.cache/swarm-prerewrite-20260912T103912Z.bundle`, 898K.
It is outside the repository and outside any backup this repository controls.

## What it was for

Until 2026-09-12 this repository had no remote at all. `worktree gc` refuses a tree whose commit is
not reachable from an advertised remote ref, so every wave's worktrees and branches were retained on
that one condition — five trees and seven branches by the time the bug wave closed, and the count
was only going to grow.

`origin` is now `git@github.com:beyond10x/swarm.git`, private. With `main` published, all five trees
became eligible and were removed through `worktree gc --apply`, and all seven merged branches were
deleted with `git branch -d`. The repository has one branch.

## What this does not fix

The rewrite gives every commit one author, which is honest about who wrote them and is the
convention `atlas` already uses. It does not make the earlier citations correct — it makes them
resolvable through a file. A page written before today and read after it needs this map, and nothing
enforces that.

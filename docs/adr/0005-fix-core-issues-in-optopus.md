# 0005. Fix problems in the optopus core upstream

- Date: 2026-09-29
- Status: Proposed

## Context

optopus-py binds [optopus](https://github.com/trash-iine/optopus), vendored as a git submodule.
Both repositories are maintained by the same project, but the binding was written as if the core
were a third-party dependency: `vendor/optopus` is never edited here, and a problem that showed
up through the binding was handled in the binding.

That broke down on the tabu keys of problems written in Python (#16). The binding mapped an int
key to `TabuKey::Var`, which the core kept in an array as long as the largest key, so a key such
as `item * 10**15 + bin` aborted the Python process. The first fix disguised large ints as a
`TabuKey::Triple` the core keeps in its map. It worked, but it only existed because the core had
no single-index key kept in the map, and it left a collision with user tuples that was argued
away rather than ruled out. Fixing the core instead (trash-iine/optopus#68: `Var` is kept in the
map and the array key is `DenseVar`) removed the workaround and fixed the same trap for Rust
users of the core.

## Options considered

- **Work around core problems in the binding.** Keeps every change in one repository and one
  PR, but the workaround is shaped by what the core happens to offer, the defect stays for
  every other user of the core, and the binding accumulates code whose only reason is a core
  gap.
- **Patch the core inside `vendor/optopus`.** Fast, but the submodule then points at a commit
  that exists nowhere upstream, and the next update of the submodule silently drops or
  conflicts with the patch.
- **Fix the core upstream, then move the submodule to the merged fix.** Two PRs and a wait for
  the upstream merge, but the fix lands where the defect is, is reviewed under the core's own
  conventions, and the binding changes only in what the new core API requires.

## Decision

A problem whose cause is in the core is fixed in trash-iine/optopus, not worked around here. The
binding PR moves `vendor/optopus` to the commit that merged the fix into optopus's `main`, never
to an unmerged branch, and makes only the changes the new core requires.

The binding keeps what is its own job: turning inputs the core would panic on into
`ValueError` / `TypeError`, and Python-facing behavior such as docstrings and error messages.

## Consequences

- A binding fix that needs a core change waits for the upstream merge. The binding PR may be
  opened against the upstream branch for review, but is merged only after the submodule points
  at optopus's `main`.
- The upstream PR follows optopus's conventions (its commit style, its `decisions/` records,
  its English and Japanese docs), and each PR links the other.
- The submodule update stays in its own commit, as CONTRIBUTING.md already asks. #16 combined it
  with the binding change; later PRs keep them apart.
- When a workaround in the binding is unavoidable for now, it says in a comment which core
  change would remove it, and an issue is filed upstream.

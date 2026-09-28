# 0002. Limit Invoke tasks to composite workflows

- Date: 2026-09-28
- Status: Accepted

## Context

`tasks.py` could either wrap single commands one-to-one (`invoke test` for `pytest`) or hold only
workflows that take several steps or need project-specific knowledge. python-project-template went
through both and settled on the second. Two things make it matter here too:

- CI runs six checks across two toolchains (`cargo fmt`, `cargo clippy`, `ruff check`,
  `ruff format`, a `maturin develop` build and `pytest`), and seeing every failure at once rather
  than stopping at the first saves round trips.
- Some steps have traps that are easy to forget: the API reference is generated from the compiled
  extension, so the docs need a fresh `maturin develop` first, and a `uv run` between that build
  and `pytest` or `sphinx-build` reinstalls a cached wheel over it.

## Options considered

- **One-to-one wrappers plus composite tasks**: discoverable through `invoke --list`, but
  `uv run invoke test` saves nothing over `uv run pytest`, and there is no rule for what belongs.
- **Composite workflows only**: a task exists only when it bundles several steps or encodes
  project knowledge (`ci`, `fix`, `docs`, `adr`, `set-version`, `check-version`); single commands
  are run directly.

## Decision

Composite workflows only. CI keeps its individual workflow steps instead of calling `invoke ci`,
so the Actions UI shows which check failed.

## Consequences

- `CI_CHECKS` in `tasks.py` mirrors `.github/workflows/ci.yml`; change both together.
- Claude Code skills call these tasks rather than repeating their steps in prose, so humans and
  agents use the same entry points.
- Thin single-command wrappers are not added to `tasks.py`.

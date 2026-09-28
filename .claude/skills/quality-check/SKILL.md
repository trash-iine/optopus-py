---
name: quality-check
description: Run the same checks as CI (cargo fmt, clippy, ruff check, ruff format, the maturin build, pytest) through `invoke ci`, then fix failures until everything passes. Use before committing or pushing, or when asked to "run the checks", "make CI green", or "fix lint".
allowed-tools: Bash(uv run *) Bash(cargo *) Read Edit
---

# Quality check

Run the CI checks locally and keep fixing until all of them pass.

## 1. Run the checks

```bash
uv run invoke ci
```

It runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `ruff check .`,
`ruff format --check .`, `maturin develop` and `pytest`, keeps going past failures, and ends with an
ok/FAIL summary. Read the whole summary before fixing anything.

## 2. Fix failures

If everything passed, report and stop. Otherwise fix, then go back to step 1.

- **Formatting and auto-fixable lint**: `uv run invoke fix` (`cargo fmt`, `ruff check --fix`,
  `ruff format`). Fix whatever remains in the code itself.
- **clippy**: change the code. `#[allow(...)]` is a last resort and gets a comment saying why.
- **Ruff**: change the code. A suppression is a last resort: a line-level `# noqa: <RULE>` with a
  reason, or a commented `ignore` entry in `pyproject.toml` (CONTRIBUTING.md, "Python").
- **build**: a `maturin develop` failure is a Rust compile error; `pytest` fails with it, so fix
  the build first.
- **pytest**: fix the code rather than weakening the test. If the test itself is wrong, say why
  before changing it.

## 3. Report

List the final result of each check and what was changed (files and a line on each).

# Repository Guidelines

optopus-py is the Python binding (PyO3 + maturin, abi3 wheel for CPython 3.9+) of
[optopus](https://github.com/trash-iine/optopus), a Rust metaheuristics library for combinatorial
optimization.

This file is a digest for AI agents. `CONTRIBUTING.md` is the canonical guide and `README.md` the
single source for setup and usage; on conflict, those and `pyproject.toml` / `Cargo.toml` win.

## Project structure
- `src/*.rs` — the bindings. `lib.rs` registers the module; the `///` comments on `#[pyclass]` /
  `#[pymethods]` items are the Python docstrings shown in the API reference.
- `vendor/optopus/` — the Rust core as a git submodule and path dependency. Do not edit it here.
- `tests/` — pytest suites run against the built extension; `tests/test_tasks.py` covers
  `tasks.py`.
- `docs/source/` — the user documentation (Sphinx + MyST), published to GitHub Pages.
- `docs/adr/` — Architecture Decision Records (`NNNN-<slug>.md`) for contributors, kept out of the
  published docs. Read them before proposing changes to tooling or conventions.
- `tasks.py` — Invoke tasks that bundle several steps: `ci`, `fix`, `docs`, `adr`, `set-version`,
  `check-version`. Do not add single-command wrappers (ADR 0002).
- `.github/workflows/` — `ci.yml` (rust, lint, python matrix), `docs.yml` (Pages deploy),
  `release.yml` (wheels on `v*` tags).
- `.claude/skills/` — `/quality-check`, `/create-pr`, `/update-docs`, `/adr`, `/release`,
  `/port-to-rust`. Keep them in sync with `CONTRIBUTING.md` and this file when rules change;
  `/port-to-rust`'s API map and run loop also follow `src/*.rs` (CONTRIBUTING.md, "Claude Code
  skills").

## Commands
- Setup: `git submodule update --init`, `uv sync --dev`, then `uv run pre-commit install` once.
- Build the extension into `.venv`: `uv run maturin develop`. Tests exercise that build, so rebuild
  after touching `src/` or `vendor/`.
- All CI checks, run to completion with a summary: `uv run invoke ci` (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `ruff check .`, `ruff format --check .`,
  `maturin develop`, `pytest`). Run it before pushing.
- Auto-fix formatting and lint: `uv run invoke fix`.
- Docs: `uv run invoke docs --strict` (rebuilds the extension first; add `--open` / `--clean`).
- A plain `uv run <cmd>` re-syncs the env and can reinstall a cached `optopus` wheel over a fresh
  `maturin develop` build; use `uv run --no-sync <cmd>` right after a build, or go through the
  Invoke tasks, which handle it.
- New ADR: `uv run invoke adr <kebab-slug> --title "<title>"`.
- Releases: follow `.claude/skills/release/SKILL.md`; never bump the version in a feature PR.

## Conventions
- Everything is written in English: code, comments, docstrings, docs, commits, PRs.
- Branches: `<type>/<short-kebab-description>`, type in
  `feat|fix|docs|refactor|test|ci|chore|release`. Never commit to `main`.
- Commits: imperative, sentence case, ≤72 characters, no gitmoji or type prefixes.
- User-visible changes get an entry under `## Unreleased` in `CHANGELOG.md` (Keep a Changelog);
  breaking ones under Changed/Removed with what users must do differently.
- Rust: rustfmt + clippy with `-D warnings`. Docstrings in Google style with Python types
  (`int | None`). Reject inputs the upstream crate would panic on with `ValueError` / `TypeError`.
- Python: Ruff with a moderate rule set and `line-length = 100` (ADR 0003). Suppress only with a
  commented `# noqa: <RULE>`.
- Tests: tiny instances with known optima, `pytest.mark.parametrize` for variants and error paths,
  hermetic (no network, writes only under `tmp_path`).
- Docs: new pages must be added to the toctree in `docs/source/index.md`.
- Decisions about conventions, tooling, dependencies or architecture get an ADR in the same PR.
  Accepted ADRs are superseded by new ones, never rewritten.

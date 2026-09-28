# Contributing

This document sets the development rules for optopus-py: coding conventions and workflow.
**Setup and usage live in [README.md](README.md)** and are not repeated here. [AGENTS.md](AGENTS.md)
is a digest of this document for AI agents; where the two disagree, this document and
`pyproject.toml` / `Cargo.toml` win.

## Workflow

- Never commit directly to `main`. Work on a branch and open a pull request.
- Name branches `<type>/<short-kebab-description>`, where `type` is one of:

  | type | Use |
  | --- | --- |
  | `feat/` | New bindings or features |
  | `fix/` | Bug fixes |
  | `docs/` | Documentation only |
  | `refactor/` | Restructuring without behavior change |
  | `test/` | Tests only |
  | `ci/` | Workflows |
  | `chore/` | Tooling, dependencies, configuration |
  | `release/` | Version bumps (see [Releases](#releases)) |

  Example: `feat/bind-vrp`, `fix/tabu-tenure-overflow`.
- A PR merges when CI is green. Delete the branch after merging.

## Commits

- English, imperative mood, sentence case, no trailing period, at most 72 characters in the subject
  (e.g. `Bind the new optopus heuristics and graph generators`). No gitmoji or type prefixes.
- Say *why* in the body when the subject cannot, and reference related issues or PRs there.
- Split independent changes into separate commits.

## Changelog

- `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Add an entry
  under `## Unreleased` for every change a user of the package can notice: new or changed API,
  behavior, installation, or published docs.
- Breaking changes go under **Changed** or **Removed** and say what the reader has to do
  differently.
- Repository-internal changes (CI, lint configuration, contributor docs) need no entry.

## Rust

- `cargo fmt` formats; `cargo clippy --all-targets -- -D warnings` must pass.
- The Python API's docstrings are the `///` comments on `#[pyclass]` / `#[pymethods]` items. Write
  them for Python users in Google style (`Args:` / `Returns:` / `Raises:`), with Python types
  (`int | None`), because Sphinx autodoc + Napoleon render them into the API reference. Single
  backticks render as inline code (`default_role = "literal"` in `docs/source/conf.py`).
- Raise `ValueError` / `TypeError` from the binding for inputs the upstream crate would reject with
  a panic; Python users should never see a Rust panic.
- `vendor/optopus` is a git submodule and a path dependency. Update it in its own commit
  (`Update the vendored optopus to <sha>`) together with the binding changes the new upstream API
  requires.

## Python

- Python code is the test suite, `tasks.py` and `docs/source/conf.py`. Ruff lints and formats it;
  `pyproject.toml` is the single source of truth for the rules (a moderate, bug-oriented set,
  `line-length = 100`). The reasoning is in ADR 0003.
- Before suppressing a rule, try to fix the code. If a suppression is unavoidable, use a
  line-level `# noqa: <RULE>` with a comment saying why, or a commented `ignore` entry in
  `pyproject.toml`.

## Tests

- Tests live in `tests/` as `test_*.py` and run against the built extension, so rebuild with
  `uv run maturin develop` before `pytest` (`uv run invoke ci` does both).
- Use tiny instances with a known optimum so assertions do not depend on the seed.
- Prefer `pytest.mark.parametrize` for input variations and error paths; match error messages
  with `match=` where the message is part of what is being tested.
- Keep tests hermetic: no network access and no writes outside `tmp_path`.

## Documentation

- Sphinx + MyST (Markdown) + furo. Build with `uv run invoke docs`, which rebuilds the extension
  first because the API reference is generated from it. `--strict` turns warnings into errors,
  `--clean` rebuilds from scratch, `--open` opens the result.
- New pages go in `docs/source/` and must be added to the `{toctree}` in `docs/source/index.md`.
- Pushes to `main` deploy the docs to GitHub Pages; pull requests only build them.

## Architecture decision records (ADRs)

- Decisions about conventions, tooling, dependency choices or architecture — anything someone will
  later ask "why is it like this?" about — are recorded as ADRs in `docs/adr/`. This
  document holds the rules; ADRs hold the reasoning and the rejected options.
- Create one with `uv run invoke adr <slug> --title "<title>"` (slug in kebab-case). Numbering and
  the template are handled for you.
- ADRs are for contributors and stay out of `docs/source/`, which is the user documentation.
- Sections: Context / Options considered / Decision / Consequences. List only options that were
  actually compared.
- Status is one of `Proposed`, `Accepted`, `Deprecated`, `Superseded by NNNN`. Do not rewrite an
  accepted ADR; write a new one and mark the old one `Superseded by NNNN`.
- A PR that makes such a decision adds its ADR in the same PR.

## Checks

- After setup, run `uv run pre-commit install` once. Commits then run file-hygiene checks,
  `ruff check --fix`, `ruff format` and `cargo fmt`. Only fast, auto-fixing hooks run there;
  clippy, the build and pytest are left to CI.
- Before pushing, run `uv run invoke ci`. It runs the same six checks as CI (`cargo fmt --check`,
  `cargo clippy`, `ruff check`, `ruff format --check`, `maturin develop`, `pytest`), keeps going
  past failures, and prints a summary. `uv run invoke fix` applies `cargo fmt` and Ruff's fixes.
- `CI_CHECKS` in `tasks.py` mirrors `.github/workflows/ci.yml`; change both together.
- Invoke tasks are limited to workflows that bundle several steps or need project knowledge
  (`ci`, `fix`, `docs`, `adr`, `set-version`, `check-version`). Do not add thin wrappers around
  single commands (ADR 0002).

## Dependency updates

- Keep the `astral-sh/ruff-pre-commit` `rev` in `.pre-commit-config.yaml` in step with the ruff
  version in `pyproject.toml`, and bump them together.
- A `pyo3` or `rand` bump may need binding changes (`rand` must match the vendored optopus), so
  read the upstream changelog rather than relying on a green CI alone.

## Releases

- A version bump never rides along in a feature PR. One `release/vX.Y.Z` PR moves every
  version reference with `uv run invoke set-version X.Y.Z`, and the tag goes on its merge commit.
  The `/release` skill (`.claude/skills/release/SKILL.md`) has the full procedure and the reasons
  for its ordering.

## Claude Code skills

Project-shared skills live in `.claude/skills/` and are available as `/<name>` in a Claude Code
session in this repository.

| Skill | Purpose |
| --- | --- |
| `/quality-check` | Run the CI checks and fix failures until they pass |
| `/create-pr` | Branch check → checks → commits → push → pull request |
| `/update-docs` | Add or edit doc pages and verify the strict build |
| `/adr` | Record a design decision made in the conversation |
| `/release` | Cut a release: version bump PR, then tag its merge commit |
| `/port-to-rust` | Port Python code that uses optopus-py to a Rust crate and compare the two |
| `/model-problem` | Model a task described in words as an optopus-py script, checked by brute force |

The skills turn this document into procedures. When a rule here changes, update the affected
skills and AGENTS.md in the same PR, and the other way around.

`/port-to-rust` does not copy the Python-to-optopus mapping; it reads it from `src/*.rs` on each
port, so the binding stays the only place it is written. Two things there follow the binding and
move with it in the same PR: `templates/src/harness.rs`, a copy of `run_all` and `derive_seed` in
`src/runner.rs` and of the report arithmetic in `src/result.rs`, and the table in its `SKILL.md`
that names the functions to read (`solve`, `build_<problem>`, `build_generic`, `build_nested`,
`build_python`).

`/model-problem` likewise names where to read the problems, `Formula`, the Python problem
protocol and the heuristics (the docs pages and `src/*.rs`) instead of listing them. Its table of
those places moves with them.

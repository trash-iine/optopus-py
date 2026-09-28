# 0003. Adopt a moderate Ruff rule set

- Date: 2026-09-28
- Status: Accepted

## Context

The Python in this repository is the test suite, `tasks.py` and the Sphinx configuration; the
library itself is Rust. Until now nothing linted or formatted it. python-project-template, which
this repository is based on, uses Ruff with `select = ["ALL"]`, but that template targets pure
Python packages whose public API is Python code.

## Options considered

- **`select = ["ALL"]`, as in the template**: 635 violations on the existing code, mostly
  docstrings, type annotations and `assert` in tests. Satisfying it means annotating and
  documenting every test function, which adds noise without catching bugs — the API those tests
  exercise is typed and documented in Rust.
- **A moderate, bug-oriented set** (`E`, `W`, `F`, `I`, `B`, `UP`, `C4`, `SIM`, `PT`, `RUF`) plus
  `ruff format`: about ten real findings (ambiguous names, unescaped regex metacharacters in
  `match=`, import order).
- **`ruff format` and import sorting only**: consistent layout, but no bug detection.

## Decision

The moderate set, with `ruff format`. `line-length = 100` matches rustfmt's default `max_width`
and the width the existing Python was already written to. `PT011` is ignored because parametrized
tests assert that several different bad inputs each raise `ValueError`, with messages that differ
per case. Markdown files are excluded, since Ruff would otherwise reformat the hand-aligned code
examples in the docs.

## Consequences

- CI's `lint` job, `invoke ci` and the pre-commit hooks run `ruff check` and `ruff format --check`.
- Because the set is not `ALL`, a Ruff upgrade rarely introduces new failures, so upgrading Ruff
  does not need the template's auto-fix workflow.
- No type checker (`ty`) is run: the extension ships no `.pyi` stubs, so there is little for one
  to check.

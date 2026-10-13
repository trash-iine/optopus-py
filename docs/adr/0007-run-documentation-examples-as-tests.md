# 0007. Run the documentation's code examples as tests

- Date: 2026-10-13
- Status: Proposed

## Context

The user documentation is meant to be enough to use the package without reading `src/`, so
every class now has a runnable example with its output in a comment. Until now the rule
"examples run as shown" (the `update-docs` skill) was checked by hand. Nothing caught an example
broken by an API change, and an example that depends on a block of another page, or on a name a
reader cannot see, looked fine in review. Writing the expanded pages turned up exactly that: a
block on `problems.md` that used an undefined `report`, and one on `heuristics.md` that used a
knapsack defined on `formula.md`.

## Options considered

- **Keep checking by hand.** No tooling, but the examples rot silently as the API changes,
  and there are now over seventy of them.
- **`sphinx.ext.doctest`.** Checks printed output as well, but needs every example rewritten
  in `>>>` form or as `testcode` directives, which MyST pages render less readably than a plain
  fenced block, and runs only through a separate Sphinx builder that CI does not invoke.
- **A pytest module that extracts the fenced Python blocks.** No new dependency and no change
  to how examples are written; CI's `pytest tests` already runs it. It checks that the blocks
  run, not that their output matches the comments.

## Decision

`tests/test_docs.py` runs every ```` ```python ```` block of each page under `docs/source/`, in
order and in one namespace per page, from a temporary working directory. There is no way to
exempt a block: an illustration that cannot run is written in another language fence, such as
`text`, or made to run.

## Consequences

- A page's examples must run top to bottom on their own: a block may use what an earlier block
  on the same page defined, never another page's. Files an example needs are written by the
  example itself.
- Examples stay cheap. Iteration limits are preferred over time limits, and time-limited
  comparisons use fractions of a second; the whole module takes a few seconds on the
  development build.
- Printed values in comments are still checked by hand when an example is written or changed.
  A mismatch there is a documentation bug the test does not catch.

# 0010. Map optopus errors to Python exception types

- Date: 2026-10-17
- Status: Accepted

## Context

optopus returns one error type, `OptError`, for a file that does not parse, a solution that does
not fit its instance, and a search that cannot go on, such as a simulated annealing step on an
empty neighborhood. The binding turned every `OptError` it looked at into `ValueError`, and it
did not look at the one `Heuristic::run` returns: `run_all` discarded it with `let _ =`. A run
that stopped this way came back as an ordinary `RunResult` whose numbers read like a finished
search, and the later runs went ahead. An existing test ran `SimulatedAnnealing` on
`BinPacking`'s `"Relocate"` this way for as long as optopus left that neighborhood empty on a
tight packing.

optopus also reports a file it cannot open as `OptError::FileLoad`, whose detail is a string, so
a missing file reached Python as `ValueError` with no way to tell it from a malformed one.

## Options considered

- **One exception for everything, `ValueError`.** No change for callers, but a failed search
  reads as bad input, and a missing file cannot be caught apart from a malformed one.
- **Record a failed run in the report** (a `status` on `RunResult`, as optopus's benchmark
  runner does). Keeps the other runs, but a caller who does not check the field gets the silent
  result this replaces.
- **Map by cause to the standard exceptions**: `ValueError` for input optopus rejects,
  `OSError` for a file that cannot be opened, `RuntimeError` for a run that cannot go on.

## Decision

Map by cause. `src/error.rs` holds the mapping: `input_error` for an `OptError` about the input
(`ValueError`, or `OSError` for `OptError::Io`), `run_error` for one a heuristic returns during a
run (`RuntimeError`), and `check_readable`, which every `load_file` calls first so that a file
that cannot be opened raises the `OSError` subclass Python's own `open` would, with the path in
`filename`. A run that fails raises at once and no later run starts, as an exception from
`on_improvement` already did. An exception a Python callback raised still takes precedence over
the `OptError` optopus returns because of it.

## Consequences

- A missing instance file raises `FileNotFoundError` instead of `ValueError`, a breaking change
  recorded in the changelog.
- A heuristic that runs into an empty neighborhood raises `RuntimeError` instead of returning a
  partial result, so the cases optopus treats as errors become visible; those are fixed in
  optopus (ADR 0005), not hidden here.
- New bindings that call an optopus function returning `Result` convert its error with
  `input_error`, and new loaders call `check_readable` first. CONTRIBUTING.md states the rule.

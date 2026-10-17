# 0012. Let BranchAndBound branch on problems written in Python

- Date: 2026-10-17
- Status: Accepted

## Context

optopus's `BranchAndBound` runs on any `BranchSpace`. An integer problem gets one by implementing
`Branchable`, on top of `IntAssignment`, through four requirements:
- the variables' ranges
- reading a solution's value of one variable
- setting one
- the same problem with narrower ranges

The binding exposed `BranchAndBound` for `MaxCut`, `Qubo`, `Sat`, `VertexCover` and `Formula`
only. A problem written in Python could not be proved optimal, though a relaxation written in
Python already worked on the built-in problems.

ADR 0011 extended the Python protocol with optional methods for the sub-problem crossover, the
search staying with built-in heuristics.

## Options considered

- **Mirror `IntAssignment` one call per variable** (`get(solution, i)`, `assign(solution, i, v)`).
  This is the closest to the trait, but it makes one Python call per variable read. Its `assign`
  also mutates a solution, which the protocol otherwise never does.
- **Whole-solution methods.** `variable_values(solution)` returns all values and is read once
  per solution and cached. `solution_from_values(values)` builds a new solution, and the binding
  derives `get` and `assign` from the two. Together with `variable_bounds()` and
  `restricted(bounds)`, these are four optional methods.
- **Require the problem to be a `Formula`.** That is already possible, but it rules out any
  objective that is not a polynomial.

## Decision

Whole-solution methods.

`PyProblem` implements `IntAssignment` and `Branchable`. A node is the Python problem
`restricted` returns. It is held to the same rules as a sub-problem in ADR 0011: the same
neighborhoods in the same order, and a shared error slot.

**Relaxation.** Only a relaxation written in Python applies. `"Binary"`, `"Interval"`,
`"Eigenvalue"` and the default need structure optopus knows only for its own problems, so they
raise `ValueError`.

**Two fixes in shared code.** These follow from how the binding drives a run on a Python
problem.
- **Recording the outcome.** The `Guarded` wrapper calls `run_once` and `is_done`, never `run`.
  So `BranchAndBound`'s outcome, and PT's and RL's run details, are recorded when `is_done`
  first answers true as well as in `run`.
- **The node's rng.** A node is warm-started and never calls `new_solution`, so it inherits the
  problem's seeded `random.Random`.

**Errors.** optopus expects `solution_from_values` to succeed (`expect`). A Python exception
there is recorded and answered with a placeholder solution, and the run stops at its next step.

## Consequences

- `BranchAndBound(heuristic, relaxation=f)` proves optima on Python problems. The error for a
  Python problem names the missing branching methods instead of saying `BranchAndBound` is
  only for the built-in problems.
- A neighborhood of a branching Python problem must read the problem's own ranges, since the
  heuristic searching a node moves with the node's neighborhoods. The docs say so.
- The protocol's optional methods now number four for branching, two for the sub-problem
  crossover, and the ruin methods.

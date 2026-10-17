# 0011. Let a problem written in Python cut out its own sub-problems

- Date: 2026-10-17
- Status: Accepted

## Context

optopus's `SubProblemBasedCrossover` crosses two parents by searching a smaller problem: the
variables they disagree on, with the rest fixed and folded into the objective. A problem opts in
through `SubProblemExtractable`, which returns the sub-problem as a value of the problem's own
type and lifts its solution back. The built-in binary problems, `Tsp` and `Formula` implement
it, and the binding exposed it as `GeneticAlgorithm(crossover="SubProblem")` for them.

A problem written in Python had one crossover, its `crossover` method, and the binding refused
`crossover="SubProblem"` on it. What the parents agree on is a property of the user's own
representation, so only the user's code can say what the sub-problem is.

Python problems define the problem and its neighborhoods, never the search (memory
`heuristics-rust-only`): the sub-problem has to be searched by a built-in heuristic.

## Options considered

- **Keep the one Python `crossover`.** Users can already write a crossover that solves a
  sub-problem, but only with their own search, not with a built-in heuristic.
- **Describe the sub-problem as data** (which variables are free) and build it in Rust. The Rust
  side knows nothing of a Python solution's structure, so it cannot fold the fixed part into an
  objective.
- **Two problem methods returning a Python problem**: `extract_sub_problem(a, b)` returns
  another object that follows the same protocol, and `lift_solution(a, b, sub_solution)` the full
  child. The sub-problem is wrapped like any Python problem and searched by the `sub_heuristic`.

## Decision

Two problem methods. `PyProblem` implements `SubProblemExtractable` by calling them.

Two details follow from how the binding works:
- **The sub-problem shares the problem's error slot.** An exception in its methods stops the
  run and is raised from `run`, as one from the problem's own methods is. A failed extraction
  yields a stand-in that stops at once.
- **The sub-problem must declare the problem's neighborhoods by the same names in the same
  order.** The `sub_heuristic` is built once against the original problem, and a Python move
  is picked by its position. A mismatch raises `ValueError`. The sub-problem also inherits
  whether moves carry their tabu keys, so a `TabuSearch` works as the `sub_heuristic`.

## Consequences

- `GeneticAlgorithm(crossover="SubProblem")` runs on a Python problem that has both methods. A
  problem without them gets a `ValueError` naming what is missing, and any other `crossover`
  name now gets an "invalid crossover" message instead of "has one crossover".
- The protocol grows by two optional methods, documented in `python_problems.md`. A further
  protocol extension, such as `BranchAndBound` on Python problems, follows the same pattern:
  optional methods, the same error slot, and the search left to built-in heuristics.

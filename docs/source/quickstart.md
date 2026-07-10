# Quickstart

This page walks through solving a small problem with each of the two problem types. It assumes you
have already [installed](installation.md) the package.

## Max Cut

`MaxCut` is a **maximization** problem: partition the vertices of a weighted graph into two sides so
that the total weight of edges crossing the partition is as large as possible.

```python
import optopus

# Build the graph from a weighted edge list (0-indexed vertices).
mc = optopus.MaxCut.from_edges([
    (0, 1, 1.0),
    (1, 2, 2.0),
    (0, 2, 3.0),
])

# Configure a heuristic and a stopping criterion.
ls = optopus.LocalSearch(
    neighbor="Flip",
    stop=optopus.StopCondition(max_iteration=10_000),
)

# Run it 5 times with a fixed seed for reproducibility.
report = ls.run(mc, runs=5, seed=42)

print(report.best_objective)    # 5.0  (cut edges (0,2)=3 and (1,2)=2)
print(report.runs[0].solution)  # [True, True, False] — side of each vertex
```

The optimal cut separates vertex `2` from `{0, 1}`, cutting the two heaviest edges (3.0 + 2.0 = 5.0).

## QUBO

`Qubo` is a **minimization** problem: minimize $x^\top Q x$ over binary $x \in \{0, 1\}^n$. Entries
are given as `(i, j, coefficient)` triples with **integer** coefficients.

```python
import optopus

# Minimize: -x0 - x1 + 2*x0*x1
q = optopus.Qubo.from_entries([
    (0, 0, -1),
    (0, 1, 2),
    (1, 1, -1),
])

sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=10.0,
    cooling_rate=0.99,
    stop=optopus.StopCondition(max_duration_secs=0.2),
)

report = sa.run(q, runs=3, seed=7)

print(report.best_objective)    # -1.0
print(report.runs[0].solution)  # [True, False] — value of each variable
```

## TSP

`TspWithCoordinates` is a **minimization** problem: find the shortest tour visiting every city
exactly once. The solution is a `list[int]` permutation of city indices rather than the
`list[bool]` used by the binary problems above.

```python
import optopus

# Five cities at the corners and center of a unit square.
tsp = optopus.TspWithCoordinates.from_coordinates(
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)],
    name="square",
)

ls = optopus.LocalSearch(
    neighbor="TwoOpt",
    stop=optopus.StopCondition(max_iteration=10_000),
)

report = ls.run(tsp, runs=3, seed=0)

print(report.best_objective)    # ~ 4.83  (square perimeter ≈ 4.0 + 2 detours via the center)
print(report.runs[0].solution)  # e.g. [0, 4, 2, 1, 3] — order of city visits
```

The available neighborhoods for TSP are `"TwoOpt"` (reverse a tour segment) and `"Relocate"`
(remove a city and reinsert it elsewhere).

## Next steps

- [API reference](api_reference.md) — every class, method, parameter, and result field.
- [Examples](examples.md) — multi-run statistics, comparing heuristics, reproducibility.

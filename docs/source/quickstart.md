# Quickstart

This page walks through solving a few small problems, then shows how to generate random instances
and how to reach for the more specialized heuristics. It assumes you have already
[installed](installation.md) the package.

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

## Generating a random graph

`MaxCut` and `VertexCover` are defined over a graph, so instead of writing an edge list by hand you
can draw one from a random graph model. The generators produce unweighted graphs; chain
`with_random_weights` to draw integer weights.

```python
import optopus

# Erdős-Rényi G(n, p), then weights drawn uniformly from 1..10.
g = optopus.Graph.erdos_renyi(200, 0.05, seed=42).with_random_weights((1, 10), seed=42)

print(g)                 # Graph(num_vertices=200, num_edges=...)

mc = optopus.MaxCut.from_graph(g)
report = optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50_000)).run(mc, seed=42)
print(report.best_objective)
```

`barabasi_albert(n, m, ...)` (scale-free) and `watts_strogatz(n, k, beta, ...)` (small world) are
available too. Both have a deterministic edge count — `m*(m-1)/2 + m*(n-m)` and `n*k/2` respectively.
Passing `seed` makes a generator reproducible; omitting it draws a fresh seed from the clock.

## Going beyond a single neighborhood

`VariableNeighborhoodSearch` alternates an intensifying `search` heuristic with a list of
increasingly disruptive `shakes`. Each step is an ordinary heuristic instance, so steps can differ in
neighborhood and in budget.

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=42))

vns = optopus.VariableNeighborhoodSearch(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=200)),
    shakes=[
        optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=5)),
        optopus.RandomWalk("Swap", stop=optopus.StopCondition(max_iteration=15)),
    ],
    stop=optopus.StopCondition(max_iteration=20_000),
)

print(vns.run(mc, runs=3, seed=42).best_objective)
```

## Problem-specific heuristics

Some algorithms exploit the structure of one problem type and so take no `neighbor` argument. Running
one on a different problem raises `ValueError`.

```python
import optopus

# WalkSAT/SKC for SAT: picks an unsatisfied clause, then flips one of its variables.
sat = optopus.Sat.from_clauses(3, [[1, 2], [-1, 3], [2, -3]])
walksat = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=10_000), noise=0.3)
print(walksat.run(sat, seed=42).best_objective)   # 3.0 — every clause satisfied

# Breakout local search for Max Cut: tabu descent with adaptive perturbations.
mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=42))
bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(3, 50), t=1_000, l0=20, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=20_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)

# Lin-Kernighan-Helsgaun for the Euclidean TSP.
tsp = optopus.TspWithCoordinates.from_coordinates(
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], name="unit-square",
)
lkh = optopus.LinKernighanHelsgaun(stop=optopus.StopCondition(max_iteration=1_000))
print(lkh.run(tsp, seed=42).best_objective)       # 4.0 — the square's perimeter
```

`PopulationAnnealing` and `RlBreakoutLocalSearch` are also available for Max Cut; see the
[API reference](api_reference.md) for their parameters.

## Next steps

- [API reference](api_reference.md) — every class, method, parameter, and result field.
- [Examples](examples.md) — multi-run statistics, comparing heuristics, reproducibility.

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

`Tsp` is a **minimization** problem: find the shortest tour visiting every city
exactly once. The solution is a `list[int]` permutation of city indices rather than the
`list[bool]` used by the binary problems above.

```python
import optopus

# Five cities at the corners and center of a unit square.
tsp = optopus.Tsp.from_coordinates(
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

Distances need not come from coordinates: `Tsp.from_distance_matrix(matrix)` takes an explicit
matrix, and `Tsp.load_file(path)` reads a TSPLIB instance.

```{note}
This problem was called `TspWithCoordinates` before it gained those constructors. optopus
renamed it to `Tsp`, and the binding follows: the old name no longer exists.
```

## Vehicle routing

`Vrp` is the capacitated vehicle routing problem: serve every customer exactly once from a depot
without exceeding the vehicle capacity, minimizing total distance. Index `0` is the depot, and a
solution is a `list[list[int]]` — one route of customer indices per vehicle, with the depot
implicit at both ends.

```python
import optopus

vrp = optopus.Vrp.from_coordinates(
    coordinates=[(0.0, 0.0), (10.0, 0.0), (11.0, 0.0), (-10.0, 0.0), (-11.0, 0.0)],
    demands=[0, 5, 5, 5, 5],   # entry 0 is the depot
    capacity=15,
    num_vehicles=2,            # 0 lets optopus pick a fleet size
)

hgs = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=20_000))
report = hgs.run(vrp, runs=3, seed=42)

print(report.best_objective)               # 44.0 — one vehicle per cluster
print(report.runs[0].solution)             # e.g. [[1, 2], [3, 4]]
print(vrp.evaluate_routes([[1, 2], [3, 4]]))
# {'objective': 44.0, 'distance': 44.0, 'overload': 0, 'route_loads': [10, 10]}
```

The objective a heuristic minimizes is `distance + penalty_weight * overload`, so an
over-capacity solution scores worse than any feasible one without being rejected outright. Use
`evaluate_routes` to read the raw distance and the overload separately.

Besides the generic heuristics (`"Relocate"`, `"Swap"`, `"TwoOpt"`), `Vrp` has two specialized
ones: `HybridGeneticSearch` and `AdaptiveLargeNeighborhoodSearch`. `Vrp.load_file(path)` reads a
CVRPLIB instance, and `Vrp.from_distance_matrix(...)` takes an explicit matrix.

## Graph coloring

`GraphColoring` assigns each vertex a color so that no edge joins two vertices of the same
color, using as few colors as possible. Conflicts are penalized rather than forbidden, at a
weight high enough (`num_vertices + 1`) that removing any conflict beats saving a color.

```python
import optopus

# A 5-cycle: the smallest graph that needs three colors.
gc = optopus.GraphColoring.from_edges(
    [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0)]
)

print(gc.num_colors())   # 3 — the palette defaults to max_degree + 1

ts = optopus.TabuSearch(
    neighbor="Recolor", tabu_tenure=(3, 10),
    stop=optopus.StopCondition(max_iteration=10_000),
)
report = ts.run(gc, runs=4, seed=1)

print(report.best_objective)               # 3.0 — a proper 3-coloring
print(gc.evaluate_colors([0, 1, 0, 1, 2]))
# {'objective': 3, 'colors_used': 3, 'conflicts': 0}
```

A `best_objective` below `penalty_weight()` means the coloring is proper, and the value is the
number of colors it used. The neighborhoods are `"Recolor"` (give one vertex another color) and
`"Swap"` (exchange two vertices' colors).

## Generating a random graph

`MaxCut`, `VertexCover` and `GraphColoring` are defined over a graph, so instead of writing an
edge list by hand you can draw one from a random graph model. The generators produce unweighted
graphs; chain `with_random_weights` to draw integer weights. `Graph.grid_torus_2d(l)` and
`Graph.grid_torus_3d(l)` give periodic lattices, the topology of the G-set's toroidal group.

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
    tabu_tenure=(6, 100), t=1_000, l0=20, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=20_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)

# Lin-Kernighan-Helsgaun for the Euclidean TSP.
tsp = optopus.Tsp.from_coordinates(
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], name="unit-square",
)
lkh = optopus.LinKernighanHelsgaun(stop=optopus.StopCondition(max_iteration=1_000))
print(lkh.run(tsp, seed=42).best_objective)       # 4.0 — the square's perimeter
```

`AdaptiveLargeNeighborhoodSearch` also runs on `Tsp` and `Vrp`, and `HybridGeneticSearch` on
`Vrp`; see the [API reference](api_reference.md) for their parameters.

```{note}
`BreakoutLocalSearch`'s `tabu_tenure` means the same prohibition length it does under
`TabuSearch`. Benlic and Hao's γ is counted twice inside the algorithm, so to reproduce their
`rand[3, |V|/10]` on the G-set you pass `(6, len(vertices) // 5)`.
```

## Next steps

- [API reference](api_reference.md) — every class, method, parameter, and result field.
- [Examples](examples.md) — multi-run statistics, comparing heuristics, reproducibility.

# Built-in problems

This page compares the problems optopus ships: what each one optimizes, how its solutions
look, which constraints it takes care of itself, and what a run reports for it. The
[API reference](api_reference.md) has every constructor and argument, and the
[Quickstart](quickstart.md) a worked example of most of them.

## Choosing a representation

A task can reach the search in three ways. Take the first that fits the task exactly.

1. **A built-in problem**, when the task is that problem and nothing more. It is the fastest:
   its moves are priced from what they touch, and some problems have a heuristic built for them.
   A side constraint it does not model rules it out. `Qubo` has no constraints at all, for
   example, so a budget on a QUBO means `Formula`.
2. **[`Formula`](formula.md)**, when every decision is a bounded integer and the objective and
   constraints are polynomials in them. Nothing needs writing besides the polynomials, and it
   is nearly as fast as a built-in problem.
3. **[A problem written in Python](python_problems.md)** for everything else: structured
   solutions, non-polynomial objectives, constraints kept by construction. It is the most
   flexible and the slowest, since every step calls into Python.

## At a glance

| Problem | Optimizes | Solution | Neighborhoods | Problem-specific heuristics |
|---|---|---|---|---|
| `MaxCut` | max cut weight | `list[bool]`, the side of each vertex | `"Flip"`, `"Swap"` | `BreakoutLocalSearch` |
| `Qubo` | min x^T Q x | `list[bool]`, each variable | `"Flip"`, `"Swap"` | |
| `Sat` | max satisfied clauses | `list[bool]`, each variable | `"Flip"`, `"Swap"` | `WalkSat` |
| `VertexCover` | min cover size | `list[bool]`, whether each vertex is in the cover | `"Flip"`, `"Swap"` | |
| `Tsp` | min tour length | `list[int]`, a permutation of the cities | `"TwoOpt"`, `"Relocate"` | `LinKernighanHelsgaun`, `AdaptiveLargeNeighborhoodSearch`, `GreedyRandomizedConstruction` |
| `JobShopScheduling` | min makespan | `list[int]`, an operation sequence | `"Swap"`, `"Relocate"` | |
| `FlowShop` | min makespan | `list[int]`, the jobs in the order they run | `"Relocate"`, `"Swap"` | `IteratedGreedy`, `AdaptiveLargeNeighborhoodSearch`, `GreedyRandomizedConstruction` |
| `Vrp` | min total distance, or time and cost for a mixed fleet | `list[list[int]]`, one route of customers per vehicle | `"Relocate"`, `"Swap"`, `"TwoOpt"` | `HybridGeneticSearch`, `AdaptiveLargeNeighborhoodSearch`, `GreedyRandomizedConstruction` |
| `Qap` | min assignment cost | `list[int]`, the location of each facility | `"Swap"` | |
| `BinPacking` | min bins used | `list[int]`, the bin of each item | `"Relocate"`, `"Swap"` | `AdaptiveLargeNeighborhoodSearch`, `GreedyRandomizedConstruction` |
| `GraphColoring` | min colors used | `list[int]`, the color of each vertex | `"Recolor"`, `"Swap"` | |
| `Formula` | a polynomial, either direction | `list[int]`, each variable | `"Change"` (alias `"Flip"`), `"Swap"`, `"Reverse"` | |

Every generic and composed heuristic runs on every problem. The crossovers `GeneticAlgorithm`
can use on each are listed under [GeneticAlgorithm](heuristics_composed.md#geneticalgorithm).

`BranchAndBound` proves an optimum on `MaxCut`, `Qubo`, `Sat`, `VertexCover` and `Formula`, see
[Exact search](heuristics_exact.md).

## Neighborhoods

Generic heuristics take the neighborhood as a string through their `neighbor` argument, and an
unknown name raises `ValueError` when `run` is called.

| Problem | `neighbor` | Move |
|---|---|---|
| `MaxCut`, `Qubo`, `Sat`, `VertexCover` | `"Flip"` | flips one variable |
| | `"Swap"` | exchanges the values of two variables, keeping how many are `True` |
| `Tsp` | `"TwoOpt"` | reverses a segment of the tour |
| | `"Relocate"` | moves one city elsewhere in the tour |
| `JobShopScheduling` | `"Swap"` | exchanges two adjacent operations in the sequence |
| | `"Relocate"` | moves one operation elsewhere in the sequence |
| `FlowShop` | `"Relocate"` | moves one job elsewhere in the sequence, the insertion move |
| | `"Swap"` | exchanges the jobs at two positions |
| `Qap` | `"Swap"` | exchanges the locations of two facilities |
| `BinPacking` | `"Relocate"` | moves one item into another bin with room for it, emptying its bin if it was the last, or into a new bin of its own if it shares one |
| | `"Swap"` | exchanges two items of different sizes in different bins, when both still fit |
| `Vrp` | `"Relocate"` | moves one customer to a position in another route |
| | `"Swap"` | exchanges two customers of different routes |
| | `"TwoOpt"` | reverses a segment within one route |
| `GraphColoring` | `"Recolor"` | gives one vertex another color |
| | `"Swap"` | exchanges the colors of two differently colored vertices |
| `Formula` | `"Change"` | sets one variable to another value in its range |
| | `"Swap"` | exchanges two variables' values |
| | `"Reverse"` | reverses the values of a range of variables |

`Vrp`'s `"Relocate"` and `"Swap"` only move customers between routes, and `"TwoOpt"` only
reorders a route. Combine them through `VariableNeighborhoodSearch` or `Sequential` to do both,
or use `HybridGeneticSearch`, which does both itself.

## What a run reports

`RunResult.best_objective`, `initial_objective` and the report's aggregates are the value the
search ranks solutions by. For some problems that is not the plain objective:

| Problem | Reported value | Better is |
|---|---|---|
| `MaxCut` | cut weight | higher |
| `Qubo` | x^T Q x | lower |
| `Sat` | number of satisfied clauses | higher |
| `VertexCover` | `cover_size + (num_vertices + 1) · uncovered_edges` | lower |
| `Tsp` | tour length | lower |
| `JobShopScheduling`, `FlowShop` | makespan | lower |
| `Qap` | assignment cost | lower |
| `BinPacking` | `num_bins − mean((load / capacity)²)`, see [BinPacking](#binpacking) | lower |
| `Vrp` | `distance + penalty_weight · overload`; for a mixed fleet, see [Mixed fleet](#vrp-mixed-fleet) | lower |
| `GraphColoring` | `colors_used + (num_vertices + 1) · conflicts` | lower |
| `Formula` | `objective − penalty`, or `−(objective + penalty)` when minimizing | higher |
| a Python problem | `objective(solution)` | as its `minimize` says |

`RunReport.best_objective` and `worst_objective` follow that direction: the lowest value is
the best for a minimizing problem. A `Formula` is always reported higher-is-better, so its
`best_objective` is the maximum over the runs even when it minimizes; see
[What the search reports](formula.md#what-the-search-reports).

Objectives are floating-point numbers, and a search updates them move by move, so the value a
run reports can differ from a fresh evaluation in the last digits. Compare objectives with a
tolerance, `math.isclose`, unless the data is integral, and recompute the objective of the
solution you keep with the problem's evaluator. On `Vrp` those tiny differences can also count
as improvements, which makes `n_best_updates` and `time_to_best_secs` overstate how long the
search kept improving and delays `max_failed_update`.

The report holds no best solution of its own; [Picking the best solution](running.md#picking-the-best-solution)
shows how to read it.

## Constraints a problem handles itself

Some problems have a hard constraint that a solution may still break during the search. They
penalize it at a weight large enough that the optimum is always feasible, when a feasible
solution exists. Nothing needs adding for these; check the result with the problem's evaluator.

| Problem | Constraint | Penalty weight | Check with |
|---|---|---|---|
| `VertexCover` | every edge covered | `num_vertices + 1` per uncovered edge | the solution: an edge with neither end `True` is uncovered |
| `Vrp` | vehicle capacity; for a mixed fleet also route-time limits and minimum counts | `(num_customers + num_vehicles) · longest_edge + 1` per unit of overload, more with speeds, service times and costs, `penalty_weight()` | `evaluate_routes(routes)["overload"] == 0`, and `time_excess` and `min_count_shortfall` for a mixed fleet |
| `GraphColoring` | no edge within one color | `num_vertices + 1` per conflict, `penalty_weight()` | `evaluate_colors(colors)["conflicts"] == 0` |
| `Formula` | the constraints you give it | the weight you give each | `eval_penalty(values) == 0` |

The other built-in problems cannot express an infeasible solution: every permutation is a tour,
a job order or an assignment of facilities, every operation sequence decodes to a valid
schedule, every assignment is a cut or a MaxSAT assignment, and the `BinPacking` moves only ever
put an item where it fits.

## Per problem

Each section lists how to build the problem and what else it offers, then solves a tiny
instance whose optimum is known. Every constructor is a static method, and the ones that take a
`name` default it to `""` (`"vrp"` for `Vrp`); `load_file` names the instance after the file.

### MaxCut

Partition the vertices into two sides, maximizing the weight of edges between them.

| Constructor | Builds |
|---|---|
| `MaxCut.from_edges(edges)` | the instance over `(u, v, weight)` edges, 0-indexed |
| `MaxCut.from_graph(graph)` | the instance over a [`Graph`](graphs_and_instances.md#graph) |
| `MaxCut.load_file(path)` | a G-set style edge-list file, `N M` then `i j w` per edge, 1-indexed |

| Method | Returns |
|---|---|
| `cut_size(assignment)` | the weight of the edges an assignment cuts, what a run maximizes |
| `graph()` | the underlying [`Graph`](graphs_and_instances.md#graph), as a copy |

```python
import optopus

# A 4-cycle with heavy edges 0–1 and 2–3 and a chord 0–2.
mc = optopus.MaxCut.from_edges([(0, 1, 3.0), (1, 2, 1.0), (2, 3, 3.0), (3, 0, 1.0), (0, 2, 2.0)])

ts = optopus.TabuSearch("Flip", (1, 2), stop=optopus.StopCondition(max_iteration=100))
report = ts.run(mc, runs=3, seed=42)
print(report.best_objective)    # 8.0 — every edge but the chord is cut
print(report.runs[0].solution)  # [False, True, False, True] — the side of each vertex
```

Weights are stored as 32-bit floats, so a sum of many non-integer weights compares only within
rounding; integer weights compare exactly. `MaxCutKernel` shrinks a sparse instance exactly
before the search, and `PlantedMaxCut` builds instances whose optimum is known, see
[Graphs and test instances](graphs_and_instances.md).

### Qubo

Minimize x^T Q x over binary x.

| Constructor | Builds |
|---|---|
| `Qubo.from_entries(entries)` | the instance from `(i, j, coefficient)` entries with integer coefficients |
| `Qubo.load_file(path)` | a sparse matrix file, `N M` then `i j v` per entry, 1-indexed; a pair given twice keeps the last value |

`energy(assignment)` returns the energy of an assignment, what a run minimizes.

A diagonal entry `(i, i, c)` is the linear term c·x_i, and an off-diagonal one the product
term c·x_i·x_j.

```python
# Minimize −3·x0 − 2·x1 − 2·x2 + 4·x0·x1 + x1·x2 − x0·x2.
q = optopus.Qubo.from_entries(
    [(0, 0, -3), (1, 1, -2), (2, 2, -2), (0, 1, 4), (1, 2, 1), (0, 2, -1)]
)

ls = optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100))
print([r.best_objective for r in ls.run(q, runs=3, seed=42).runs])  # [-3.0, -3.0, -6.0]

ts = optopus.TabuSearch("Flip", (1, 2), stop=optopus.StopCondition(max_iteration=100))
best = ts.run(q, runs=3, seed=42).runs[0]
print(best.best_objective, best.solution)  # -6.0 [True, False, True] — x0 = x2 = 1
```

Two of the descents stop at a local optimum of −3, `x1` alone; the tabu search walks out of it.

`(i, j)` and `(j, i)` name the same coefficient, and an entry given twice replaces the earlier
one rather than adding to it. Sum your terms into one entry per pair before passing them:

```python
terms = [(0, 1, 2), (1, 0, 2), (0, 0, -1)]  # 2·x0·x1 written from both sides
summed = {}
for i, j, c in terms:
    key = (min(i, j), max(i, j))
    summed[key] = summed.get(key, 0) + c
q = optopus.Qubo.from_entries([(i, j, c) for (i, j), c in summed.items()])
```

A QUBO with a side constraint, such as a budget, is a [`Formula`](formula.md).

### Sat

Maximize the number of satisfied clauses (MaxSAT; a satisfiable instance is solved when every
clause holds).

| Constructor | Builds |
|---|---|
| `Sat.from_clauses(n_vars, clauses)` | the instance over `n_vars` variables; each clause is a list of DIMACS literals |
| `Sat.load_file(path)` | a DIMACS CNF file |

| Method | Returns |
|---|---|
| `num_satisfied(assignment)` | the clauses an assignment satisfies, what a run maximizes |
| `num_vars()`, `num_clauses()` | the instance's size |

A literal `+i` is variable `i` and `-i` its negation, **1-indexed**, while the solution list is
0-indexed, so variable `i` is `solution[i - 1]`.

```python
# x1, ¬x1 and (x1 ∨ x2): at most two of the three clauses can hold together.
sat = optopus.Sat.from_clauses(2, [[1], [-1], [1, 2]])

best = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=1_000)).run(sat, seed=42).runs[0]
print(best.best_objective)  # 2.0
x1, x2 = best.solution  # variable i is solution[i - 1]
```

`WalkSat` stops as soon as every clause holds; on an unsatisfiable instance like this one it
runs to its `stop`. A literal 0 or a variable above `n_vars` raises `ValueError`.

### VertexCover

Choose the fewest vertices such that every edge has a chosen end.

| Constructor | Builds |
|---|---|
| `VertexCover.from_edges(edges)` | the instance over `(u, v, weight)` edges; the weights are ignored, pass `1.0` |
| `VertexCover.from_graph(graph)` | the instance over a [`Graph`](graphs_and_instances.md#graph) |
| `VertexCover.load_file(path)` | an edge-list file in the format of `MaxCut.load_file`; the weights are ignored |

| Method | Returns |
|---|---|
| `evaluate_cover(assignment)` | a dict with `objective`, `cover_size` and `uncovered_edges`; the objective is `cover_size + penalty_weight() * uncovered_edges` |
| `penalty_weight()` | the penalty per uncovered edge |
| `graph()` | the underlying graph, as a copy |

```python
edges = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0), (0, 2, 1.0)]
vc = optopus.VertexCover.from_edges(edges)

ts = optopus.TabuSearch("Flip", (1, 3), stop=optopus.StopCondition(max_iteration=200))
best = ts.run(vc, runs=3, seed=42).runs[0]
cover = best.solution
uncovered = [(u, v) for u, v, _ in edges if not (cover[u] or cover[v])]
print(best.best_objective, cover, uncovered)  # 3.0 [False, True, True, False, True] []
```

The search may cross covers that miss an edge, penalized at `num_vertices + 1` per uncovered
edge, so a reported value of at most `num_vertices` is a valid cover of that size. Check the
edges as above before using the cover.

### Tsp

Find the shortest closed tour through every city.

| Constructor | Builds |
|---|---|
| `Tsp.from_coordinates(coordinates, name="", edge_weight_type="Continuous", nearest_neighbors=None)` | distances between `(x, y)` points: plain Euclidean, or TSPLIB's `"EUC_2D"`, `"CEIL_2D"`, `"ATT"` or `"GEO"` |
| `Tsp.from_distance_matrix(matrix, name="")` | the given distances; symmetric, finite and non-negative |
| `Tsp.load_file(path)` | a TSPLIB instance |

`nearest_neighbors=k` keeps only the distances to each city's `k` nearest cities, `n × k`
numbers instead of `n²`, and computes any other pair from the coordinates when asked: the
distances are the same, and a large instance fits in memory. `load_file` does this on its own
above 2000 cities.

| Method | Returns |
|---|---|
| `tour_length(tour)` | the length of a closed tour, what a run minimizes |
| `distance(i, j)` | the distance from city `i` to city `j` |
| `num_cities()` | the number of cities |
| `coordinates()`, `edge_weight_type()` | the points and the distance formula, `None` for an instance built from a matrix |

```python
dist = [
    [0, 2, 9, 10],
    [2, 0, 6, 4],
    [9, 6, 0, 3],
    [10, 4, 3, 0],
]
tsp = optopus.Tsp.from_distance_matrix(dist)

best = optopus.LocalSearch("TwoOpt", stop=optopus.StopCondition(max_iteration=100)).run(
    tsp, seed=42
).runs[0]
print(best.best_objective, best.solution)  # 18.0 [2, 0, 1, 3] — 9 + 2 + 4 + 3
```

The solution lists the cities in visiting order; the return to the first city is implied.
`load_file` reads TSPLIB:

```python
from pathlib import Path

Path("square.tsp").write_text(
    "NAME: square\nTYPE: TSP\nDIMENSION: 4\nEDGE_WEIGHT_TYPE: EUC_2D\n"
    "NODE_COORD_SECTION\n1 0 0\n2 30 0\n3 30 40\n4 0 40\nEOF\n"
)
square = optopus.Tsp.load_file("square.tsp")
print(square, square.num_cities())  # Tsp(name="square", n_cities=4) 4
```

A distance matrix that is not symmetric, finite and non-negative raises `ValueError`. The moves
price a reversed segment from the edges at its ends, which is only right when a segment is as
long in both directions.

### JobShopScheduling

Each job is a list of `(machine, duration)` operations that must run in order, and a machine
runs one operation at a time. Minimize the makespan, the time the last operation finishes.

| Constructor | Builds |
|---|---|
| `JobShopScheduling.from_jobs(jobs, name="")` | the instance from one list of `(machine, duration)` per job, machines 0-indexed |
| `JobShopScheduling.load_file(path)` | a Taillard / OR-Library file: `n_jobs n_machines`, then one line of `machine duration` pairs per job, machines 0-indexed |

| Method | Returns |
|---|---|
| `makespan(operations)` | the makespan of an operation sequence, what a run minimizes |
| `completion_times(operations)` | when the operation at each position of the sequence finishes |
| `num_jobs()`, `num_machines()` | the instance's size |

A solution is a sequence in which job `j` appears once per operation: its `k`-th occurrence
schedules the `k`-th operation of job `j`, as early as the machines allow.

```python
jobs = [
    [(0, 3), (1, 2), (2, 2)],  # job 0: machine 0 for 3, then machine 1 for 2, ...
    [(0, 2), (2, 1), (1, 4)],
    [(1, 4), (2, 3), (0, 1)],
]
shop = optopus.JobShopScheduling.from_jobs(jobs)

ts = optopus.TabuSearch("Swap", (2, 4), stop=optopus.StopCondition(max_iteration=300))
best = ts.run(shop, runs=3, seed=42).runs[0]
print(best.best_objective)  # 11.0 — the optimum over all 1,680 sequences
print(best.solution)        # [0, 1, 1, 2, 0, 2, 0, 2, 1]
```

### FlowShop

The permutation flow shop: every job passes through the machines in the same order, and every
machine takes the jobs in one common order, the solution. Minimize the makespan.

| Constructor | Builds |
|---|---|
| `FlowShop.from_processing_times(times, name="")` | the instance from `times[machine][job]`, one row per machine |
| `FlowShop.load_file(path)` | Taillard's layout: `n_jobs n_machines`, then one row of times per machine |

| Method | Returns |
|---|---|
| `makespan(sequence)` | the makespan of a job order |
| `neh()` | the order the NEH construction builds, the usual baseline |
| `num_jobs()`, `num_machines()` | the instance's size |
| `processing_time(machine, job)` | how long `job` takes on `machine` |

```python
from pathlib import Path

Path("demo.fs").write_text("5 3\n5 9 8 10 1\n9 3 10 1 8\n9 4 5 8 6\n")
shop = optopus.FlowShop.load_file("demo.fs")
print(shop.num_jobs(), shop.num_machines())  # 5 3
print(shop.neh(), shop.makespan(shop.neh()))  # [4, 0, 2, 1, 3] 45

ig = optopus.IteratedGreedy(stop=optopus.StopCondition(max_iteration=200), removal_count=2)
print(ig.run(shop, runs=3, seed=42).best_objective)  # 44.0 — one better than NEH, optimal
```

`IteratedGreedy` is the heuristic to start with: it takes a few jobs out, puts each back where
the makespan grows least and descends by insertion, and on Taillard's instances beats
`AdaptiveLargeNeighborhoodSearch`. Of the neighborhoods, `"Relocate"` is the one the flow shop
literature relies on; it has `n (n − 1)` moves, all priced in `O(n² m)`.

### Vrp

Serve every customer once from a depot with vehicles of one capacity, minimizing the total
distance.

| Constructor | Builds |
|---|---|
| `Vrp.from_coordinates(coordinates, demands, capacity, num_vehicles=0, name="vrp", rounded=False, nearest_neighbors=None)` | Euclidean distances; `rounded` rounds them as CVRPLIB does, and `nearest_neighbors` keeps only each node's nearest as `Tsp`'s does |
| `Vrp.from_distance_matrix(matrix, demands, capacity, num_vehicles=0, name="vrp")` | the given distances; symmetric, finite and non-negative |
| `Vrp.with_fleet(coordinates, demands, vehicle_types, service_times=None, objective_mode="TotalTime", cost_weight=1.0, name="vrp", rounded=False)` | a mixed fleet, see [Mixed fleet](#vrp-mixed-fleet) |
| `Vrp.load_file(path)` | a CVRPLIB file, or a mixed fleet from a `.toml` file |

| Method | Returns |
|---|---|
| `evaluate_routes(routes)` | a dict with `objective`, `distance`, `overload` and `route_loads`, and for a mixed fleet every term of its objective |
| `num_customers()`, `num_vehicles()` | the instance's size; `num_vehicles()` is the number of routes in every solution |
| `split_giant_tour(giant)` | routes from an order of all customers, cut where the objective is lowest (Prins' Split) |
| `capacity()` | the vehicle capacity; `ValueError` on a fleet of several types |
| `penalty_weight()` | the penalty per unit of overload |
| `vehicle_types()`, `slot_types()`, `service_times()`, `objective_mode()`, `cost_weight()` | the fleet's description, see [Mixed fleet](#vrp-mixed-fleet) |
| `with_objective_mode(mode)` | a copy charging `"TotalTime"` or `"Makespan"` |

The [Quickstart](quickstart.md#vehicle-routing) solves a four-customer instance.

Index `0` is the depot and customers are `1` to `n`. Build it from coordinates and
demands, a distance matrix, or a CVRPLIB file; a fleet of several vehicle types is described
under [Mixed fleet](#vrp-mixed-fleet). A solution has one route per vehicle, with the
depot implied at both ends, and a route may be empty. `evaluate_routes` reports the raw
distance, the overload and each route's load.

`num_vehicles` is the size of the fleet, and every heuristic returns exactly that many routes;
the unused vehicles are empty routes. Setting it above what the demand needs is therefore safe,
and a spare vehicle gives the search room to move customers. `num_vehicles=0` lets optopus
choose: the vehicles first-fit decreasing packs the demands into, plus 10% rounded down and at
least one spare; read the result back with `num_vehicles()`. A fleet too small for the demand, or a customer whose demand exceeds the
capacity, is not an error: the search returns its best solution with `overload > 0`, so check
the overload before using the routes.

`evaluate_routes` takes one route per vehicle, `num_vehicles()` of them with empty ones
included, and raises `ValueError` otherwise, so a plan that needs more vehicles than the fleet
has is rejected rather than scored.

`HybridGeneticSearch` and `AdaptiveLargeNeighborhoodSearch` handle both kinds of move
themselves. A generic heuristic needs them combined, since `"Relocate"` and `"Swap"` only move
customers between routes and `"TwoOpt"` only reorders one; a `SimulatedAnnealing` over a single
one of them stalls. A variable neighborhood search that polishes with all three:

```python
import random
import optopus

rng = random.Random(7)
coords = [(0, 0)] + [(rng.randint(-50, 50), rng.randint(-50, 50)) for _ in range(30)]
demands = [0] + [rng.randint(1, 9) for _ in range(30)]
vrp = optopus.Vrp.from_coordinates(coords, demands, capacity=40, num_vehicles=6)

# One round runs every move type to a local optimum. Each LocalSearch ends on one
# non-improving iteration, so 3 failed iterations in a row is a round that found nothing.
polish = optopus.Sequential(
    [optopus.LocalSearch(nb, stop=optopus.StopCondition(max_iteration=1_000))
     for nb in ("Relocate", "Swap", "TwoOpt")],
    stop=optopus.StopCondition(max_failed_update=3),
)
vns = optopus.VariableNeighborhoodSearch(
    search=polish,
    shakes=[optopus.RandomWalk("Relocate", stop=optopus.StopCondition(max_iteration=k))
            for k in (2, 4, 8)],
    stop=optopus.StopCondition(max_duration_secs=0.25),
)
report = vns.run(vrp, runs=3, seed=42)
print(report.best_objective)  # ≈ 606.12, the optimum HybridGeneticSearch also reaches
```

A larger `max_failed_update` on `polish` repeats rounds that have already converged and spends
the budget there.

A distance matrix must be symmetric, finite and non-negative, as for `Tsp`; `matrix[i][j]` is
the distance between nodes `i` and `j`. Demands must not be negative, and the depot's demand is
ignored.

(vrp-mixed-fleet)=
#### Mixed fleet

`Vrp.with_fleet` builds an instance served by several vehicle types. A `VehicleType` has a
capacity, a `max_count` of vehicles, a speed, a `fixed_cost` charged once per used vehicle, a
`variable_cost_per_distance`, a `min_count` of vehicles that must be used and a
`max_route_time`. Customers can take a service time. The fleet has `num_vehicles()` routes, the
`max_count` of every type summed, laid out type by type in the order the types were given:
route `i` of every solution is driven by type `slot_types()[i]`. A customer moved between routes
of two types changes vehicle type, so the same neighborhoods, `HybridGeneticSearch` and
`AdaptiveLargeNeighborhoodSearch` all choose the fleet as they route.

The objective is

```text
time + cost_weight · cost + penalty_weight · (overload + time_excess + min_count_shortfall)
```

A route takes `distance / speed` plus the service times on it. `time` sums those route times
with `objective_mode="TotalTime"`, the default, or takes the longest with `"Makespan"`. `cost`
adds the fixed cost of every used vehicle and the per-distance cost of every route, and
`cost_weight` (default 1.0) sets its exchange rate against time. A capacity, a route-time limit
and a minimum count are penalized alike, at a weight that bounds everything else the objective
can charge. One type of speed 1 with no costs or service times is exactly the CVRP above.

```python
import random
import optopus

rng = random.Random(3)
coords = [(0, 0)] + [(rng.randint(-30, 30), rng.randint(-30, 30)) for _ in range(20)]
demands = [0] + [rng.randint(1, 5) for _ in range(20)]
service = [0] + [2.0] * 20

truck = optopus.VehicleType("truck", capacity=30, max_count=2, fixed_cost=50.0)
van = optopus.VehicleType(
    "van", capacity=10, max_count=4, speed=1.5, fixed_cost=10.0, max_route_time=90.0
)
vrp = optopus.Vrp.with_fleet(coords, demands, [truck, van], service_times=service)

report = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=500)).run(
    vrp, runs=3, seed=42
)
best = min(report.runs, key=lambda r: r.best_objective)
result = vrp.evaluate_routes(best.solution)
print(round(result["objective"], 2), result["used_count"])  # 374.15 [1, 3]
for route, t, time in zip(best.solution, vrp.slot_types(), result["route_times"]):
    if route:
        print(vrp.vehicle_types()[t].name, route, round(time, 1))
```

The best plan uses one truck and three vans. The truck drives the longest route, 137.7 time
units, which no van may, and each van stays within its 90. `evaluate_routes` reports every term of the objective separately:
`route_distances`, `route_times`, `used_count` per type, `total_time`, `makespan`,
`time_excess`, `min_count_shortfall` and `total_cost`, next to the CVRP's `distance`,
`overload` and `route_loads`. The solution is feasible when `overload`, `time_excess` and
`min_count_shortfall` are all 0. `capacity()` raises `ValueError` on a fleet of several types;
read each type's from `vehicle_types()`. `with_objective_mode("Makespan")` returns a copy that
charges the longest route instead.

`Vrp.load_file` reads a mixed fleet from a `.toml` file, in optopus's format; any other file is
read as CVRPLIB:

```toml
name = "demo"                  # optional, defaults to the file name
objective_mode = "TotalTime"   # or "Makespan"
cost_weight = 1.0              # optional
rounded = false                # optional, round distances as CVRPLIB does

[depot]
x = 0.0
y = 0.0

[[vehicle_types]]              # one table per type, keys as VehicleType's arguments
name = "truck"
capacity = 30
speed = 1.0
fixed_cost = 50.0
max_count = 2

[[customers]]                  # one table per customer, ids exactly 1..n
id = 1
x = 3.0
y = 4.0
demand = 2
service_time = 2.0             # optional
```

### GraphColoring

Color the vertices so that no edge joins two of one color, using as few colors as possible.

| Constructor | Builds |
|---|---|
| `GraphColoring.from_edges(edges, num_colors=None)` | the instance over `(u, v, weight)` edges; the weights are ignored |
| `GraphColoring.from_graph(graph, num_colors=None)` | the instance over a [`Graph`](graphs_and_instances.md#graph) |
| `GraphColoring.load_file(path, num_colors=None)` | an edge-list file in the format of `MaxCut.load_file`; the weights are ignored |

| Method | Returns |
|---|---|
| `num_colors()` | the palette size; `None` above makes it `max_degree + 1`, which always admits a proper coloring |
| `penalty_weight()` | the penalty per conflicting edge, `num_vertices + 1` |
| `graph()` | the underlying graph, as a copy |
| `evaluate_colors(colors)` | a dict with `objective`, `colors_used` and `conflicts` |

```python
# The Petersen graph: an outer 5-cycle, spokes, and an inner pentagram. It needs three colors.
petersen = (
    [(i, (i + 1) % 5, 1.0) for i in range(5)]
    + [(i, i + 5, 1.0) for i in range(5)]
    + [(5 + i, 5 + (i + 2) % 5, 1.0) for i in range(5)]
)
gc = optopus.GraphColoring.from_edges(petersen)
print(gc.num_colors(), gc.penalty_weight())  # 4 11

ts = optopus.TabuSearch("Recolor", (2, 5), stop=optopus.StopCondition(max_iteration=2_000))
best = ts.run(gc, runs=3, seed=42).runs[0]
print(gc.evaluate_colors(best.solution))
# {'objective': 3, 'colors_used': 3, 'conflicts': 0}
```

A reported value below `penalty_weight()` means the coloring is proper and equals the number of
colors used. Expect a search to reach a proper coloring quickly and to shed colors slowly:
emptying a color class takes every vertex in it moving away one at a time, with no reward until
the last one leaves. To ask whether `k` colors suffice, pass `num_colors=k` and check for zero
conflicts:

```python
two = optopus.GraphColoring.from_edges(petersen, num_colors=2)
best = ts.run(two, runs=3, seed=42).runs[0]
print(two.evaluate_colors(best.solution)["conflicts"])  # 3 — two colors do not suffice
```

### Qap

The quadratic assignment problem: place `n` facilities at `n` locations, one each, minimizing
`Σ_i Σ_j flow[i][j] · distance[p[i]][p[j]]` for the assignment `p`.

| Constructor | Builds |
|---|---|
| `Qap.from_matrices(flow, distance, name="")` | the instance from two `n × n` integer matrices; neither needs to be symmetric |
| `Qap.load_file(path)` | a QAPLIB instance: `n`, then the flow matrix and the distance matrix |

| Method | Returns |
|---|---|
| `cost(assignment)` | the cost of an assignment, `assignment[facility]` being its location |
| `size()` | `n` |

```python
flow = [[0, 3, 0, 2], [3, 0, 0, 1], [0, 0, 0, 4], [2, 1, 4, 0]]
distance = [[0, 22, 53, 53], [22, 0, 40, 62], [53, 40, 0, 55], [53, 62, 55, 0]]
qap = optopus.Qap.from_matrices(flow, distance)

ts = optopus.TabuSearch("Swap", (1, 2), stop=optopus.StopCondition(max_iteration=50))
best = ts.run(qap, runs=3, seed=42).runs[0]
print(best.best_objective, best.solution)  # 790.0 [2, 3, 0, 1]
print(qap.cost(best.solution))             # 790 — the optimum over all 24 assignments
```

A `"Swap"` is priced in `O(n)`, and `TabuSearch` forbids a swap that would put both facilities
back at locations they recently left, the rule of Taillard's robust tabu search, which makes it
the usual choice.

### BinPacking

Pack items into as few bins of one capacity as possible.

| Constructor | Builds |
|---|---|
| `BinPacking.from_sizes(capacity, sizes, name="")` | the instance; an item larger than a bin raises `ValueError` |
| `BinPacking.load_file(path)` | `n capacity`, then the `n` sizes, the layout of the Falkenauer instances |

| Method | Returns |
|---|---|
| `evaluate_bins(bins)` | a dict with `objective`, `num_bins`, `loads` and `bins` for a solution |
| `lower_bound()` | `⌈Σ size / capacity⌉`; a packing that reaches it is optimal |
| `first_fit(order=None)` | the bin of each item when each in turn goes into the first bin with room; sizes in decreasing order give First Fit Decreasing |
| `capacity()`, `sizes()`, `num_items()` | the instance's data |

A solution gives each item's bin, numbered from 0. Every packing the search visits fits.

```python
bins = optopus.BinPacking.from_sizes(10, [6, 4, 5, 5, 7, 3, 2, 8, 3, 7])
print(bins.lower_bound())  # 5

alns = optopus.AdaptiveLargeNeighborhoodSearch(
    stop=optopus.StopCondition(max_iteration=500), removal_fraction=0.3
)
best = alns.run(bins, runs=3, seed=42).runs[0]
print(bins.evaluate_bins(best.solution))
# {'objective': 4.0, 'num_bins': 5, 'loads': [10, 10, 10, 10, 10], 'bins': [0, 0, 1, 1, 2, 3, 4, 4, 2, 3]}
```

Counting bins alone would make most moves ties, so the search ranks packings by
`num_bins − mean((load / capacity)²)`. The second term lies in `(0, 1]`: fewer bins always wins,
and among packings with as many bins the fuller ones, from which a bin can be emptied, come
first. Above, five full bins score `5 − 1 = 4.0`.

A descent through `"Relocate"` stalls on a packing that only a chain of moves could empty a bin
from. `AdaptiveLargeNeighborhoodSearch` and `GreedyRandomizedConstruction`, which rebuild a
packing, get past that.

### Formula

Polynomials over bounded integer variables with penalty-weighted constraints. It has its own
page: [Modeling with Formula](formula.md).

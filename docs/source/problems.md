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
| `Tsp` | min tour length | `list[int]`, a permutation of the cities | `"TwoOpt"`, `"Relocate"` | `LinKernighanHelsgaun`, `AdaptiveLargeNeighborhoodSearch` |
| `JobShopScheduling` | min makespan | `list[int]`, an operation sequence | `"Swap"`, `"Relocate"` | |
| `Vrp` | min total distance, or time and cost for a mixed fleet | `list[list[int]]`, one route of customers per vehicle | `"Relocate"`, `"Swap"`, `"TwoOpt"` | `HybridGeneticSearch`, `AdaptiveLargeNeighborhoodSearch` |
| `GraphColoring` | min colors used | `list[int]`, the color of each vertex | `"Recolor"`, `"Swap"` | |
| `Formula` | a polynomial, either direction | `list[int]`, each variable | `"Change"` (alias `"Flip"`), `"Swap"`, `"Reverse"` | |

Every generic and composed heuristic runs on every problem. `GeneticAlgorithm` takes the
problem's crossover by name:

| Problem | `crossover` (default first) |
|---|---|
| `MaxCut`, `Qubo`, `Sat`, `VertexCover`, `Formula` | `"Uniform"`, `"SubProblem"` |
| `Tsp` | `"Order"`, `"SubProblem"` |
| `Vrp` | `"Order"` |
| `JobShopScheduling` | `"Ppx"` |
| `GraphColoring` | `"Uniform"` |

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
| `JobShopScheduling` | makespan | lower |
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

The report holds no best solution of its own. Pick the run whose `best_objective` is the report's,
and read its `solution`:

```python
best = min(report.runs, key=lambda r: r.best_objective)  # max for MaxCut, Sat and every Formula
```

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
every operation sequence decodes to a valid schedule, and every assignment is a cut or a MaxSAT
assignment.

## Per problem

### MaxCut

Partition the vertices into two sides, maximizing the weight of edges between them. Build it
with `from_edges([(u, v, weight), ...])` or `from_graph(graph)`. Weights are stored as 32-bit
floats, so a sum of many non-integer weights compares only within rounding; integer weights
compare exactly. `MaxCutKernel` shrinks a sparse instance exactly before the search, and
`PlantedMaxCut` builds instances whose optimum is known.

### Qubo

Minimize x^T Q x over binary x. `from_entries([(i, j, coefficient), ...])` takes integer
coefficients. A diagonal entry `(i, i, c)` is the linear term of x_i. `(i, j)` and `(j, i)` name
the same coefficient, and an entry given twice replaces the earlier one rather than adding to
it: sum your terms into one entry per pair before passing them.

### Sat

Maximize the number of satisfied clauses (MaxSAT; a satisfiable instance is solved when every
clause holds). `from_clauses(n_vars, clauses)` takes DIMACS literals: `+i` is variable `i` and
`-i` its negation, **1-indexed**, while the solution list is 0-indexed, so variable `i` is
`solution[i - 1]`. `WalkSat` stops as soon as every clause holds.

### VertexCover

Choose the fewest vertices such that every edge has a chosen end. Edge weights are ignored; pass
`1.0`. An uncovered edge is penalized as in the table above.

### Tsp

Find the shortest closed tour through every city. Build it from `(x, y)` coordinates, an
explicit distance matrix, or a TSPLIB file with `load_file`. The solution lists the cities in
visiting order; the return to the first city is implied.

A distance matrix must be symmetric, finite and non-negative, and anything else raises
`ValueError`. The moves price a reversed segment from the edges at its ends, which is only
right when a segment is as long in both directions.

### JobShopScheduling

Each job is a list of `(machine, duration)` operations that must run in order, and a machine
runs one operation at a time. Minimize the makespan, the time the last operation finishes.
`from_jobs` takes the jobs; machines are 0-indexed. A solution is a sequence in which job `j`
appears once per operation: its `k`-th occurrence schedules the `k`-th operation of job `j`, as
early as the machines allow.

### Vrp

Serve every customer once from a depot with vehicles of one capacity, minimizing the total
distance. Index `0` is the depot and customers are `1` to `n`. Build it from coordinates and
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
    stop=optopus.StopCondition(max_duration_secs=2.0),
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

Color the vertices so that no edge joins two of one color, using as few colors as possible. The
palette defaults to `max_degree + 1` colors, which always admits a proper coloring; pass
`num_colors` to change it. A reported value below `penalty_weight()` means the coloring is proper
and equals the number of colors used. `evaluate_colors` reports both parts.

Expect a search to reach a proper coloring quickly and to shed colors slowly: emptying a color
class takes every vertex in it moving away one at a time, with no reward until the last one
leaves. To ask whether `k` colors suffice, pass `num_colors=k` and check for zero conflicts.

### Formula

Polynomials over bounded integer variables with penalty-weighted constraints. It has its own
page: [Modeling with Formula](formula.md).

# Problem-specific heuristics

These heuristics are built on the structure of particular problems, so they take no
`neighbor` argument and usually beat a generic heuristic on the problems they fit. Running one
on any other problem raises `ValueError` when `run` is called.

| Heuristic | Runs on |
|---|---|
| [`WalkSat`](#walksat) | `Sat` |
| [`BreakoutLocalSearch`](#breakoutlocalsearch) | `MaxCut`; `from_parts` builds one for any problem |
| [`LinKernighanHelsgaun`](#linkernighanhelsgaun) | `Tsp` |
| [`AdaptiveLargeNeighborhoodSearch`](#adaptivelargeneighborhoodsearch) | `Vrp`, `Tsp`, `FlowShop`, `BinPacking`, a Python problem with the ruin methods |
| [`IteratedGreedy`](#iteratedgreedy) | `FlowShop`, `Tsp`, `Vrp`, `BinPacking`, a Python problem with the ruin methods |
| [`GreedyRandomizedConstruction`](#greedyrandomizedconstruction) | `Vrp`, `Tsp`, `FlowShop`, `BinPacking`, a Python problem with the ruin methods |
| [`HybridGeneticSearch`](#hybridgeneticsearch) | `Vrp` |

Their defaults are the published settings and a sensible start. Every one takes a
`StopCondition` and has the same `run(problem, runs=1, seed=None)` as the others, see
[Running a search](running.md).

## WalkSat

WalkSAT/SKC for MaxSAT. Every iteration picks an unsatisfied clause at random and flips one of
its variables: with probability `noise` a random one, otherwise the one whose flip breaks the
fewest satisfied clauses. A flip that breaks none is always taken. The run ends as soon as
every clause holds.

| Argument | Default | Meaning |
|---|---|---|
| `stop` | | `StopCondition` |
| `noise` | `0.3` | probability of a random flip, in `[0, 1]` |
| `adaptive` | `False` | tune `noise` during the run, following Hoos (2002) |

```python
import optopus

# (x1 ∨ x2) ∧ (¬x1 ∨ x3) ∧ (¬x2 ∨ ¬x3) ∧ (x3 ∨ x4) ∧ (¬x4 ∨ x1) ∧ (¬x3 ∨ ¬x4 ∨ x2)
sat = optopus.Sat.from_clauses(4, [[1, 2], [-1, 3], [-2, -3], [3, 4], [-4, 1], [-3, -4, 2]])

walksat = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=10_000))
report = walksat.run(sat, runs=3, seed=42)
print(report.best_objective)    # 6.0 — all six clauses hold
print(report.runs[0].solution)  # [True, False, True, False] — x1 to x4
```

The objective counts satisfied clauses, so a satisfiable instance is solved when it equals the
number of clauses. `adaptive=True` saves tuning `noise` on a new family of instances.

## BreakoutLocalSearch

Benlic and Hao's breakout local search for Max Cut. Each round runs a descent to a local
optimum, then perturbs it: either with directed moves that read the tabu memory, flips and
swaps, or with random flips. The longer the search goes without a new best, the stronger and
more random the perturbation gets.

| Argument | Default | Meaning |
|---|---|---|
| `tabu_tenure` | | `(min, max)`, the prohibition length, meaning what it does under `TabuSearch` |
| `t` | | rounds without a new best before the random perturbation takes over |
| `l0` | | the initial perturbation length in moves, `>= 1` |
| `p0` | | the floor of the probability of a directed perturbation, in `[0, 1]` |
| `q` | | the share of directed perturbations that flip rather than swap, in `[0, 1]` |
| `stop` | | `StopCondition` |

```python
mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(6, 12), t=1_000, l0=3, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)  # 93.0, the optimum
```

None of the arguments has a default. The paper's setting on the G-set is `t=1000`, `l0` about
`|V| / 100`, `p0=0.8` and `q=0.5`, with γ drawn from `rand[3, |V|/10]`. γ is counted twice
inside the algorithm, so that is `tabu_tenure=(6, len(vertices) // 5)` here.
[`BreakoutLocalSearch.from_parts`](heuristics_composed.md#breakoutlocalsearchfrom_parts) builds
the same framework from other heuristics, for any problem.

## LinKernighanHelsgaun

Lin-Kernighan-Helsgaun for the symmetric TSP. Every iteration looks for an improving
variable-depth edge exchange, up to `max_depth` edges at once, among each city's
`num_neighbors` nearest cities, and applies the first it finds. The run ends at a local
optimum, where no such exchange exists.

| Argument | Default | Meaning |
|---|---|---|
| `stop` | | `StopCondition`; the run may end earlier, at a local optimum |
| `num_neighbors` | `5` | candidate cities kept per city |
| `max_depth` | `5` | the most edges one exchange replaces |

```python
import math

# Twelve cities on the unit circle, listed out of order.
order = [0, 5, 10, 3, 8, 1, 6, 11, 4, 9, 2, 7]
tsp = optopus.Tsp.from_coordinates(
    [(math.cos(2 * math.pi * k / 12), math.sin(2 * math.pi * k / 12)) for k in order]
)

lkh = optopus.LinKernighanHelsgaun(stop=optopus.StopCondition(max_iteration=1_000))
report = lkh.run(tsp, runs=3, seed=42)
print(round(report.best_objective, 4))         # 6.2117 — the regular 12-gon
print(round(24 * math.sin(math.pi / 12), 4))   # 6.2117
```

A local optimum ends the run, so for more than one descent put it inside `Iterated` or
`Restart`, or use `runs`.

## AdaptiveLargeNeighborhoodSearch

Ruin and recreate. Every iteration takes part of the solution out with one of three destroy
operators and puts it back with one of two repair operators, and accepts the result under a
simulated-annealing rule. Operator pairs that have been paying off are drawn more often.

| Destroy | Removes |
|---|---|
| Random | elements drawn uniformly |
| Worst | the elements whose removal saves the most, such as the longest detours |
| Shaw | the elements most related to a random one, such as nearby customers |

| Repair | Puts back first |
|---|---|
| Greedy | the element whose cheapest place costs least, at that place |
| Regret-2 | the element that loses most if it misses its best container |

On `Vrp`, `Tsp` and `FlowShop`, and on a Python problem with `repair_around`, a descent around
the elements just put back follows every repair; `local_repair=False` leaves it out.

| Argument | Default | Meaning |
|---|---|---|
| `stop` | | `StopCondition`; one iteration is one ruin and recreate |
| `removal_fraction` | `0.15` | share of the `n` elements taken out per iteration, in `(0, 1]` |
| `cooling_rate` | `0.9995` | factor applied to the acceptance temperature every iteration, in `(0, 1]` |
| `scores` | `(4.0, 2.0, 1.0)` | what an operator pair earns for a new best, for beating the current solution, and for an accepted worse one; only the ratios matter |
| `segment_length` | `100` | iterations between two updates of the operator weights, `>= 1` |
| `reaction` | `0.1` | how much of a segment's average score enters the weights, in `[0, 1]` |
| `max_removal` | `50` | ceiling on the elements taken out per iteration, `>= 1` |
| `local_repair` | `True` | whether the descent follows every repair |

```python
bins = optopus.BinPacking.from_sizes(10, [6, 4, 5, 5, 7, 3, 2, 8, 3, 7])
alns = optopus.AdaptiveLargeNeighborhoodSearch(
    stop=optopus.StopCondition(max_iteration=500), removal_fraction=0.3
)
best = alns.run(bins, runs=3, seed=42).runs[0]
print(bins.evaluate_bins(best.solution)["num_bins"], bins.lower_bound())  # 5 5 — optimal
```

- It removes `round(removal_fraction · n)` elements, at least 1, at most `n − 1` and at most
  `max_removal`.
  On a small instance the default removes one element at a time, which a ruin and recreate
  cannot improve with; raise it so that several elements move together, as above. On a large
  one a smaller fraction makes each iteration cheaper, since repair prices every place for every
  removed element, and buys more iterations in the same time.
- The acceptance temperature starts where a solution 5% worse than the starting one is accepted
  half the time. The default `cooling_rate` barely cools within a few thousand iterations. For a
  fixed time budget, measure the iterations a short run makes, then choose `cooling_rate` so
  that the temperature falls by two or three orders of magnitude over the budget:
  `cooling_rate = 0.001 ** (1 / iterations)`.
- A [problem written in Python](python_problems.md#ruin-methods) runs it once it defines the
  ruin methods.

## IteratedGreedy

Iterated greedy, proposed for the permutation flow shop. Every iteration takes `removal_count`
elements out at random, puts each back where the objective grows least, and descends around
them. The result replaces the current solution when it is no worse, and otherwise with
probability `exp(−Δ / temperature)` at a constant temperature. On Taillard's flow shop instances
it beats `AdaptiveLargeNeighborhoodSearch`.

It runs on every problem `AdaptiveLargeNeighborhoodSearch` does, with the same descent: by
insertion on `FlowShop`, the one around the reinserted elements on `Tsp` and `Vrp`,
`repair_around` on a Python problem, and none on `BinPacking`. Off the flow shop, give the
temperature in the units of the objective with `temperature`.

| Argument | Default | Meaning |
|---|---|---|
| `stop` | | `StopCondition`; one iteration is one destruction and reconstruction with its descent |
| `removal_count` | `4` | jobs taken out per iteration, `>= 1` |
| `temperature_factor` | `0.4` | on `FlowShop`, the temperature is this times a tenth of the mean processing time; positive and finite |
| `temperature` | `None` | the temperature itself, in the units of the objective; required off `FlowShop`, where it replaces the one above |
| `local_repair` | `True` | whether the descent follows every reconstruction |

```python
# 3 machines × 5 jobs: times[machine][job].
shop = optopus.FlowShop.from_processing_times(
    [[5, 9, 8, 10, 1], [9, 3, 10, 1, 8], [9, 4, 5, 8, 6]]
)
print(shop.makespan(shop.neh()))  # 45 — the NEH construction, the usual baseline

ig = optopus.IteratedGreedy(stop=optopus.StopCondition(max_iteration=200), removal_count=2)
report = ig.run(shop, runs=3, seed=42)
print(report.best_objective)     # 44.0 — the optimum over all 120 orders
```

`removal_count=4, temperature_factor=0.4` is the tuned setting of Ruiz and Stützle; on a
handful of jobs, take out fewer.

## GreedyRandomizedConstruction

The construction step of GRASP. A call takes every element out and builds a new solution from
nothing, one element at a time: each step prices every element left at its cheapest place and
puts one, drawn from those within `alpha` of the cheapest, where it is cheapest. It is a single
step, not a search: one call builds exactly one solution.

| Argument | Default | Meaning |
|---|---|---|
| `alpha` | `0.2` | width of the candidate list, in `[0, 1]`: `0` is pure greedy, `1` a random order |
| `stop` | `None` | can end a call before its construction, never make it build more |

GRASP is that construction as the perturbation of [`Iterated`](heuristics_composed.md#iterated),
with a descent as its search, keeping the best of the descents:

```python
grasp = optopus.Iterated(
    search=optopus.LocalSearch("TwoOpt", stop=optopus.StopCondition(max_iteration=1_000)),
    perturbation=optopus.GreedyRandomizedConstruction(alpha=0.2),
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(round(grasp.run(tsp, runs=3, seed=42).best_objective, 4))  # 6.2117
```

Run on its own, a call returns one construction, a quick starting point:

```python
greedy = optopus.GreedyRandomizedConstruction(alpha=0.0).run(tsp, seed=42).runs[0]
print(greedy.iterations, round(greedy.best_objective, 4))  # 1 6.2117
```

One construction prices every element left at every step, `O(n²)` placements for `n` elements,
which is slow on thousands of them.

## HybridGeneticSearch

Vidal's hybrid genetic search for the vehicle routing problem. Each individual is a giant tour
of all customers, split into routes by an exact dynamic program. Every iteration crosses two
parents, splits the child, improves it with a local search over each customer's `granularity`
nearest neighbors, and adds it to the population. Infeasible individuals stay alive under a
capacity penalty that adapts so that about `target_feasible` of the children are feasible, and
survivors are chosen by cost and diversity.

| Argument | Default | Meaning |
|---|---|---|
| `stop` | | `StopCondition`; one iteration is one child |
| `min_population_size` | `25` | μ, the population size survivor selection cuts back to, `>= 4` |
| `generation_size` | `40` | λ, children produced between two cuts, `>= 1` |
| `granularity` | `20` | nearest neighbors each local-search move considers, `>= 1` |
| `target_feasible` | `0.2` | the share of feasible children the penalty aims for, in `(0, 1)` |
| `restart_generations` | `20000` | restart the population after this many generations without improvement; `None` never restarts |
| `n_elite` | `4` | individuals survivor selection keeps for their cost alone, `>= 1` |
| `n_closest` | `5` | nearest individuals a diversity is averaged over, `>= 1` |
| `penalty_adaptation` | `(100, 1.2, 0.85, 0.05)` | `(period, increase, decrease, tolerance)`: every `period` children the penalty is multiplied by `increase` when the feasible share is more than `tolerance` below the target, by `decrease` when it is that far above |
| `repair_probability` | `0.5` | probability an infeasible child is also repaired under a boosted penalty, in `[0, 1]` |
| `initial_population` | `(4, 4)` | `(factor, nearest_neighbor_every)`: `factor · min_population_size` initial individuals, one in every `nearest_neighbor_every` built by nearest neighbor |
| `descent_passes` | `64` | passes the local search may spend on one child, `>= 1` |

```python
vrp = optopus.Vrp.from_coordinates(
    coordinates=[(0.0, 0.0), (10.0, 0.0), (11.0, 0.0), (-10.0, 0.0), (-11.0, 0.0)],
    demands=[0, 5, 5, 5, 5],
    capacity=15,
    num_vehicles=2,
)
hgs = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=200))
report = hgs.run(vrp, runs=3, seed=42)
print(report.best_objective)  # 44.0 — one vehicle per cluster
```

The run starts by building `4 · min_population_size` individuals, each split and improved, and
counts them as that many iterations: a run with `max_iteration=1` still builds them. On a large
`Vrp` under a short time limit, lower `min_population_size`. On a release build it produces
roughly 600 children a second on a 200-customer `Vrp` and ten times that on 30 customers.
`HybridGeneticSearch` and `AdaptiveLargeNeighborhoodSearch` measure level on `Vrp` at equal
budgets of tens of seconds, so try both. It also handles a mixed fleet, see
{ref}`Mixed fleet <vrp-mixed-fleet>`.

# Choosing a heuristic

A heuristic decides how the search moves through a problem's solutions. This page says which to
reach for, how to set its main parameters, and how to spend a time budget. Every argument is in
the [API reference](api_reference.md#heuristics).

## Running one

Every heuristic has the same `run(problem, runs=1, seed=None)` and returns a
[`RunReport`](api_reference.md#results).

- `runs` independent runs execute **one after another**, each from its own random starting
  solution. A budget of `max_duration_secs=d` per run makes the call take about `runs × d`.
- With a `seed`, run 0 uses that seed and the others derive theirs from it, so the same call
  reproduces the same report. Without one, every call differs.
- A heuristic instance holds only its configuration. Reuse it across problems and calls.

## Stopping

`StopCondition` stops a run as soon as any of its limits is reached:

| Argument | Stops after |
|---|---|
| `max_iteration` | this many iterations |
| `max_duration_secs` | this much wall-clock time |
| `max_failed_update` | this many consecutive iterations without a new best |

Set at least one: a `StopCondition()` with none is accepted, and a heuristic that does not
stop by itself then runs forever. `LocalSearch` stops by itself at a local optimum, and
`WalkSat` once every clause holds; the others run until the condition says otherwise.

A small problem often reaches its best in milliseconds, so a large `max_iteration` mostly
spends time after the answer is found. `max_failed_update` stops a run once it stagnates and
makes a good first budget when you do not yet know how long the search needs; read
`time_to_best_secs` in the report to size a fixed budget afterwards.

What one iteration is depends on the heuristic, so the same `max_iteration` means very
different amounts of work:

| Heuristic | One iteration |
|---|---|
| `LocalSearch`, `TabuSearch` | scores the **whole** neighborhood and applies one move |
| `SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `LateAcceptanceHillClimbing`, `RandomWalk` | scores one random move |
| `BeamSearch` | expands the whole neighborhood of every candidate in the beam |
| `PopulationAnnealing` | one temperature step: resampling plus `sweeps_per_step` sweeps of every replica |
| `GeneticAlgorithm`, `HybridGeneticSearch` | produces one offspring |
| `AdaptiveLargeNeighborhoodSearch` | ruins and recreates the solution once |
| `Sequential`, `Iterated`, `VariableNeighborhoodSearch`, `Restart` | counts the iterations of their steps, see [Composing heuristics](#composing-heuristics) |

A limit is checked between units of work, never inside one, so the smallest unit a heuristic
has always runs to completion and a run can overshoot a time limit by up to one unit. For most
heuristics the unit is one iteration. For a composed heuristic it is one call of a step. For
`HybridGeneticSearch` the first unit is its whole initial population, `4 · min_population_size`
individuals each split into routes and improved, which it counts as that many iterations: a run
with `max_iteration=1` still builds it. On a large `Vrp` under a short time limit that first unit
dominates; lower `min_population_size` to shorten it.

Time on a `--release` build (`maturin develop --release`, or an installed wheel). The default
development build is several times slower, and the ratio differs from heuristic to heuristic.

A time limit sidesteps the difference and is the natural unit when comparing heuristics. An
iteration limit makes a run reproducible regardless of the machine, which a time limit does
not.

## Which one

Start from the problem. When it has a heuristic of its own, try that first; it is built on
the problem's structure and usually wins.

| Problem | Start with |
|---|---|
| `Sat` | `WalkSat` |
| `MaxCut` | `BreakoutLocalSearch` |
| `Tsp` | `LinKernighanHelsgaun` |
| `Vrp` | `HybridGeneticSearch`, or `AdaptiveLargeNeighborhoodSearch` |

Otherwise pick a generic heuristic by what the search landscape needs:

| Situation | Reach for |
|---|---|
| A quick baseline, or a polishing step inside a composed heuristic | `LocalSearch` |
| A general default when one random move is cheap to score | `SimulatedAnnealing` |
| As `SimulatedAnnealing`, with one parameter instead of a temperature schedule | `LateAcceptanceHillClimbing` |
| Small neighborhoods, or a landscape where every good move goes through a worse solution (for instance `Formula` with `"Eq"` constraints) | `TabuSearch` |
| A search that stalls in one basin | `Iterated` or `VariableNeighborhoodSearch` around one of the above |
| Diversity across many candidates | `PopulationAnnealing`, `GeneticAlgorithm` |

`LocalSearch` and `TabuSearch` score every move of the neighborhood at each step. That is cheap
for a flip over a few thousand variables and expensive for a pairwise move such as `"TwoOpt"` or
`"Swap"` on a large instance, and for a Python problem it is one Python call per move.
Heuristics that score one random move per step scale to any neighborhood size.

## Setting the main parameters

### SimulatedAnnealing

A worsening move of size Δ is accepted with probability `exp(−Δ / T)`, and T is multiplied by
`cooling_rate` after every step.

- Set `initial_temperature` near a typical worsening Δ of one move, so that such a move starts
  out accepted about a third of the time. Much hotter wastes the first part of the run on a
  random walk; much colder makes it local search from the start.
- Set `cooling_rate` so that the temperature ends well below the smallest Δ that matters by
  the last iteration: `cooling_rate = (T_end / T_start) ** (1 / iterations)`. For 10⁶
  iterations from 10 to 0.01 that is about `0.999993`.
- On a problem that penalizes infeasibility (`Vrp`, `GraphColoring`, `VertexCover`, a
  `Formula` with constraints), take Δ from moves between feasible solutions, and moves that
  break a constraint are then rejected. That needs such moves to exist. When every move out of a
  feasible solution breaks a constraint, as a single `"Change"` does under an `"Eq"` on a
  `Formula`, the search has to cross infeasible solutions: set the temperature near the penalty
  weight, or use `TabuSearch`, which crosses them without a temperature.
- An objective with a small secondary term, such as a tie-breaker below one unit of a count,
  puts the useful Δ on that term's scale, often thousandths, while a move that changes the count
  costs a whole unit. Set the temperature on the small scale.
- The schedule counts steps, not seconds. Under `max_duration_secs` alone, the temperature a
  run reaches depends on how fast the machine is, so derive `cooling_rate` from an iteration
  count measured on a short run.

The typical Δ is easier to measure than to guess. For a problem written in Python, sample it
with the neighborhood's own methods:

```python
import random

def worsening_deltas(problem, neighborhood, samples=1_000, seed=0):
    rng = random.Random(seed)
    solution = problem.new_solution(rng)
    deltas = []
    for _ in range(samples):
        move = neighborhood.random_neighbor(problem, solution, rng)
        if move is not None:
            d = neighborhood.delta(problem, solution, move)
            deltas.append(d if problem.minimize else -d)
    return sorted(d for d in deltas if d > 0)
```

Its median is a reasonable `initial_temperature`, and its smallest values bound `T_end`. Sample
from a solution close to the ones the search will see, such as the result of a short
`LocalSearch`, since a random solution has larger deltas.

`BangBangSimulatedAnnealing` reheats between `min_wave_threshold` and `max_wave_threshold`
instead of cooling once, which suits long runs that would otherwise freeze early.

### TabuSearch

After a move, the variables or edges it touched stay tabu for a tenure drawn uniformly from
`tabu_tenure = (min, max)`. A tabu move is still taken when it would give a new best.

- A tenure of 1 only forbids undoing the last move, and the search cycles between two
  solutions.
- Start from a few moves and grow it with the instance, around a tenth of the number of
  variables. A tenure close to the number of variables forbids almost everything.
- A range rather than a single value, such as `(5, 15)`, keeps the search from settling into a
  cycle of fixed length.

### LateAcceptanceHillClimbing

A move is accepted when the result is no worse than the solution `history_length` steps
earlier. `1` is close to hill climbing. A few thousand is a reasonable default; longer histories
explore more and converge more slowly, so they need more iterations.

### PopulationAnnealing

Keeps `population_size` replicas and raises the inverse temperature β from `initial_beta` by
`delta_beta` per step, resampling the replicas by their Boltzmann weight. `neighbor` defaults to
`"Flip"`, which suits the binary problems; pass the problem's own move otherwise. With a pairwise
move such as `"TwoOpt"`, set `sweep_length`, because the default counts the whole neighborhood
once per run.

### Problem-specific heuristics

Their defaults are the published settings and a sensible start. The ones worth knowing:

- `WalkSat(noise=0.3)`; `adaptive=True` tunes the noise during the run.
- `BreakoutLocalSearch`'s `tabu_tenure` has the same meaning as in `TabuSearch`, so the paper's
  `rand[3, |V|/10]` is `(6, len(vertices) // 5)`.
- `HybridGeneticSearch` counts one offspring per iteration, after an initial population of
  `4 · min_population_size` (see [Stopping](#stopping)). On a release build it produces
  roughly 600 offspring a second on a 200-customer `Vrp` and ten times that on 30 customers, so
  a few seconds is already thousands of iterations.
- `AdaptiveLargeNeighborhoodSearch` removes `round(removal_fraction · n)` of the `n` elements
  per iteration, at least 1, at most `n − 1` and at most 50. On a small instance the default
  0.15 removes one element at a time, which a ruin-and-recreate cannot improve with; raise it so
  that several elements move together. On a large one a smaller fraction makes each iteration
  cheaper, since repair prices every place for every removed element, and buys more iterations
  in the same time.
- Its acceptance temperature starts where a solution 5% worse than the starting one is accepted
  half the time, and `cooling_rate` multiplies it every iteration. The default 0.9995 barely
  cools within a few thousand iterations. For a fixed time budget, measure the iterations a
  short run makes, then choose `cooling_rate` from the SA formula above so that the temperature
  falls by two or three orders of magnitude over the budget.
- For `Vrp`, `HybridGeneticSearch` and `AdaptiveLargeNeighborhoodSearch` measure level at
  equal budgets of tens of seconds, so try both. `AdaptiveLargeNeighborhoodSearch` is also the
  one that runs on `Tsp` and on a problem written in Python.

## Composing heuristics

Composed heuristics take other heuristic instances as steps, each with its own neighborhood and
its own `StopCondition`, which bounds one call of that step. The steps work on the composed
heuristic's current solution, and the best solution any step finds is kept.

| Heuristic | One round | Use it to |
|---|---|---|
| `Sequential(steps)` | runs every step in order | clean up a random start with `LocalSearch`, then hand over to `TabuSearch` |
| `Iterated(search, perturbation)` | runs `search`, then `perturbation`, and carries on from the perturbed solution | escape a basin one search stalls in; the usual first composition to try |
| `VariableNeighborhoodSearch(search, shakes)` | runs shake `k`, then `search`; keeps the result and returns to shake 0 if it improved on the round's start, otherwise restores that start and moves to shake `k + 1` | mix neighborhoods of different reach, such as `Vrp`'s inter- and intra-route moves |
| `Restart(heuristic, restart)` | runs `heuristic`, then starts over from a new random solution if `restart` is met | sample many basins when each search converges fast |
| `GeneticAlgorithm(population_size, mutation)` | recombines two parents with the problem's crossover and improves the child with `mutation` | keep a diverse population; `mutation` is typically a short `LocalSearch`. `parent_selection="DistantTopK"` and `"BiasedFitness"` read the problem's distance between solutions, `"Tournament"` does not |
| `BreakoutLocalSearch.from_parts(descent, random, directed, ...)` | breakout local search from any heuristics | use the MaxCut algorithm's framework on another problem |

Each call of a step starts that step afresh from the composed heuristic's current solution:
a `SimulatedAnnealing` step starts again at its `initial_temperature`, and a
`LateAcceptanceHillClimbing` step with an empty history. Only the solution carries over. Give
such a step a short schedule of its own rather than one sized for the whole run.

`Iterated` has no acceptance test: a perturbation that makes things worse is not undone, and
only the best solution found survives it. Use `VariableNeighborhoodSearch` with one shake when
you want a round kept only if it improves.

### How the outer `stop` counts

The outer `stop` of `Sequential`, `Iterated`, `VariableNeighborhoodSearch` and `Restart` does
not count rounds. Every iteration a step makes counts toward it, and it is checked only when a
step returns, never in the middle of one:

- `max_iteration=n` stops at the first step boundary at or after `n` iterations of the steps
  added together. `Iterated(search=LocalSearch(...), ..., stop=StopCondition(max_iteration=20))`
  stops after the first search if that search alone takes 20 iterations.
- `Sequential` checks after every step, so a limit reached inside the first step skips the
  others. `Iterated` checks between `search` and `perturbation`, and
  `VariableNeighborhoodSearch` between the shake and `search`.
- `max_failed_update=m` counts iterations since the best solution of the whole run improved,
  across all steps. A step that ends on a non-improving iteration, as every `LocalSearch` does,
  adds to it.

To run a number of rounds, multiply by the iterations one round makes. To stop when a whole
round finds nothing, use `max_failed_update` larger than one round's iterations. A time limit
avoids the arithmetic altogether.

`Restart`'s `restart` condition is read the same way, against the whole run's counters and
not from the start of the attempt. With `max_iteration=n` it restarts after every call of
`heuristic` once `n` iterations have passed in total. With `max_failed_update=m` it restarts
after every call once `m` iterations have passed without a new best, until a new best resets
the count. Give `heuristic` its own `max_failed_update` to decide how long one attempt runs.

A perturbation is typically a `RandomWalk` with a small `max_iteration`, the number of random
moves it makes:

```python
import optopus

ils = optopus.Iterated(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=1_000)),
    perturbation=optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=10)),
    stop=optopus.StopCondition(max_duration_secs=1.0),
)
mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=1))
print(ils.run(mc, runs=2, seed=42).best_objective)
```

## Problems written in Python

Every generic and composed heuristic runs on a problem written in Python, and
`AdaptiveLargeNeighborhoodSearch` does once the problem defines the ruin methods. Some need
optional members: `TabuSearch` needs `tabu_keys` on the neighborhood, `GeneticAlgorithm` a
`crossover` on the problem. Each step calls into Python, so prefer heuristics that score one
random move per step, and give every neighborhood `delta` and `random_neighbor`.
[Problems written in Python](python_problems.md) lists what each heuristic calls.

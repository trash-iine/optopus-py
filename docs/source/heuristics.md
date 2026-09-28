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

Set at least one. `LocalSearch` also stops by itself at a local optimum, and `WalkSat` once
every clause holds; the others run until the condition says otherwise.

What one iteration is depends on the heuristic, so the same `max_iteration` means very
different amounts of work:

| Heuristic | One iteration |
|---|---|
| `LocalSearch`, `TabuSearch` | scores the **whole** neighborhood and applies one move |
| `SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `LateAcceptanceHillClimbing`, `RandomWalk` | scores one random move |
| `BeamSearch` | expands the whole neighborhood of every candidate in the beam |
| `HybridGeneticSearch` | produces one offspring |

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
- The schedule counts steps, not seconds. Under `max_duration_secs` alone, the temperature a
  run reaches depends on how fast the machine is, so derive `cooling_rate` from an iteration
  count measured on a short run.

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
- `HybridGeneticSearch` counts one offspring per iteration, so budgets in the tens of thousands
  of iterations are normal.
- `AdaptiveLargeNeighborhoodSearch`'s `removal_fraction` is the share of the solution torn out
  and rebuilt per iteration.

## Composing heuristics

Composed heuristics take other heuristic instances as steps, each with its own neighborhood and
its own `StopCondition`, which bounds one call of that step. The outer `stop` bounds the whole
run.

| Heuristic | What it does | Use it to |
|---|---|---|
| `Sequential(steps)` | runs the steps in order, then repeats | clean up a random start with `LocalSearch`, then hand over to `TabuSearch` |
| `Iterated(search, perturbation)` | alternates the two, keeping a round only if it improves | escape a basin one search stalls in; the usual first composition to try |
| `VariableNeighborhoodSearch(search, shakes)` | as `Iterated` with shakes of growing strength, escalating only while they fail | mix neighborhoods of different reach, such as `Vrp`'s inter- and intra-route moves |
| `Restart(heuristic, restart)` | starts `heuristic` over from a new random solution whenever `restart` is met | sample many basins when each search converges fast |
| `GeneticAlgorithm(population_size, mutation)` | recombines parents with the problem's crossover and improves each child with `mutation` | keep a diverse population; `mutation` is typically a short `LocalSearch` |
| `BreakoutLocalSearch.from_parts(descent, random, directed, ...)` | breakout local search from any heuristics | use the MaxCut algorithm's framework on another problem |

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

# Choosing a heuristic

A heuristic decides how the search moves through a problem's solutions. This page says which to
reach for and how to set its main parameters. Each heuristic has a section of its own, with
every argument and an example, on one of four pages:

- [Generic heuristics](heuristics_generic.md) run on every problem and take its moves by name:
  `LocalSearch`, `SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `TabuSearch`,
  `LateAcceptanceHillClimbing`, `RandomWalk`, `BeamSearch`, `PopulationAnnealing`,
  `ParallelTempering`, `ReinforcementLearningSearch`.
- [Composed heuristics](heuristics_composed.md) are built from other heuristics:
  `Sequential`, `Iterated`, `VariableNeighborhoodSearch`, `Restart`, `GeneticAlgorithm`,
  `BreakoutLocalSearch.from_parts`.
- [Problem-specific heuristics](heuristics_specific.md) exploit one problem's structure:
  `WalkSat`, `BreakoutLocalSearch`, `LinKernighanHelsgaun`, `AdaptiveLargeNeighborhoodSearch`,
  `IteratedGreedy`, `GreedyRandomizedConstruction`, `HybridGeneticSearch`.
- [Exact search](heuristics_exact.md) proves an optimum: `BranchAndBound`.

How to run any of them, stop them and read their results is in
[Running a search](running.md).

## Which one

Start from the problem. When it has a heuristic of its own, try that first; it is built on
the problem's structure and usually wins.

| Problem | Start with |
|---|---|
| `Sat` | [`WalkSat`](heuristics_specific.md#walksat) |
| `MaxCut` | [`BreakoutLocalSearch`](heuristics_specific.md#breakoutlocalsearch) |
| `Tsp` | [`LinKernighanHelsgaun`](heuristics_specific.md#linkernighanhelsgaun) |
| `Vrp` | [`HybridGeneticSearch`](heuristics_specific.md#hybridgeneticsearch), or [`AdaptiveLargeNeighborhoodSearch`](heuristics_specific.md#adaptivelargeneighborhoodsearch) |
| `FlowShop` | [`IteratedGreedy`](heuristics_specific.md#iteratedgreedy) |
| `Qap` | [`TabuSearch`](heuristics_generic.md#tabusearch) with `"Swap"` |
| `BinPacking` | [`AdaptiveLargeNeighborhoodSearch`](heuristics_specific.md#adaptivelargeneighborhoodsearch) |

Otherwise pick a generic heuristic by what the search landscape needs:

| Situation | Reach for |
|---|---|
| A quick baseline, or a polishing step inside a composed heuristic | [`LocalSearch`](heuristics_generic.md#localsearch) |
| A general default when one random move is cheap to score | [`SimulatedAnnealing`](heuristics_generic.md#simulatedannealing) |
| As [`SimulatedAnnealing`](heuristics_generic.md#simulatedannealing), with one parameter instead of a temperature schedule | [`LateAcceptanceHillClimbing`](heuristics_generic.md#lateacceptancehillclimbing) |
| Small neighborhoods, or a landscape where every good move goes through a worse solution (for instance `Formula` with `"Eq"` constraints) | [`TabuSearch`](heuristics_generic.md#tabusearch) |
| A search that stalls in one basin | [`Iterated`](heuristics_composed.md#iterated) or [`VariableNeighborhoodSearch`](heuristics_composed.md#variableneighborhoodsearch) around one of the above |
| Diversity across many candidates | [`PopulationAnnealing`](heuristics_generic.md#populationannealing), [`ParallelTempering`](heuristics_generic.md#paralleltempering), [`GeneticAlgorithm`](heuristics_composed.md#geneticalgorithm) |

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

### The other heuristics

Each one's section says what its parameters do and where to start them:
[`TabuSearch`](heuristics_generic.md#tabusearch),
[`LateAcceptanceHillClimbing`](heuristics_generic.md#lateacceptancehillclimbing),
[`PopulationAnnealing`](heuristics_generic.md#populationannealing) and
[`ParallelTempering`](heuristics_generic.md#paralleltempering) on
[Generic heuristics](heuristics_generic.md).

### Problem-specific heuristics

Their defaults are the published settings and a sensible start; what is worth changing is in
each one's section on [Problem-specific heuristics](heuristics_specific.md), such as
`AdaptiveLargeNeighborhoodSearch`'s `removal_fraction` on a small instance and its
`cooling_rate` under a time budget.

## Proving optimality

None of the heuristics above can tell whether a better solution exists. `BranchAndBound` can, on
`MaxCut`, `Qubo`, `Sat`, `VertexCover` and `Formula`, with any of them searching each node; see
[Exact search](heuristics_exact.md).

## Problems written in Python

Every generic and composed heuristic runs on a problem written in Python, and
`AdaptiveLargeNeighborhoodSearch` does once the problem defines the ruin methods. Some need
optional members: `TabuSearch` needs `tabu_keys` on the neighborhood, `GeneticAlgorithm` a
`crossover` on the problem. Each step calls into Python, so prefer heuristics that score one
random move per step, and give every neighborhood `delta` and `random_neighbor`.
[Problems written in Python](python_problems.md) lists what each heuristic calls.

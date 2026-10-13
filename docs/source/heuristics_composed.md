# Composed heuristics

A composed heuristic is built from other heuristic instances, its *steps*. Each step keeps its
own neighborhood and its own `StopCondition`, which bounds one call of that step, so a single
search can mix a fast descent with a disruptive kick or a move that only one step knows. The
steps work on the composed heuristic's current solution, and the best solution any step finds
is kept. A composed heuristic runs on every problem all its steps fit, a problem-specific step
included when the problem matches.

The examples on this page share the instance of [Generic heuristics](heuristics_generic.md),
whose optimum is 93:

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
```

| Heuristic | One round | Use it to |
|---|---|---|
| [`Sequential(steps)`](#sequential) | runs every step in order | clean up a random start with `LocalSearch`, then hand over to `TabuSearch` |
| [`Iterated(search, perturbation)`](#iterated) | runs `search`, then `perturbation`, and carries on from the perturbed solution | escape a basin one search stalls in; the usual first composition to try |
| [`VariableNeighborhoodSearch(search, shakes)`](#variableneighborhoodsearch) | runs shake `k`, then `search`; keeps the result and returns to shake 0 if it improved on the round's start, otherwise restores that start and moves to shake `k + 1` | mix neighborhoods of different reach, such as `Vrp`'s inter- and intra-route moves |
| [`Restart(heuristic, restart)`](#restart) | runs `heuristic`, then starts over from a new random solution if `restart` is met | sample many basins when each search converges fast |
| [`GeneticAlgorithm(population_size, mutation)`](#geneticalgorithm) | recombines two parents with the problem's crossover and improves the child with `mutation` | keep a diverse population |
| [`BreakoutLocalSearch.from_parts(...)`](#breakoutlocalsearchfrom_parts) | a descent, then a perturbation chosen and sized by how long the search has stagnated | use the Max Cut algorithm's framework on another problem |

Each call of a step starts that step afresh from the composed heuristic's current solution:
a `SimulatedAnnealing` step starts again at its `initial_temperature`, and a
`LateAcceptanceHillClimbing` step with an empty history. Only the solution carries over. Give
such a step a short schedule of its own rather than one sized for the whole run.

## How the outer stop counts

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

## Sequential

Runs its steps one after another, each until its own stop condition, and repeats the sequence
until `stop` is met.

| Argument | Default | Meaning |
|---|---|---|
| `steps` | | heuristic instances, run in order; must not be empty |
| `stop` | | `StopCondition` for the whole run, checked after every step |

```python
seq = optopus.Sequential(
    [
        optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
        optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=200)),
    ],
    stop=optopus.StopCondition(max_iteration=1_000),
)
print(seq.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

A `Sequential` of descents over different moves, repeated until a whole round finds nothing,
polishes with all of them; [Vrp](problems.md#vrp) uses one inside a
`VariableNeighborhoodSearch`.

## Iterated

Iterated local search: runs `search`, then `perturbation`, and starts the next round from the
perturbed solution. There is no acceptance test: a perturbation that makes things worse is not
undone, and only the best solution found survives it. Use `VariableNeighborhoodSearch` with
one shake when you want a round kept only if it improves.

| Argument | Default | Meaning |
|---|---|---|
| `search` | | the heuristic that intensifies, such as `LocalSearch` |
| `perturbation` | | the heuristic that kicks the solution out of its local optimum, such as a short `RandomWalk` |
| `stop` | | `StopCondition` for the whole run |

A perturbation is typically a `RandomWalk` whose `max_iteration` is the number of random moves:

```python
ils = optopus.Iterated(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    perturbation=optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=3)),
    stop=optopus.StopCondition(max_iteration=3_000),
)
print(ils.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

Too few random moves and the next descent falls back into the same local optimum; too many and
every round starts from a random solution. GRASP is an `Iterated` whose perturbation is
[`GreedyRandomizedConstruction`](heuristics_specific.md#greedyrandomizedconstruction).

## VariableNeighborhoodSearch

Alternates an intensifying `search` with `shakes` of increasing strength. Each round applies
shake `k` and then `search`. A round that improves on its starting solution is kept and the
next starts from shake 0; one that does not is undone and the next tries shake `k + 1`, so the
search kicks harder the longer it stays stuck.

| Argument | Default | Meaning |
|---|---|---|
| `search` | | the heuristic applied after every shake, such as `LocalSearch` |
| `shakes` | | perturbations from weakest to strongest; must not be empty |
| `stop` | | `StopCondition` for the whole run |

```python
vns = optopus.VariableNeighborhoodSearch(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    shakes=[
        optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=k))
        for k in (2, 4, 8)
    ],
    stop=optopus.StopCondition(max_iteration=5_000),
)
print(vns.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

The shakes can differ in neighborhood as well as strength, such as `"Flip"` and then `"Swap"`.

## Restart

Runs `heuristic` again and again, starting over from a new random solution whenever `restart`
is met, and keeps the best solution across the attempts.

| Argument | Default | Meaning |
|---|---|---|
| `heuristic` | | the heuristic to restart; its own `stop` bounds one attempt |
| `restart` | | `StopCondition` checked after every call of `heuristic`; a new random start follows once it is met |
| `stop` | | `StopCondition` for the whole run |

`restart` is read against the whole run's counters, not from the start of the attempt. With
`max_iteration=n` it restarts after every call of `heuristic` once `n` iterations have passed in
total, so `max_iteration=0` restarts after every call: a multi-start.

```python
multistart = optopus.Restart(
    optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    restart=optopus.StopCondition(max_iteration=0),
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(multistart.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

With `max_failed_update=m` it restarts after every call once `m` iterations have passed without
a new best, until a new best resets the count. Give `heuristic` its own `max_failed_update` to
decide how long one attempt runs.

## GeneticAlgorithm

A steady-state genetic algorithm. Each iteration picks two parents from the population,
recombines them with the problem's crossover, improves the child with `mutation`, and puts it
in the population in place of the worst member.

| Argument | Default | Meaning |
|---|---|---|
| `population_size` | | members, `>= 2` |
| `mutation` | | the heuristic applied to every child, typically a short `LocalSearch` or `TabuSearch` |
| `stop` | | `StopCondition`; one iteration is one child |
| `crossover` | `None` | the problem's crossover by name; `None` is its default, see the table below |
| `sub_heuristic` | `None` | the heuristic `"SubProblem"` solves the parents' disagreement with |
| `init_improvement` | `None` | a heuristic applied to every random initial member |
| `parent_selection` | `"Tournament"` | `"Tournament"`, `"DistantTopK"` or `"BiasedFitness"`, see below |
| `parent_top_k` | `None` | how many of the best members `"DistantTopK"` picks from; required by it |
| `n_elite` | `4` | members `"BiasedFitness"` keeps by cost alone; must be below `population_size` |
| `n_closest` | `5` | neighbors `"BiasedFitness"` measures a member's diversity against |

```python
ga = optopus.GeneticAlgorithm(
    population_size=20,
    mutation=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    stop=optopus.StopCondition(max_iteration=200),
)
print(ga.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

The crossovers each problem accepts, the default first:

| Problem | `crossover` |
|---|---|
| `MaxCut`, `Qubo`, `Sat`, `VertexCover`, `Formula` | `"Uniform"`, `"SubProblem"` |
| `Tsp` | `"Order"`, `"SubProblem"` |
| `Vrp`, `FlowShop`, `Qap` | `"Order"` |
| `JobShopScheduling` | `"Ppx"` |
| `BinPacking` | `"Group"` |
| `GraphColoring` | `"Uniform"` |
| a Python problem | its `crossover(a, b, rng)` method; leave `crossover` unset |

`"SubProblem"` keeps what both parents agree on and solves the rest, a smaller instance, with
`sub_heuristic`. Paired with a tabu mutation and `init_improvement`, it is the hybrid
evolutionary algorithm:

```python
hea = optopus.GeneticAlgorithm(
    population_size=10,
    mutation=optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=50)),
    stop=optopus.StopCondition(max_iteration=100),
    crossover="SubProblem",
    sub_heuristic=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50)),
    init_improvement=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50)),
)
print(hea.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

Parent selection decides how fast the population loses its diversity:

- `"Tournament"`: each parent is the better of two random members.
- `"DistantTopK"`: the most distant pair among the `parent_top_k` best members, which keeps
  recombining solutions that differ.
- `"BiasedFitness"`: Vidal's ranking by both cost and distance to the `n_closest` nearest
  members, used to pick parents and to choose who leaves the population. The `n_elite` best
  are kept on cost alone.

The last two read the distance between solutions, which every built-in problem has and a Python
problem gives with an optional `distance(a, b)`.

## BreakoutLocalSearch.from_parts

Breakout local search assembled from other heuristics, for any problem. Each round runs
`descent` to a local optimum, then applies `l` steps of one perturbation. The perturbation is
`random` right after a new best, or when a draw falls outside the directed probability, and
otherwise one of `directed`, picked by its share. The directed probability starts at 1 and
falls toward `p0` as the search stagnates; `l` returns to `l0` whenever a new local optimum
turns up and grows by one when the same one comes back; after `t` rounds without a new best the
random perturbation takes over.

| Argument | Default | Meaning |
|---|---|---|
| `descent` | | the heuristic run to a local optimum every round, such as `LocalSearch` |
| `random` | | the perturbation that reads no history, such as `RandomWalk`; one of its iterations is one perturbation step |
| `directed` | | a non-empty list of `(heuristic, share)` pairs, perturbations that read the tabu memory such as `TabuSearch`, with non-negative shares of the directed probability |
| `tabu_tenure` | | `(min, max)`, the ban length the directed perturbations record |
| `t` | | rounds without a new best before the random perturbation takes over |
| `l0` | | the initial perturbation length, `>= 1` |
| `p0` | | the floor of the directed probability, in `[0, 1]` |
| `stop` | | `StopCondition` |

```python
bls = optopus.BreakoutLocalSearch.from_parts(
    descent=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    random=optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=1)),
    directed=[(optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=1)), 1.0)],
    tabu_tenure=(2, 5),
    t=50,
    l0=2,
    p0=0.5,
    stop=optopus.StopCondition(max_iteration=3_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

A perturbation of length `l` is `l` single iterations of the chosen heuristic, so the
perturbations' own stop conditions are never read; any `StopCondition` will do there. The
schedule tells local optima apart by distance,
so a Python problem defines `distance(a, b)` or relies on `==`. On `MaxCut` the
[`BreakoutLocalSearch`](heuristics_specific.md#breakoutlocalsearch) constructor builds Benlic
and Hao's tuned version directly.

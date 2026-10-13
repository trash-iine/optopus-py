# Generic heuristics

A generic heuristic runs on every problem: the built-in ones, `Formula`, and a
[problem written in Python](python_problems.md). It moves through solutions with the problem's
own moves, which it takes by name through `neighbor`, such as `"Flip"` for a `MaxCut` or
`"TwoOpt"` for a `Tsp`. The names each problem accepts are in
[Neighborhoods](problems.md#neighborhoods), and an unknown name raises `ValueError` when `run`
is called.

Every heuristic takes a `StopCondition` and has the same `run(problem, runs=1, seed=None)`;
[Running a search](running.md) covers both, and what one iteration of each heuristic is.
[Choosing a heuristic](heuristics.md) says which to try first and how to tune the main
parameters.

The examples on this page share one instance, a random 30-vertex Max Cut whose optimum, 93, is
proven on the [Exact search](heuristics_exact.md) page:

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
```

## LocalSearch

Greedy best-improvement descent. Every iteration scores the whole neighborhood and applies the
best move; the run ends at the first local optimum, where no move improves. It is the fastest
way to a decent solution, a baseline, and the usual polishing step inside a
[composed heuristic](heuristics_composed.md).

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `stop` | | `StopCondition`; the run may end earlier, at a local optimum |

```python
ls = optopus.LocalSearch(neighbor="Flip", stop=optopus.StopCondition(max_iteration=1_000))
report = ls.run(mc, runs=5, seed=42)
print([r.best_objective for r in report.runs])  # [92.0, 87.0, 86.0, 88.0, 86.0]
print(report.avg_iterations)                    # 9.4 — each run stopped at a local optimum
```

Each run ends in a different local optimum, none of them the optimum 93. More runs sample more
of them; the heuristics below escape them instead.

## SimulatedAnnealing

Every iteration draws one random move. An improving move is always taken, and a worsening move
of size Δ with probability `exp(−Δ / T)`. The temperature `T` starts at
`initial_temperature` and is multiplied by `cooling_rate` after every iteration, so the search
wanders early and settles into a descent as it cools. A sound default whenever one random move
is cheap to score.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `initial_temperature` | | starting temperature, in units of the objective |
| `cooling_rate` | | factor applied to the temperature every iteration, such as `0.999` |
| `stop` | | `StopCondition` |

```python
sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(sa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

The schedule decides the result. [Setting the main parameters](heuristics.md#simulatedannealing)
derives both from the size of a typical worsening move and the iteration budget.

## BangBangSimulatedAnnealing

Simulated annealing whose temperature oscillates instead of cooling once: it cools by
`cooling_rate` until it falls below `min_wave_threshold`, then heats by dividing by
`cooling_rate` until it rises above `max_wave_threshold`, and so on. Each reheating lets the
search leave the basin it froze in, which suits long runs that plain annealing would spend
frozen.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `initial_temperature` | | starting temperature |
| `cooling_rate` | | factor per iteration while cooling; its inverse while heating |
| `min_wave_threshold` | | the temperature at which cooling turns into heating |
| `max_wave_threshold` | | the temperature at which heating turns into cooling |
| `stop` | | `StopCondition` |

```python
bbsa = optopus.BangBangSimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    min_wave_threshold=0.05,
    max_wave_threshold=2.0,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(bbsa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

All six arguments are positional in this order, and none has a default. One wave from
`max_wave_threshold` down to `min_wave_threshold` takes
`log(min / max) / log(cooling_rate)` iterations, about 3,700 here.

## TabuSearch

Every iteration scores the whole neighborhood and applies the best move that is not tabu, even
when it makes the solution worse. The variables or edges a move touches then stay tabu for a
tenure drawn from `tabu_tenure`, which keeps the search from undoing its last moves and walking
back into the local optimum it just left. A tabu move is still taken when it gives a new best.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `tabu_tenure` | | `(min, max)`; each tenure is drawn uniformly from this range |
| `stop` | | `StopCondition` |

```python
ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(2, 5),
    stop=optopus.StopCondition(max_iteration=500),
)
print(ts.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

It crosses worse solutions without a temperature to tune, which makes it the first choice on
small neighborhoods and on a `Formula` whose `"Eq"` constraints every single move breaks. On
`Qap` a `"Swap"` is tabu by Taillard's rule, and on a Python problem the neighborhood must
define `tabu_keys`. Sizing the tenure:

- A tenure of 1 only forbids undoing the last move, and the search cycles between two
  solutions.
- Start from a few moves and grow it with the instance, around a tenth of the number of
  variables. A tenure close to the number of variables forbids almost everything.
- A range rather than a single value, such as `(5, 15)`, keeps the search from settling into a
  cycle of fixed length.

## LateAcceptanceHillClimbing

Every iteration draws one random move and accepts it when the result is no worse than the
current solution, or than the solution `history_length` iterations earlier. Like simulated
annealing it accepts some worsening moves, but its one parameter is a length instead of a
temperature schedule, and it needs no knowledge of the objective's scale.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `history_length` | | how many iterations back the comparison reaches, `>= 1` |
| `stop` | | `StopCondition` |

```python
lahc = optopus.LateAcceptanceHillClimbing(
    neighbor="Flip",
    history_length=50,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(lahc.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

`1` is close to hill climbing. Longer histories explore more and converge more slowly, so they
need more iterations; a few thousand suits a run of millions.

## RandomWalk

Every iteration applies a uniformly random move, whatever it does to the objective. The best
solution seen is still kept. On its own it is a baseline that shows how much the other
heuristics add; its real use is as the perturbation of `Iterated` and the shakes of
`VariableNeighborhoodSearch`, where `max_iteration` is the number of random moves, the kick's
strength.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `stop` | | `StopCondition`; required, since a random walk never stops by itself |

```python
rw = optopus.RandomWalk(neighbor="Flip", stop=optopus.StopCondition(max_iteration=1_000))
print(rw.run(mc, runs=3, seed=42).best_objective)  # 81.0 — the baseline to beat
```

## BeamSearch

Keeps `beam_width` candidate solutions. Every iteration expands the whole neighborhood of every
candidate and keeps the best `beam_width` of the solutions it reached. A width of 1 is a descent
that also takes the best worsening move at a local optimum.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `beam_width` | | candidates kept, `>= 1` |
| `stop` | | `StopCondition` |

```python
bs = optopus.BeamSearch(neighbor="Flip", beam_width=4, stop=optopus.StopCondition(max_iteration=50))
print(bs.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

One iteration scores `beam_width` whole neighborhoods, so it is expensive with a pairwise move
on a large instance.

## PopulationAnnealing

Keeps `population_size` replicas and anneals them together. Each temperature step raises the
inverse temperature β by `delta_beta`, resamples the replicas in proportion to their Boltzmann
weight, so that good replicas are copied and poor ones dropped, and sweeps every replica
`sweeps_per_step` times with Metropolis moves. Every `reset_period` steps the schedule starts
again from `initial_beta`.

| Argument | Default | Meaning |
|---|---|---|
| `population_size` | | replicas, `>= 2` |
| `stop` | | `StopCondition`; one iteration is one sweep of every replica |
| `initial_beta` | `0.1` | starting inverse temperature, `> 0` |
| `delta_beta` | `0.02` | added to β every step, `> 0` |
| `sweeps_per_step` | `50` | Metropolis sweeps per replica per step, `>= 1` |
| `reset_period` | `400` | steps between restarts of the schedule; `0` never restarts |
| `neighbor` | `"Flip"` | the move driving the sweeps; pass the problem's own when it has no `"Flip"` |
| `sweep_length` | `None` | proposals per sweep; `None` is the neighborhood's size |

```python
pa = optopus.PopulationAnnealing(
    population_size=20,
    stop=optopus.StopCondition(max_iteration=200),
    initial_beta=0.2,
    delta_beta=0.1,
    sweeps_per_step=10,
)
print(pa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

β is in inverse units of the objective, and the run reaches `initial_beta + steps · delta_beta`
at its coldest, where `steps` is `max_iteration / sweeps_per_step`. With the defaults, a short
run stays hot: 200 iterations is four steps, to about β = 0.18, and 10,000 iterations reach only
β = 4.1. Size `delta_beta` so the last steps make worsening moves rare. With a pairwise move such as `"TwoOpt"`, set `sweep_length`, since
the default sweep proposes every pair.

## ParallelTempering

Keeps `num_replicas` replicas at fixed inverse temperatures, a geometric ladder from
`beta_min` (hottest) to `beta_max` (coldest). Every round sweeps each replica
`sweeps_per_exchange` times at its own temperature, then lets neighboring replicas trade
places. A good configuration a hot replica finds walks down the ladder to where it is refined,
and one stuck at the cold end walks up to where it can leave. Nothing cools, which is the
difference from `PopulationAnnealing`.

| Argument | Default | Meaning |
|---|---|---|
| `num_replicas` | | replicas on the ladder, `>= 2` |
| `stop` | | `StopCondition`; one iteration is one sweep of every replica |
| `beta_min` | `0.1` | inverse temperature of the hottest replica, `> 0` |
| `beta_max` | `5.0` | inverse temperature of the coldest replica, above `beta_min` |
| `sweeps_per_exchange` | `1` | sweeps between two exchange attempts, `>= 1` |
| `neighbor` | `"Flip"` | the move driving the sweeps |
| `sweep_length` | `None` | proposals per sweep; `None` is the neighborhood's size |

```python
pt = optopus.ParallelTempering(num_replicas=8, stop=optopus.StopCondition(max_iteration=1_000))
print(pt.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

The temperatures are in units of the objective: `beta_max` should make the coldest replica a
descent and `beta_min` let the hottest move freely, with replicas close enough that neighbors
overlap in energy, or a good configuration never travels down the ladder. When widening the
range, add replicas with it. With a pairwise move, set `sweep_length` as for
`PopulationAnnealing`.

## ReinforcementLearningSearch

Learns which moves to take while it searches. Every iteration describes each candidate move by
21 features, its gain, whether it improves and its rank among the candidates, each on its own
and crossed with six features of the search such as how far the run has got and how long it has
stagnated. It picks a move from a softmax over a linear score of those features, and updates the weights by REINFORCE from the reward the move earned.

| Argument | Default | Meaning |
|---|---|---|
| `neighbor` | | the move, by name |
| `stop` | | `StopCondition` |
| `learning_rate` | `0.01` | policy-gradient step, `>= 0`; `0` never learns |
| `softmax_temperature` | `1.0` | temperature of the softmax over move scores, `> 0`; higher picks more uniformly |
| `reward_shaping` | `"Normalized"` | `"Normalized"`: the move's gain over the step's largest; `"Raw"`: the gain itself; `"BestImprovement"`: 1 for a new best, else 0 |
| `policy_weights` | `None` | 21 starting weights, one per feature; `None` starts at zero |
| `max_candidates` | `None` | moves sampled per iteration; `None` scores the whole neighborhood |

```python
rl = optopus.ReinforcementLearningSearch(
    neighbor="Flip", stop=optopus.StopCondition(max_iteration=2_000)
)
print(rl.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

The policy starts from scratch in every run. On a large neighborhood, set `max_candidates` to a
few hundred: scoring every move each iteration is what makes it slow.

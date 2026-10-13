# Examples

These examples compare heuristics and aggregate runs. They assume `optopus` is
[installed](installation.md).

## Aggregating statistics over multiple runs

Metaheuristics are stochastic, so a single run rarely tells the whole story. Pass `runs=N` to repeat
the search and read aggregate statistics from the `RunReport`.

```python
import optopus

mc = optopus.MaxCut.from_edges([
    (0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0), (2, 3, 1.5), (0, 3, 2.0),
])

ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(3, 7),
    stop=optopus.StopCondition(max_iteration=5_000),
)

report = ts.run(mc, runs=10, seed=1)

print(f"best  = {report.best_objective}")
print(f"avg   = {report.avg_objective:.2f}")
print(f"worst = {report.worst_objective}")
print(f"std   = {report.std_objective:.3f}")
print(f"avg time to best = {report.avg_time_to_best_secs * 1e3:.2f} ms")
```

## Comparing heuristics on the same problem

```python
import optopus

q = optopus.Qubo.from_entries([
    (0, 0, -1), (0, 1, 2), (1, 1, -1), (1, 2, 1), (2, 2, -2),
])
stop = optopus.StopCondition(max_iteration=5_000)

heuristics = {
    "LocalSearch": optopus.LocalSearch(neighbor="Flip", stop=stop),
    "SimulatedAnnealing": optopus.SimulatedAnnealing(
        neighbor="Flip", initial_temperature=5.0, cooling_rate=0.995, stop=stop
    ),
    "TabuSearch": optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 7), stop=stop),
    "LateAcceptanceHillClimbing": optopus.LateAcceptanceHillClimbing(
        neighbor="Flip", history_length=20, stop=stop
    ),
}

for name, h in heuristics.items():
    report = h.run(q, runs=5, seed=1)
    print(f"{name:28} best={report.best_objective} avg={report.avg_objective:.2f}")
```

## Benchmarking specialized heuristics on a random instance

Problem-specific heuristics and `VariableNeighborhoodSearch` share the same `run` interface, so they
drop into the same comparison loop. Generating the instance from a `Graph` keeps the benchmark
reproducible end to end.

```python
import optopus

mc = optopus.MaxCut.from_graph(
    optopus.Graph.erdos_renyi(500, 0.02, seed=42).with_random_weights((1, 10), seed=42)
)
stop = optopus.StopCondition(max_duration_secs=0.2)

heuristics = {
    "LocalSearch": optopus.LocalSearch(neighbor="Flip", stop=stop),
    "TabuSearch": optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 50), stop=stop),
    "VariableNeighborhoodSearch": optopus.VariableNeighborhoodSearch(
        search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=200)),
        shakes=[
            optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=5)),
            optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=20)),
        ],
        stop=stop,
    ),
    "BreakoutLocalSearch": optopus.BreakoutLocalSearch(
        tabu_tenure=(6, 100), t=1_000, l0=20, p0=0.8, q=0.5, stop=stop
    ),
    "PopulationAnnealing": optopus.PopulationAnnealing(population_size=20, stop=stop),
}

for name, h in heuristics.items():
    report = h.run(mc, runs=3, seed=1)
    print(f"{name:28} best={report.best_objective:.0f} avg={report.avg_objective:.1f}")
```

## More examples

Every class has a runnable example in its own section:

- [Running a search](running.md): `run`, seeds and reproducibility, `StopCondition`, and every
  field of `RunReport` and `RunResult`, `improvement` included.
- [Built-in problems](problems.md#per-problem): one example per problem.
- [Generic](heuristics_generic.md), [composed](heuristics_composed.md),
  [problem-specific](heuristics_specific.md) heuristics and [exact search](heuristics_exact.md):
  one example per heuristic.
- [Graphs and test instances](graphs_and_instances.md): random graphs, measuring how often a
  heuristic reaches a planted optimum, and solving through a kernel.
- [Modeling with Formula](formula.md#examples) and
  [Problems written in Python](python_problems.md#example).

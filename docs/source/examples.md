# Examples

These examples assume `optopus` is [installed](installation.md).

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
stop = optopus.StopCondition(max_duration_secs=1.0)

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
        tabu_tenure=(3, 50), t=1_000, l0=20, p0=0.8, q=0.5, stop=stop
    ),
    "RlBreakoutLocalSearch": optopus.RlBreakoutLocalSearch(
        tabu_tenure=(3, 50), t=1_000, l0=20, stop=stop
    ),
    "PopulationAnnealing": optopus.PopulationAnnealing(population_size=20, stop=stop),
}

for name, h in heuristics.items():
    report = h.run(mc, runs=3, seed=1)
    print(f"{name:28} best={report.best_objective:.0f} avg={report.avg_objective:.1f}")
```

## Reading the improvement metric

`improvement` is sign-corrected so that **positive always means better**, regardless of whether the
problem is a maximization (`MaxCut`) or minimization (`Qubo`) problem.

```python
run = report.runs[0]
print(f"initial objective = {run.initial_objective}")
print(f"best objective    = {run.best_objective}")
print(f"improvement       = {run.improvement}")  # always >= 0 for a successful search
```

## Reproducibility

```python
import optopus

mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])
stop = optopus.StopCondition(max_iteration=5_000)

a = optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 7), stop=stop).run(mc, runs=3, seed=99)
b = optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 7), stop=stop).run(mc, runs=3, seed=99)

assert a.best_objective == b.best_objective  # identical with the same seed
```

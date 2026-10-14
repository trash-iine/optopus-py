# Running a search and reading results

Every heuristic is used the same way: build it with a `StopCondition`, call
`run(problem, runs=1, seed=None)`, and read the `RunReport` it returns. This page covers those
three pieces. Which heuristic to build is in [Choosing a heuristic](heuristics.md).

## `run`

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    stop=optopus.StopCondition(max_iteration=10_000),
)

report = sa.run(mc, runs=4, seed=7)
print(report)
# RunReport(num_runs=4, best_objective=93, avg_objective=93, worst_objective=93, std_objective=0)
```

| Argument | Default | Meaning |
|---|---|---|
| `problem` | | a built-in problem, a `Formula`, or a [problem written in Python](python_problems.md) |
| `runs` | `1` | independent runs, each from its own random starting solution; `0` raises `ValueError` |
| `seed` | `None` | master seed. Run 0 uses it as is and the others derive theirs from it, so the same call reproduces the same report. `None` draws a fresh seed every call |

- The runs execute **one after another**. A budget of `max_duration_secs=d` per run makes the
  call take about `runs × d`.
- A built-in problem runs without the GIL, so other Python threads keep running meanwhile. A
  problem written in Python calls back into Python at every step.
- A heuristic instance holds only its configuration. Reuse it across problems and calls.
- A heuristic that does not fit the problem, such as a `neighbor` the problem does not have or
  `WalkSat` on a `MaxCut`, raises `ValueError` when `run` is called, not when it is built.

Two calls with the same seed give the same results:

```python
a = sa.run(mc, runs=3, seed=99)
b = sa.run(mc, runs=3, seed=99)
assert [r.solution for r in a.runs] == [r.solution for r in b.runs]
```

That holds under an iteration limit; see [Stopping](#stopping) for time limits.

## Stopping

`StopCondition` stops a run as soon as any of its limits is reached:

| Argument | Default | Stops after |
|---|---|---|
| `max_iteration` | `None` | this many iterations |
| `max_duration_secs` | `None` | this much wall-clock time, in seconds; negative or infinite raises `ValueError` |
| `max_failed_update` | `None` | this many consecutive iterations without a new best |

```python
optopus.StopCondition(max_iteration=10_000)
optopus.StopCondition(max_duration_secs=0.5)
optopus.StopCondition(max_iteration=100_000, max_failed_update=2_000)  # whichever comes first
```

Set at least one: a `StopCondition()` with none is accepted, and a heuristic that does not
stop by itself then runs forever. `LocalSearch` and `LinKernighanHelsgaun` stop by themselves
at a local optimum, `WalkSat` once every clause holds, and `GreedyRandomizedConstruction` after
one construction; the others run until the condition says otherwise.

A small problem often reaches its best in milliseconds, so a large `max_iteration` mostly
spends time after the answer is found. `max_failed_update` stops a run once it stagnates and
makes a good first budget when you do not yet know how long the search needs; read
`time_to_best_secs` in the report to size a fixed budget afterwards. Size it against how the
heuristic moves: an annealing that is still hot rarely sets a new best, so a small
`max_failed_update` ends it before it cools:

```python
early = optopus.SimulatedAnnealing(
    "Flip", 2.0, 0.999, optopus.StopCondition(max_failed_update=500)
).run(mc, seed=7)
run = early.runs[0]
print(run.iterations, run.best_objective)  # 746 89.0 — stopped long before it cooled
```

What one iteration is depends on the heuristic, so the same `max_iteration` means very
different amounts of work:

| Heuristic | One iteration |
|---|---|
| `LocalSearch`, `TabuSearch` | scores the **whole** neighborhood and applies one move |
| `SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `LateAcceptanceHillClimbing`, `RandomWalk` | scores one random move |
| `ReinforcementLearningSearch` | scores the whole neighborhood, or `max_candidates` moves of it, and applies one |
| `BeamSearch` | expands the whole neighborhood of every candidate in the beam |
| `PopulationAnnealing` | one sweep of every replica; a temperature step of `sweeps_per_step` sweeps counts that many |
| `ParallelTempering` | one sweep of every replica; a round of `sweeps_per_exchange` sweeps and one exchange counts that many |
| `GeneticAlgorithm`, `HybridGeneticSearch` | produces one offspring |
| `AdaptiveLargeNeighborhoodSearch`, `IteratedGreedy` | ruins and recreates the solution once |
| `GreedyRandomizedConstruction` | builds one solution, and a call makes exactly one |
| `WalkSat` | flips one variable |
| `LinKernighanHelsgaun` | applies one improving variable-depth exchange |
| `BreakoutLocalSearch` | one move of its descent or of a perturbation |
| `Sequential`, `Iterated`, `VariableNeighborhoodSearch`, `Restart` | counts the iterations of their steps, see [How the outer stop counts](heuristics_composed.md#how-the-outer-stop-counts) |
| `BranchAndBound` | takes one node, on top of the iterations its `heuristic` makes there, see [Exact search](heuristics_exact.md) |

A limit is checked between units of work, never inside one, so the smallest unit a heuristic
has always runs to completion and a run can overshoot a limit by up to one unit. For most
heuristics the unit is one iteration. For `PopulationAnnealing` it is a temperature step, so a
run makes at least `sweeps_per_step` iterations. For a composed heuristic it is one call of a
step. For `HybridGeneticSearch` the first unit is its whole initial population, see
[HybridGeneticSearch](heuristics_specific.md#hybridgeneticsearch).

The iterations a time limit allows depend on the machine and the build: the default
development build is several times slower than `maturin develop --release` or an installed
wheel, so time anything on those. A time limit is the natural unit when comparing heuristics;
only an iteration limit reproduces a run exactly, whatever the machine.

## The report

`run` returns a `RunReport`: one `RunResult` per run in `report.runs`, and statistics over
them. Every attribute is read-only.

| `RunReport` attribute | Meaning |
|---|---|
| `runs` | the `RunResult` of every run, in run order |
| `num_runs` | how many runs there were |
| `best_objective`, `worst_objective` | the best and worst `best_objective` over the runs, in the problem's direction |
| `avg_objective`, `std_objective` | mean and population standard deviation of `best_objective` |
| `best_time_to_best_secs`, `avg_time_to_best_secs` | fastest and mean `time_to_best_secs` |
| `avg_total_time_secs` | mean `total_time_secs` |
| `avg_initial_objective`, `avg_improvement` | means of `initial_objective` and `improvement` |
| `avg_iterations`, `avg_n_accepted`, `avg_n_rejected`, `avg_n_best_updates` | means of the per-run counters |

| `RunResult` attribute | Meaning |
|---|---|
| `solution` | the best solution of the run; its shape per problem is in [Built-in problems](problems.md#at-a-glance) |
| `best_objective` | its objective, on the scale the search ranks by, see [What a run reports](problems.md#what-a-run-reports) |
| `initial_objective` | the objective of the random starting solution |
| `improvement` | from `initial_objective` to `best_objective`, positive when better, whichever the direction |
| `best_iteration` | the iteration the best solution was found at |
| `iterations` | iterations the run made, in the heuristic's unit (see the table above) |
| `time_to_best_secs`, `total_time_secs` | seconds until the best solution, and for the whole run |
| `n_accepted`, `n_rejected` | moves applied, and iterations that applied none |
| `n_best_updates` | how often the best solution strictly improved |
| `seed` | the seed this run used; `None` when `run` got no seed |
| `trajectory` | every improvement of the best solution, as `(elapsed_secs, iteration, objective)`; see below |
| `proven_optimal`, `dual_bound` | what `BranchAndBound` proved, see [Exact search](heuristics_exact.md); `None` for every other heuristic |

```python
run = report.runs[0]
print(run)
# RunResult(best_objective=93, best_iteration=1745, time_to_best_secs=..., total_time_secs=...)
print(run.seed, run.iterations, run.n_accepted, run.n_rejected)  # 7 10000 729 9271
print(run.initial_objective, run.best_objective, run.improvement)  # 60.0 93.0 33.0
```

### How a run converged

`trajectory` lists every improvement of the run's best solution, in the order it happened, as
`(elapsed_secs, iteration, objective)` tuples. `elapsed_secs` is on the clock
`time_to_best_secs` uses and `objective` on the scale of `best_objective`. Each entry is strictly
better than the one before, so the last one carries the run's best, found at `best_iteration`.
The list is empty when the run never improved on its starting solution.

```python
for elapsed, iteration, objective in run.trajectory[-3:]:
    print(f"{elapsed * 1e3:7.3f} ms  iteration {iteration:5}  {objective}")
assert run.trajectory[-1][1:] == (run.best_iteration, run.best_objective)
```

The times differ from machine to machine. A seeded run under an iteration limit repeats its
iterations and objectives exactly, which makes them the axis to compare two runs on.

### Watching a run as it goes

`trajectory` arrives with the report. To see improvements while a run is still going, pass
`on_improvement` to `run`, a callable it calls as `on_improvement(run, elapsed_secs, objective)`.
The first improvement of each run arrives at once; after that, the latest improvement arrives
at most every 0.1 seconds, so a fast search does not spend its time in Python, and none is left
waiting when the search goes quiet. `objective` is on the scale of `best_objective`, and every
call reports a better value than the one before it in that run.

```python
def show(run, elapsed, objective):
    print(f"run {run}: {objective} after {elapsed:.3f} s")

report = optopus.SimulatedAnnealing(
    "Flip", 5.0, 0.999, optopus.StopCondition(max_iteration=5_000)
).run(mc, runs=2, seed=1, on_improvement=show)
```

The callback runs on another thread at times, so keep it short and let it only record or
forward what it gets. An exception it raises is raised from `run` once the run under way ends,
and no later run starts.

### Picking the best solution

The report holds no best solution of its own. Pick the run whose `best_objective` is the
report's and read its `solution`, with `max` on a maximizing problem and `min` on a minimizing
one:

```python
best = max(report.runs, key=lambda r: r.best_objective)  # min for a minimizing problem
print(best.solution)  # one bool per vertex: the side it is on
```

Which direction each problem reports, and why a reported value can differ from a fresh
evaluation in the last digits, is in [What a run reports](problems.md#what-a-run-reports).

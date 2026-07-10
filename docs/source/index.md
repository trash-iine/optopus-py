# optopus documentation

`optopus` provides Python bindings for the [optopus](https://github.com/trash-iine/optopus)
combinatorial-optimization library written in Rust. It lets you build optimization problems from
in-memory Python data and run metaheuristics on them — entirely in Python, with the search loop
executing in native Rust.

## What you get

- **Problems** built directly from Python data: `MaxCut` (maximization) and `Qubo` (minimization).
- **Heuristics**: `LocalSearch`, `SimulatedAnnealing`, `TabuSearch`, and
  `LateAcceptanceHillClimbing`.
- **Full result statistics**: every `run` returns a `RunReport` aggregating per-run objectives,
  timings, and acceptance counts across one or more runs.

## A 30-second example

```python
import optopus

mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])
ls = optopus.LocalSearch(neighbor="Flip", stop=optopus.StopCondition(max_iteration=10_000))
report = ls.run(mc, runs=5, seed=42)

print(report.best_objective)   # 5.0
print(report.runs[0].solution) # [True, True, False]
```

```{toctree}
---
maxdepth: 1
caption: Contents:
---

installation
quickstart
api_reference
examples
```

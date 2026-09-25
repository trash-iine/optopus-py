# optopus documentation

`optopus` provides Python bindings for the [optopus](https://github.com/trash-iine/optopus)
combinatorial-optimization library written in Rust. It lets you build optimization problems from
in-memory Python data and run metaheuristics on them — entirely in Python, with the search loop
executing in native Rust.

## What you get

- **Problems** built directly from Python data: `MaxCut`, `Qubo`, `Sat`, `VertexCover`, `Tsp`,
  `JobShopScheduling`, `Vrp`, `GraphColoring`, and `Formula` — plus a `Graph` type with random
  graph generators and periodic lattices to feed the graph-based ones.
- **Generic heuristics** that work with any problem: `LocalSearch`, `SimulatedAnnealing`,
  `BangBangSimulatedAnnealing`, `TabuSearch`, `LateAcceptanceHillClimbing`, `RandomWalk`,
  `BeamSearch`, `PopulationAnnealing`, and `ReinforcementLearningSearch`.
- **Composed heuristics** built from other heuristic instances: `VariableNeighborhoodSearch`,
  `Sequential`, `Iterated`, `Restart`, `GeneticAlgorithm`, and `BreakoutLocalSearch.from_parts`.
- **Problem-specific heuristics** that exploit one problem's structure: `WalkSat`,
  `BreakoutLocalSearch`, `LinKernighanHelsgaun`, `AdaptiveLargeNeighborhoodSearch`, and
  `HybridGeneticSearch`.
- **Instances with a known optimum**: `PlantedMaxCut` plants an optimal cut by construction, so
  "did the heuristic reach the optimum" is answerable; `MaxCutKernel` reduces an instance
  exactly and lifts the answer back.
- **Your own problems**: any Python object that provides a starting solution, an objective and
  its neighborhoods runs under the generic and composed heuristics, and under ALNS once it
  defines the ruin methods. See [](python_problems.md).
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
python_problems
api_reference
examples
```

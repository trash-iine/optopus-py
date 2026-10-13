# optopus documentation

`optopus` provides Python bindings for the [optopus](https://github.com/trash-iine/optopus)
combinatorial-optimization library written in Rust. It lets you build optimization problems from
in-memory Python data and run metaheuristics on them — entirely in Python, with the search loop
executing in native Rust.

## Install

```bash
pip install git+https://github.com/trash-iine/optopus-py.git
```

This builds the extension from source, so it needs Python ≥ 3.9 and a
[Rust toolchain](https://rustup.rs) (rustc ≥ 1.88). Prebuilt wheels for Linux x86_64 and macOS
install without Rust; [Installation](installation.md) has their commands, version pins and a
development setup.

## Use it in three steps

1. **Build a problem** from your data. Here, Max Cut on a weighted triangle: split the vertices
   into two sides so that the edges between them weigh as much as possible.
2. **Configure a heuristic** and a `StopCondition` that ends each run.
3. **Run it** and read the `RunReport`: the best objective over the runs, and each run's
   solution.

```python
import optopus

# 1. A problem: edges as (u, v, weight), vertices numbered from 0.
mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])

# 2. A heuristic: tabu search over single-vertex flips, 1,000 iterations per run.
ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(1, 2),
    stop=optopus.StopCondition(max_iteration=1_000),
)

# 3. Five independent runs, reproducible through the seed.
report = ts.run(mc, runs=5, seed=42)
print(report.best_objective)    # 5.0 — cut edges (0, 2) and (1, 2)
print(report.runs[0].solution)  # [True, True, False] — the side of each vertex
```

Every problem and every heuristic plugs into these three steps the same way: swap `MaxCut` for
`Tsp` and `"Flip"` for `"TwoOpt"`, or `TabuSearch` for `SimulatedAnnealing`, and the rest stays.

## Where to go next

| To | Read |
|---|---|
| see a few problems solved end to end | [Quickstart](quickstart.md) |
| control runs, seeds and time limits, and read every result field | [Running a search](running.md) |
| pick the problem that fits a task, and see each one solved | [Built-in problems](problems.md) |
| state your own objective and constraints as polynomials | [Modeling with Formula](formula.md) |
| write a problem as a Python class | [Problems written in Python](python_problems.md) |
| generate graphs, or instances with a known optimum | [Graphs and test instances](graphs_and_instances.md) |
| choose and tune a heuristic | [Choosing a heuristic](heuristics.md) |
| look up one heuristic's arguments and an example | [Generic](heuristics_generic.md), [composed](heuristics_composed.md), [problem-specific](heuristics_specific.md), [exact](heuristics_exact.md) |
| check a signature | [API reference](api_reference.md) |

## What you get

- **Problems** built directly from Python data: [`MaxCut`](problems.md#maxcut),
  [`Qubo`](problems.md#qubo), [`Sat`](problems.md#sat),
  [`VertexCover`](problems.md#vertexcover), [`Tsp`](problems.md#tsp),
  [`JobShopScheduling`](problems.md#jobshopscheduling), [`FlowShop`](problems.md#flowshop),
  [`Vrp`](problems.md#vrp), [`Qap`](problems.md#qap), [`BinPacking`](problems.md#binpacking),
  [`GraphColoring`](problems.md#graphcoloring), and [`Formula`](formula.md) — plus a
  [`Graph`](graphs_and_instances.md#graph) type with random graph generators and periodic
  lattices to feed the graph-based ones.
- **[Generic heuristics](heuristics_generic.md)** that work with any problem: `LocalSearch`,
  `SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `TabuSearch`,
  `LateAcceptanceHillClimbing`, `RandomWalk`, `BeamSearch`, `PopulationAnnealing`,
  `ParallelTempering`, and `ReinforcementLearningSearch`.
- **[Composed heuristics](heuristics_composed.md)** built from other heuristic instances:
  `VariableNeighborhoodSearch`, `Sequential`, `Iterated`, `Restart`, `GeneticAlgorithm`, and
  `BreakoutLocalSearch.from_parts`.
- **[Problem-specific heuristics](heuristics_specific.md)** that exploit one problem's
  structure: `WalkSat`, `BreakoutLocalSearch`, `LinKernighanHelsgaun`,
  `AdaptiveLargeNeighborhoodSearch`, `HybridGeneticSearch`, `IteratedGreedy`, and
  `GreedyRandomizedConstruction`.
- **[Exact search](heuristics_exact.md)**: `BranchAndBound` proves an optimum on `MaxCut`,
  `Qubo`, `Sat`, `VertexCover` and `Formula`, with another heuristic searching each node.
- **[Instances with a known optimum](graphs_and_instances.md#plantedmaxcut)**: `PlantedMaxCut`
  plants an optimal cut by construction, so "did the heuristic reach the optimum" is
  answerable; [`MaxCutKernel`](graphs_and_instances.md#maxcutkernel) reduces an instance
  exactly and lifts the answer back.
- **[Your own problems](python_problems.md)**: any Python object that provides a starting
  solution, an objective and its neighborhoods runs under the generic and composed heuristics,
  and under ALNS once it defines the ruin methods.
- **[Full result statistics](running.md#the-report)**: every `run` returns a `RunReport`
  aggregating per-run objectives, timings, and acceptance counts across one or more runs.

```{toctree}
---
maxdepth: 1
caption: Getting started
---

installation
quickstart
running
```

```{toctree}
---
maxdepth: 1
caption: Problems
---

problems
formula
python_problems
graphs_and_instances
```

```{toctree}
---
maxdepth: 1
caption: Heuristics
---

heuristics
heuristics_generic
heuristics_composed
heuristics_specific
heuristics_exact
```

```{toctree}
---
maxdepth: 1
caption: Reference
---

api_reference
examples
```

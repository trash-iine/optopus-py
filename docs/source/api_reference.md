# API reference

This reference is generated automatically from the docstrings of the compiled `optopus` extension
(authored as Rust `///` doc comments). For a narrative introduction, see
[Quickstart](quickstart.md); every class below also has a section with an example on the guide
pages linked from each heading.

`optopus.__version__` is the installed package's version, as a string such as `"0.1.0"`.

## Problems

A *problem* describes **what** to optimize. Each is built from in-memory Python data via a static
factory method, then handed to a heuristic.

```{eval-rst}
.. autoclass:: optopus.MaxCut
   :members:

.. autoclass:: optopus.Qubo
   :members:

.. autoclass:: optopus.Sat
   :members:

.. autoclass:: optopus.VertexCover
   :members:

.. autoclass:: optopus.Tsp
   :members:

.. autoclass:: optopus.JobShopScheduling
   :members:

.. autoclass:: optopus.Vrp
   :members:

.. autoclass:: optopus.FlowShop
   :members:

.. autoclass:: optopus.Qap
   :members:

.. autoclass:: optopus.BinPacking
   :members:

.. autoclass:: optopus.VehicleType
   :members:

.. autoclass:: optopus.GraphColoring
   :members:

.. autoclass:: optopus.Formula
   :members:
```

### Instances with a known optimum

`PlantedMaxCut` generates Max Cut instances whose optimum is exact by construction, which is what
makes "did the heuristic reach the optimum" answerable.

```{eval-rst}
.. autoclass:: optopus.PlantedMaxCut
   :members:
```

### Problem reduction

`MaxCutKernel` shrinks a Max Cut instance by exact kernelization. The kernel is an ordinary
`MaxCut`, so any heuristic runs on it unchanged, and `lift` maps the result back.

```{eval-rst}
.. autoclass:: optopus.MaxCutKernel
   :members:
```

Problems written in Python are plain objects, see [Problems written in Python](python_problems.md).

## Graph

`MaxCut`, `VertexCover` and `GraphColoring` are defined over an undirected weighted graph. Build
one explicitly from an edge list, draw one from a random graph model (Erdős-Rényi,
Barabási-Albert, Watts-Strogatz), or take a periodic lattice (`grid_torus_2d`, `grid_torus_3d`),
then hand it to the problem's `from_graph`. Examples:
[Graphs and test instances](graphs_and_instances.md).

```{eval-rst}
.. autoclass:: optopus.Graph
   :members:
```

## Heuristics

A *heuristic* describes **how** to search. Every heuristic exposes the same
`run(problem, runs=1, seed=None)` method returning a `RunReport` (see [Results](#results) below).

### Generic

These work with every problem type, Python problems included. The neighborhood is selected with
the `neighbor` argument. Examples: [Generic heuristics](heuristics_generic.md).

```{eval-rst}
.. autoclass:: optopus.LocalSearch
   :members:

.. autoclass:: optopus.SimulatedAnnealing
   :members:

.. autoclass:: optopus.TabuSearch
   :members:

.. autoclass:: optopus.LateAcceptanceHillClimbing
   :members:

.. autoclass:: optopus.RandomWalk
   :members:

.. autoclass:: optopus.BangBangSimulatedAnnealing
   :members:

.. autoclass:: optopus.BeamSearch
   :members:

.. autoclass:: optopus.PopulationAnnealing
   :members:

.. autoclass:: optopus.ParallelTempering
   :members:

.. autoclass:: optopus.ReinforcementLearningSearch
   :members:
```

### Composed

These take other heuristic instances as their steps and run on every problem the steps fit.
`GeneticAlgorithm` also needs a crossover, which each built-in problem has and a Python problem
provides as a method. Examples: [Composed heuristics](heuristics_composed.md).

```{eval-rst}
.. autoclass:: optopus.VariableNeighborhoodSearch
   :members:

.. autoclass:: optopus.Sequential
   :members:

.. autoclass:: optopus.Iterated
   :members:

.. autoclass:: optopus.Restart
   :members:

.. autoclass:: optopus.GeneticAlgorithm
   :members:
```

`BreakoutLocalSearch.from_parts` builds breakout local search from other heuristics the same way,
see `BreakoutLocalSearch` below.

### Exact search

`BranchAndBound` takes another heuristic too, and searches with it until the best solution is
proven optimal. It runs on `MaxCut`, `Qubo`, `Sat`, `VertexCover` and `Formula`, and reports
what it proved in `RunResult.proven_optimal` and `RunResult.dual_bound`. See
[Exact search](heuristics_exact.md).

```{eval-rst}
.. autoclass:: optopus.BranchAndBound
   :members:

.. autoclass:: optopus.EigenvalueRelaxation
```

### Problem-specific

These exploit the structure of one problem type and take no `neighbor` argument. Running one on a
different problem raises `ValueError`. `AdaptiveLargeNeighborhoodSearch` and
`GreedyRandomizedConstruction` also run on a Python problem that defines the ruin methods.
Examples: [Problem-specific heuristics](heuristics_specific.md).

```{eval-rst}
.. autoclass:: optopus.WalkSat
   :members:

.. autoclass:: optopus.BreakoutLocalSearch
   :members:

.. autoclass:: optopus.LinKernighanHelsgaun
   :members:

.. autoclass:: optopus.AdaptiveLargeNeighborhoodSearch
   :members:

.. autoclass:: optopus.IteratedGreedy
   :members:

.. autoclass:: optopus.GreedyRandomizedConstruction
   :members:

.. autoclass:: optopus.HybridGeneticSearch
   :members:
```

### Neighborhoods

Generic heuristics take the neighborhood as a string. The accepted values and what each move
does are listed per problem in [Neighborhoods](problems.md#neighborhoods).

## Stop condition

How the limits combine and what one iteration is for each heuristic:
[Stopping](running.md#stopping).

```{eval-rst}
.. autoclass:: optopus.StopCondition
   :members:
```

## Results

Every `run` call returns a `RunReport` aggregating one or more `RunResult` entries. What the
objective values mean for each problem, penalties included, is in
[What a run reports](problems.md#what-a-run-reports), and every field with an example in
[The report](running.md#the-report).

```{eval-rst}
.. autoclass:: optopus.RunReport
   :members:

.. autoclass:: optopus.RunResult
   :members:
```

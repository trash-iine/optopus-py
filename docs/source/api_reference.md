# API reference

This reference is generated automatically from the docstrings of the compiled `optopus` extension
(authored as Rust `///` doc comments). For a narrative introduction, see
[Quickstart](quickstart.md) and [Examples](examples.md).

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

.. autoclass:: optopus.GraphColoring
   :members:

.. autoclass:: optopus.Formula
   :members:
```

`TspWithCoordinates` remains bound to the same class as `Tsp`, so existing code keeps working.

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

## Graph

`MaxCut`, `VertexCover` and `GraphColoring` are defined over an undirected weighted graph. Build
one explicitly from an edge list, draw one from a random graph model (Erdős-Rényi,
Barabási-Albert, Watts-Strogatz), or take a periodic lattice (`grid_torus_2d`, `grid_torus_3d`),
then hand it to the problem's `from_graph`.

```{eval-rst}
.. autoclass:: optopus.Graph
   :members:
```

## Heuristics

A *heuristic* describes **how** to search. Every heuristic exposes the same
`run(problem, runs=1, seed=None)` method returning a `RunReport` (see [Results](#results) below).

### Generic

These work with every problem type; the neighborhood is selected with the `neighbor` argument.

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

.. autoclass:: optopus.VariableNeighborhoodSearch
   :members:
```

### Problem-specific

These exploit the structure of one problem type and take no `neighbor` argument. Running one on a
different problem raises `ValueError`.

```{eval-rst}
.. autoclass:: optopus.WalkSat
   :members:

.. autoclass:: optopus.BreakoutLocalSearch
   :members:

.. autoclass:: optopus.LinKernighanHelsgaun
   :members:

.. autoclass:: optopus.AdaptiveLargeNeighborhoodSearch
   :members:

.. autoclass:: optopus.HybridGeneticSearch
   :members:
```

### Neighborhoods

Generic heuristics take the neighborhood as a string. The accepted values per problem:

| Problem | `neighbor` |
|---|---|
| `MaxCut`, `Qubo`, `Sat`, `VertexCover` | `"Flip"`, `"Swap"` |
| `Tsp` | `"TwoOpt"`, `"Relocate"` |
| `JobShopScheduling` | `"Swap"`, `"Relocate"` |
| `Vrp` | `"Relocate"`, `"Swap"`, `"TwoOpt"` |
| `GraphColoring` | `"Recolor"`, `"Swap"` |
| `Formula` | `"Change"` (alias `"Flip"`), `"Swap"`, `"Reverse"` |

`Vrp`'s `"TwoOpt"` reverses a segment within one route, so on its own it cannot move a customer
between vehicles; pair it with `"Relocate"` or `"Swap"` through
`VariableNeighborhoodSearch` to do both.

## Stop condition

```{eval-rst}
.. autoclass:: optopus.StopCondition
   :members:
```

## Results

Every `run` call returns a `RunReport` aggregating one or more `RunResult` entries.

```{eval-rst}
.. autoclass:: optopus.RunReport
   :members:

.. autoclass:: optopus.RunResult
   :members:
```

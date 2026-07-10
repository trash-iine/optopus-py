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

.. autoclass:: optopus.TspWithCoordinates
   :members:

.. autoclass:: optopus.JobShopScheduling
   :members:

.. autoclass:: optopus.Formula
   :members:
```

## Heuristics

A *heuristic* describes **how** to search. Every heuristic works with every problem type and exposes
the same `run(problem, runs=1, seed=None)` method returning a `RunReport` (see [Results](#results) below).

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
```

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

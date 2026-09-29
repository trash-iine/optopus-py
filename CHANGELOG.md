# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

Tracks optopus up to `8de6180`, which reworked large parts of the library's public API. Several
entries below are breaking; see **Changed** and **Removed**.

### Added

- `Vrp`, the capacitated vehicle routing problem, with the `Relocate`, `Swap` and `TwoOpt`
  neighborhoods. Build it from coordinates, an explicit distance matrix, or a CVRPLIB file;
  `num_vehicles=0` lets optopus pick a fleet size. `evaluate_routes` scores a route partition
  and reports the raw distance and the capacity overload separately, since the objective a
  heuristic minimizes is `distance + penalty_weight * overload`.
- `GraphColoring`, minimum proper coloring, with the `Recolor` and `Swap` neighborhoods and an
  `evaluate_colors` that reports colors used and conflicts. The palette defaults to
  `max_degree + 1` and can be set explicitly.
- Problem-specific heuristics: `WalkSat` (SAT), `BreakoutLocalSearch` (Max Cut),
  `LinKernighanHelsgaun` (TSP), `AdaptiveLargeNeighborhoodSearch` (VRP and TSP) and
  `HybridGeneticSearch` (VRP). These take no `neighbor` argument and raise `ValueError` when run
  on a problem they do not apply to.
- Problems written in Python. Any object with `minimize`, `new_solution`, `objective` and a
  `neighborhoods` mapping can be passed to the generic heuristics, `PopulationAnnealing` and
  `VariableNeighborhoodSearch`. Exceptions raised in its methods, and Ctrl-C, stop the run and
  propagate from `run`.
- `VariableNeighborhoodSearch`, which alternates an intensifying search heuristic with a list of
  increasingly disruptive shakes. Steps are ordinary heuristic instances, so each may use its own
  neighborhood and stopping criterion.
- Composed heuristics built from other heuristic instances: `Sequential`, `Iterated`, `Restart`,
  `GeneticAlgorithm` (with each problem's crossover, including `"SubProblem"` over any
  `sub_heuristic`, and `Tournament`, `DistantTopK` or `BiasedFitness` parent selection), and
  `BreakoutLocalSearch.from_parts`, which assembles breakout local search from a descent and
  perturbations for any problem.
- `ReinforcementLearningSearch`, a generic heuristic with a softmax policy over move features.
- Python problems run under the composed heuristics too. `GeneticAlgorithm` uses their `crossover`
  method, solutions are compared with their optional `distance` method, and a problem with the
  ruin methods runs under `AdaptiveLargeNeighborhoodSearch`.
- `PlantedMaxCut`, which generates Max Cut instances whose optimum is exact by construction
  (`tile_planting_2d`, `tile_planting_3d`, `wishart`), plus `verify` and `has_exact_optimum` —
  the latter says whether "did the run reach the optimum" is an equality test or needs a
  tolerance.
- `MaxCutKernel`, exact kernelization of a Max Cut instance. `kernel()` is an ordinary `MaxCut`,
  so any heuristic runs on it unchanged, and `lift` / `project` / `offset` carry solutions and
  objectives between the two. `is_trivial()` reports when the rules did not fire, which is the
  case on regular and dense graphs.
- `Graph`, with the Erdős-Rényi, Barabási-Albert and Watts-Strogatz random graph generators, the
  `grid_torus_2d` / `grid_torus_3d` periodic lattices and `with_random_weights`, plus
  `MaxCut.from_graph`, `VertexCover.from_graph` and `GraphColoring.from_graph`.
- `Tsp.from_distance_matrix` and `Tsp.load_file`, so a tour no longer has to come from 2D
  coordinates.
- `PopulationAnnealing` accepts `sweep_length`, pinning the proposals per sweep. Counting the
  neighborhood is O(n) for a single-variable move but O(n²) for a pairwise one such as
  `"TwoOpt"`, where pinning a length is worth it.
- `Formula` accepts a `bounds` argument giving each variable an integer range, and the `Reverse`
  neighborhood alongside `Change` and `Swap`. `eval_objective` and `eval_penalty` score an
  assignment without running a search.
- `Vrp.penalty_weight()`, the weight the objective charges per unit of overload, matching
  `GraphColoring.penalty_weight()`.
- The Sphinx documentation is now published to GitHub Pages at
  <https://trash-iine.github.io/optopus-py/>, rebuilt from `main` on every push.
- Three guide pages in the documentation. "Built-in problems" compares the problems, the
  constraints each penalizes itself and what a run reports for each, including that
  `best_objective` carries the penalty for `VertexCover`, `Vrp`, `GraphColoring` and `Formula`
  and that a minimizing `Formula` reports it negated. "Modeling with Formula" covers the
  polynomial and constraint format, how a violation is penalized and how to choose a penalty
  weight. "Choosing a heuristic" covers which heuristic to use, what one iteration costs and how
  to set the main parameters.
- More guidance from a second simulated first use: a stop condition is checked between units of
  work, so `HybridGeneticSearch` always builds its initial population; measuring deltas to set
  a `SimulatedAnnealing` temperature, including on a secondary term and under `"Eq"`
  constraints; `AdaptiveLargeNeighborhoodSearch`'s removal cap, per-iteration cost and cooling
  for a time budget; a `Vrp` variable neighborhood search; a one-hot assignment with
  `Formula`; the `tabu_keys` contract (`(i, j)` is one key, `[i, j]` two) and the ruin methods'
  container numbering.

### Changed

- Wheels are built with the release settings optopus measured, fat LTO and a single codegen
  unit, which the binding had never applied. `TabuSearch` runs about 1.9x faster on a
  2000-vertex MaxCut; the other heuristics measured gain little.
- Updated the vendored optopus to `8de6180`.
- **`PopulationAnnealing` is now a generic heuristic**: optopus generalized it off Max Cut, so it
  runs on every problem type and takes a `neighbor` argument. It defaults to `"Flip"` and comes
  after the existing parameters, so calls that did not name it keep working. Its `cluster_moves`
  argument is gone, the operator having been dropped upstream.
- **`BreakoutLocalSearch` lost its `plateau_prob` argument**, which upstream removed.
- **`BreakoutLocalSearch`'s `tabu_tenure` now means the same prohibition length it does under
  `TabuSearch`.** Benlic and Hao's γ is counted twice inside the algorithm, so reproducing their
  `rand[3, |V|/10]` means passing `(6, |V| // 5)`.
- **`Formula` variables are integers, not booleans.** A solution is reported as a `list[int]`
  rather than a `list[bool]`; for binary variables the entries are 0 and 1, which compare equal
  to `False` and `True`. Variables are still binary unless `bounds` says otherwise.
- **`Formula`'s `Flip` neighborhood is now named `Change`**, matching optopus's integer moves.
  `Flip` remains accepted, because on a binary variable a change *is* a flip.
- `AdaptiveLargeNeighborhoodSearch` now also runs on `Tsp`, so its restriction message reads
  "only available for Vrp or Tsp".
- `pyo3/extension-module` is now an opt-in `extension-module` cargo feature rather than being
  always on, so plain `cargo build` and `cargo test` link against libpython and succeed on macOS.
  maturin still enables it for wheels via `features` in `pyproject.toml`, so built wheels are
  unchanged.
- Disabled the doctest pass (`doctest = false`); the `///` comments are Python docstrings rather
  than Rust doc examples, and rustdoc otherwise fails on the lib sharing its name with the
  `optopus` dependency.

- Inputs the search would price wrongly now raise `ValueError` instead of running: an
  asymmetric, negative or non-finite distance matrix in `Tsp.from_distance_matrix` and
  `Vrp.from_distance_matrix`, a negative `Vrp` demand, and a negative or non-finite
  `Formula` `penalty_weight` or a `Clamp` range with `lo` above `hi`. Symmetrize an
  asymmetric matrix, for example with `(d[i][j] + d[j][i]) / 2`, and state a reward as a
  positive weight on the opposite constraint instead of a negative weight.

### Removed

- **`TspWithCoordinates`, renamed to `Tsp`.** optopus renamed the problem when it gained
  non-Euclidean distances, and the binding follows without keeping an alias. Rename the call;
  nothing else about the class changed.
- **`RlBreakoutLocalSearch`.** Upstream moved the contextual-bandit perturbation controller out of
  the library and into `examples/rl_bls.rs`, so there is no longer a type to bind. The generic
  `BreakoutLocalSearch` covers the same algorithm with a fixed schedule.

### Fixed

- `Sat.from_clauses` raises `ValueError` for a literal of 0 or one naming a variable above
  `n_vars`, and `StopCondition` for a negative or non-finite `max_duration_secs`. Both used to
  fail with a `PanicException` from the core.
- `StopCondition`'s repr prints `None` and plain numbers instead of Rust's `Some(...)`.
- A large int returned by `tabu_keys`, such as `item * 10**15 + bin`, no longer aborts the
  Python process. Int keys used to index an array sized by the largest key; they are now kept
  in a map, through optopus's `TabuKey::Var`, with the same tabu behavior.
- `Formula` raises `ValueError` when a constraint reads a variable index outside `[0, n_vars)`,
  as it already did for the objective. It used to fail with a `PanicException` from the core.
- `help(optopus.Formula)` and the API reference now show the constructor's arguments, which
  were written where Python never reads them, and the signatures of `Formula`,
  `PopulationAnnealing` and `HybridGeneticSearch` show their real defaults instead of
  `Ellipsis`.
- The docstrings of `Iterated`, `Restart`, `Sequential` and `VariableNeighborhoodSearch`
  described their stop conditions and `Iterated`'s acceptance wrongly. The outer `stop` counts
  the steps' iterations and is checked between steps, `Restart`'s `restart` is read against the
  whole run, and `Iterated` does not undo a worse round.

## 0.1.0 (2026-08-06)

### Added

- Initial release: pyo3 bindings for the [optopus](https://github.com/trash-iine/optopus)
  combinatorial optimization library.
- Problem types: `MaxCut`, `Qubo`, `Sat`, `VertexCover`, `TspWithCoordinates`,
  `JobShopScheduling`, `Formula`.
- Heuristics: `LocalSearch`, `SimulatedAnnealing`, `BangBangSimulatedAnnealing`,
  `TabuSearch`, `LateAcceptanceHillClimbing`, `RandomWalk`, `BeamSearch`.
- `StopCondition`, multi-run `RunReport` / `RunResult` with seeded reproducibility.
- Prebuilt abi3 wheels for Linux x86_64 and macOS (arm64 / x86_64), published to GitHub
  Releases so installation no longer requires a Rust toolchain on those platforms.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `VariableNeighborhoodSearch`, which alternates an intensifying search heuristic with a list of
  increasingly disruptive shakes. Steps are ordinary heuristic instances, so each may use its own
  neighborhood and stopping criterion.
- `PopulationAnnealing`, a generic heuristic like `SimulatedAnnealing`: its Metropolis sweeps draw
  from the `neighbor` neighborhood (default `"Flip"`), and `sweep_length` fixes the proposals per
  sweep instead of counting the neighborhood.
- Problem-specific heuristics: `WalkSat` (SAT), `BreakoutLocalSearch` (Max Cut),
  `LinKernighanHelsgaun` (Euclidean TSP), `AdaptiveLargeNeighborhoodSearch` (VRP and Euclidean TSP)
  and `HybridGeneticSearch` (VRP). These take no `neighbor` argument and raise `ValueError` when run
  on a different problem type. `BreakoutLocalSearch`'s `tabu_tenure` is the ban length it means
  under `TabuSearch`, so Benlic and Hao's `rand[3, |V|/10]` is `(6, |V|/5)`.
- `Graph`, with the Erdős-Rényi, Barabási-Albert and Watts-Strogatz random graph generators plus
  `with_random_weights`, and `MaxCut.from_graph` / `VertexCover.from_graph` to build problems from it.
- The Sphinx documentation is now published to GitHub Pages at
  <https://trash-iine.github.io/optopus-py/>, rebuilt from `main` on every push.

### Changed

- Updated the vendored optopus to `d54f2c1`, which brings faster TSP distance lookups, allocation-free
  LKH, cheaper Job Shop neighbor evaluation, O(1) random neighbor sampling, a population annealing
  that works on every problem, and ALNS for the TSP.
- `pyo3/extension-module` is now an opt-in `extension-module` cargo feature rather than being always
  on, so plain `cargo build` and `cargo test` link against libpython and succeed on macOS. maturin
  still enables it for wheels via `features` in `pyproject.toml`, so built wheels are unchanged.
- Disabled the doctest pass (`doctest = false`); the `///` comments are Python docstrings rather than
  Rust doc examples, and rustdoc otherwise fails on the lib sharing its name with the `optopus`
  dependency.

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

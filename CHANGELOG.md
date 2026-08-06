# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `VariableNeighborhoodSearch`, which alternates an intensifying search heuristic with a list of
  increasingly disruptive shakes. Steps are ordinary heuristic instances, so each may use its own
  neighborhood and stopping criterion.
- Problem-specific heuristics: `WalkSat` (SAT), `PopulationAnnealing`, `BreakoutLocalSearch` and
  `RlBreakoutLocalSearch` (Max Cut), and `LinKernighanHelsgaun` (Euclidean TSP). These take no
  `neighbor` argument and raise `ValueError` when run on a different problem type.
- `Graph`, with the Erdős-Rényi, Barabási-Albert and Watts-Strogatz random graph generators plus
  `with_random_weights`, and `MaxCut.from_graph` / `VertexCover.from_graph` to build problems from it.
- The Sphinx documentation is now published to GitHub Pages at
  <https://trash-iine.github.io/optopus-py/>, rebuilt from `main` on every push.

### Changed

- Updated the vendored optopus to `01b88fb`, which brings faster TSP distance lookups, allocation-free
  LKH, cheaper Job Shop neighbor evaluation, and O(1) random neighbor sampling.
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

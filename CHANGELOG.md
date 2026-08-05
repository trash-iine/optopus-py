# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (unreleased)

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

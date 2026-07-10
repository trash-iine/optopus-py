# optopus

[![CI](https://github.com/trash-iine/optopus-py/actions/workflows/ci.yml/badge.svg)](https://github.com/trash-iine/optopus-py/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Python bindings for [optopus](https://github.com/trash-iine/optopus), a metaheuristic
optimization library for combinatorial problems written in Rust.

## Features

**Problem types**

- `MaxCut` — maximum cut on a weighted graph
- `Qubo` — quadratic unconstrained binary optimization
- `Sat` — boolean satisfiability (MaxSAT-style objective)
- `VertexCover` — minimum vertex cover
- `TspWithCoordinates` — traveling salesperson over 2D coordinates
- `JobShopScheduling` — job shop scheduling
- `Formula` — custom pseudo-Boolean objectives

**Heuristics**

- `LocalSearch`
- `SimulatedAnnealing`
- `BangBangSimulatedAnnealing`
- `TabuSearch`
- `LateAcceptanceHillClimbing`
- `RandomWalk`
- `BeamSearch`

Every heuristic takes a `StopCondition` (`max_iteration`, `max_duration_secs`,
`max_failed_update`) and supports reproducible multi-run experiments: `run(problem, runs=N,
seed=...)` returns a `RunReport` with the best objective, per-run `RunResult`s, and timing
statistics.

## Installation

Install directly from this repository:

```bash
pip install git+https://github.com/trash-iine/optopus-py.git
```

To pin a specific tag or commit:

```bash
pip install git+https://github.com/trash-iine/optopus-py.git@v0.1.0
```

The package is built from source, so a [Rust toolchain](https://rustup.rs) (rustc >= 1.88)
is required in addition to Python >= 3.9. pip fetches the vendored `optopus` submodule
automatically — no extra steps needed. The extension targets the stable ABI (abi3), so the
same build works on CPython 3.9+.

## Quickstart

Max Cut with local search:

```python
import optopus

# Build the graph from a weighted edge list (0-indexed vertices).
mc = optopus.MaxCut.from_edges([
    (0, 1, 1.0),
    (1, 2, 2.0),
    (0, 2, 3.0),
])

# Configure a heuristic and a stopping criterion.
ls = optopus.LocalSearch(
    neighbor="Flip",
    stop=optopus.StopCondition(max_iteration=10_000),
)

# Run it 5 times with a fixed seed for reproducibility.
report = ls.run(mc, runs=5, seed=42)

print(report.best_objective)    # 5.0  (cut edges (0,2)=3 and (1,2)=2)
print(report.runs[0].solution)  # [True, True, False] — side of each vertex
```

QUBO with simulated annealing:

```python
import optopus

# Minimize: -x0 - x1 + 2*x0*x1
q = optopus.Qubo.from_entries([
    (0, 0, -1),
    (0, 1, 2),
    (1, 1, -1),
])

sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=10.0,
    cooling_rate=0.99,
    stop=optopus.StopCondition(max_duration_secs=0.2),
)

report = sa.run(q, runs=3, seed=7)

print(report.best_objective)    # -1.0
print(report.runs[0].solution)  # [True, False] — value of each variable
```

See the Sphinx documentation under [`docs/`](docs/source/index.md) for the full API
reference and more examples.

## Development

The Rust core is vendored as a git submodule, so clone with:

```bash
git clone --recurse-submodules https://github.com/trash-iine/optopus-py.git
```

Then:

```bash
uv sync --dev            # set up the dev environment
uv run maturin develop   # build and install the extension into .venv
uv run pytest tests -v   # run the smoke tests
uv run invoke docs       # build the Sphinx docs
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. The vendored Rust core
[`optopus`](https://github.com/trash-iine/optopus) (under `vendor/optopus`) is licensed
under the same terms.

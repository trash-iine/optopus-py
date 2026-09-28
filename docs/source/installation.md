# Installation

`optopus` is a compiled extension (Rust + [PyO3](https://pyo3.rs), built with
[maturin](https://www.maturin.rs)). Prebuilt wheels are published for common platforms, so a Rust
toolchain is only needed when you build from source.

## Install a prebuilt wheel (no Rust required)

Wheels are attached to each [GitHub release](https://github.com/trash-iine/optopus-py/releases).
Install the one matching your platform:

```bash
# Linux x86_64
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl

# macOS (Apple Silicon)
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-macosx_11_0_arm64.whl

# macOS (Intel)
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-macosx_11_0_x86_64.whl
```

For another version, replace `v0.1.0` and the version in the filename with the tag you want. The
extension targets the stable ABI (abi3), so a single wheel per platform covers CPython 3.9+ — no
need to match your Python minor version.

## Install from source

For platforms without a prebuilt wheel (Windows, aarch64 Linux, musl), install from the
repository:

```bash
$ pip install git+https://github.com/trash-iine/optopus-py.git
```

To pin a specific tag or commit:

```bash
$ pip install git+https://github.com/trash-iine/optopus-py.git@v0.1.0
```

Requirements: [Python](https://www.python.org) ≥ 3.9 and a [Rust toolchain](https://rustup.rs)
(rustc ≥ 1.88). pip clones the repository, fetches the vendored `optopus` submodule
automatically, and builds the extension via maturin — no extra steps needed.

The rest of this page covers setting up a development environment from a clone.

## Prerequisites

- [Python](https://www.python.org) ≥ 3.9
- [uv](https://github.com/astral-sh/uv) — Python environment and dependency manager
- [Rust toolchain](https://rustup.rs) (`cargo`, `rustc`) — required to compile the extension

The [`optopus`](https://github.com/trash-iine/optopus) Rust library is vendored as a git submodule
at `vendor/optopus` (the binding crate references it through a path dependency,
`optopus = { path = "vendor/optopus" }`), so clone with submodules:

```bash
$ git clone --recurse-submodules https://github.com/trash-iine/optopus-py.git
```

If you already have a checkout without the submodule, initialize it with:

```bash
$ git submodule update --init
```

## Set up the development environment

From the `optopus-py` directory:

```bash
$ uv sync --dev
```

This creates `.venv/`, compiles the extension via maturin, installs it into the environment, and
pulls in the documentation tooling (Sphinx, furo, myst-parser, invoke).

## Verify the installation

```bash
$ uv run python -c "import optopus; print(optopus.MaxCut.from_edges([(0, 1, 1.0)]))"
MaxCut(num_vertices=2, num_edges=1)
```

## Standalone build (without uv)

If you prefer a plain virtual environment, you can build and install the extension directly with
maturin:

```bash
$ python -m venv .venv
$ source .venv/bin/activate
$ pip install maturin
$ maturin develop --release
```

## Build the documentation

```bash
$ uv run invoke docs
```

The generated HTML is written to `docs/build/html/`. Open `docs/build/html/index.html` in a browser
to read it.

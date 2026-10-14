# 0009. Rebuild the extension when its Rust sources change

- Date: 2026-10-14
- Status: Accepted

## Context

uv decides whether a local package needs building again from its `cache-keys`, which default to
`pyproject.toml`, `setup.py` and `setup.cfg`. None of them changes when `src/*.rs` or the
vendored optopus does, so after such a change `uv sync` and a plain `uv run` reinstall the wheel
uv built before, from the old sources. That holds for this project in its own environment and for
any project that depends on an optopus-py checkout by path: the code runs, but it is not the code
on disk, and nothing says so.

The repository has worked around it: the Invoke tasks run their commands bare after
`maturin develop`, and the docs build and the instructions use `uv run --no-sync` (ADR 0002).
That keeps a fresh `maturin develop` build in place, but a plain `uv run` or `uv sync` still
brings back the stale one.

## Options considered

- **Keep the workaround.** No change, but a stale extension stays one `uv run` away, and a
  dependent project has no workaround but `uv sync --reinstall-package optopus`.
- **`reinstall-package = ["optopus"]` in `[tool.uv]`.** Never stale, but every sync builds the
  extension, about a minute with the release profile (ADR 0006), changed or not.
- **`cache-keys` listing the Rust sources.** uv builds again exactly when a file the extension is
  built from has changed, here or in a project depending on this one by path.

## Decision

Set `[tool.uv] cache-keys` to `pyproject.toml`, `Cargo.toml`, `Cargo.lock`, `src/**/*.rs`,
`vendor/optopus/Cargo.toml` and `vendor/optopus/src/**/*.rs`. The globs name the source
directories rather than `**/*.rs`, which would also match files a build writes under `target/`
and make every sync build again. The vendored crate has no build script and no other crates.

Checked on a copy of the repository, both in its own environment and as a path dependency of
another project: with the keys, editing `src/lib.rs` or `vendor/optopus/src` makes the next
`uv run` build again and run the new code, and an unchanged tree is not built (0.01 s). Without
them, the same edit leaves the old build in place.

## Consequences

- `uv sync` and a plain `uv run` never run an extension older than its sources. The first one
  after a Rust change builds the release profile, which takes about a minute.
- After `maturin develop`, a plain `uv run` builds again too, with the release profile, and
  replaces the dev build: correct, but slow. The Invoke tasks, `docs/Makefile` and the docs
  workflow keep running bare or with `--no-sync` for that reason, no longer to avoid a stale
  wheel.
- A new source location of the extension, such as a build script or another vendored crate,
  has to be added to `cache-keys`.

# 0006. Optimize dev builds and give wheels fat LTO

- Date: 2026-09-29
- Status: Proposed

## Context

`Cargo.toml` set no profiles, and Cargo reads profiles from the root crate only. Two things
followed.

- `maturin develop`, which `invoke ci`, `invoke docs`, the README and every skill run, builds the
  unoptimized `dev` profile. The heuristics are generic and instantiated in this crate, so the
  whole search loop ran unoptimized. Agents simulating first-time users timed everything on it
  and drew wrong conclusions (a heuristic "overrunning" a budget, iteration rates far too low).
- The vendored optopus sets `lto = "fat"` and `codegen-units = 1` in its `[profile.release]`,
  measured to keep TabuSearch off an allocation-layout cliff, but that profile never reached the
  wheels built from this crate.

Measured on an Apple Silicon laptop, one run each after a warm-up, times stable to 0.02 s across
repeats: HGS and ALNS 300 iterations on a 200-customer Vrp, TabuSearch 300k iterations on a
2000-vertex Erdős-Rényi MaxCut. Rebuild is after touching `src/lib.rs`.

| Profile | Rebuild | Clean build | HGS | ALNS | Tabu |
|---|---|---|---|---|---|
| dev, opt-level 0 (before) | 1 s | 9 s | 12.20 s | 3.14 s | 22.31 s |
| dev, opt-level 1 | 4 s | 18 s | 0.92 s | 0.24 s | 0.94 s |
| dev, opt-level 3 | 4 s | | 0.69 s | 0.19 s | 3.64 s |
| dev, dependencies only at opt-level 3 | 16 s | | 0.71 s | 0.30 s | 14.17 s |
| release, no LTO (before) | 6 s | | 0.49 s | 0.14 s | 0.65 s |
| release, fat LTO, 1 codegen unit | 43 s | | 0.47 s | 0.13 s | 0.35 s |

## Options considered

- **Keep the profiles, and document `--release` for timing.** No build cost, but every tool and
  agent that runs the default build keeps producing timings 13-35x off, and the wheels stay
  without the LTO optopus measured.
- **Optimize only the dependencies in `dev`.** The usual trick, but the heuristics are
  instantiated here, so it leaves TabuSearch 22x slower than release while costing the most
  rebuild time.
- **`dev` at opt-level 3.** Fastest on HGS and ALNS, but it landed TabuSearch on the layout
  cliff (5x slower than opt-level 1), which only fat LTO removes.
- **`dev` at opt-level 1, `release` with fat LTO.** Dev builds 13-24x faster for a few seconds
  of build time, with debug assertions kept; wheels get optopus's measured release settings.

## Decision

Set `[profile.dev] opt-level = 1` and `[profile.release] lto = "fat"`, `codegen-units = 1`.
Anything that measures time builds with `maturin develop --release`.

## Consequences

- `invoke ci`, `invoke docs` and the tests run on a build fast enough for realistic instances;
  a clean build takes about 9 s longer, an incremental one about 3 s.
- Release builds take about 40 s longer, which only the release workflow and deliberate
  `--release` builds pay.
- Opt-level 1 missing the TabuSearch cliff is luck of the layout, not a property. Timings that
  matter use `--release`, which the README, AGENTS.md and the `model-problem` and
  `port-to-rust` skills say.
- `port-to-rust` now compares like with like: its Rust template already used these release
  settings, and the Python side did not.

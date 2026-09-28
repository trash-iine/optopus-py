---
name: port-to-rust
description: Port Python code that uses optopus-py (built-in problems, heuristics, or a problem written in Python) to a standalone Rust crate on the optopus core, then run both with the same seeds and compare the results. Use when asked to rewrite, port, translate or convert optopus-py code to Rust ("Rust に書き直して", "optopus で Rust 化して"), or to check a Rust port against its Python original.
allowed-tools: Bash(cargo *) Bash(uv run *) Bash(git -C vendor/optopus *) Read Write Edit
---

# Port optopus-py code to Rust

The binding maps every Python class and argument onto an optopus type; a port runs that map
backwards. The map is the binding's source, not a copy of it: read it from `src/*.rs` each
time, so the port follows whatever optopus the binding builds against today.

| To find | Read |
|---|---|
| the upstream type and constructor behind a Python problem or `Graph` method | its `#[pyclass]` in `src/problem.rs` / `src/graph.rs`: what the `#[new]` or `#[staticmethod]` calls |
| a heuristic's parameters and their defaults | its `#[pyo3(signature = ...)]` in `src/heuristic.rs` |
| the Rust move type behind a `neighbor` name, a problem-specific heuristic, a GA crossover | `build_<problem>` and `crossover_<problem>` in `src/runner.rs` |
| the upstream constructor call and argument order for each heuristic | `build_generic` and `build_nested` in `src/runner.rs` |
| `minimize`, the reported objective and the solution's Python form, per problem | the `run_all(...)` call for that problem in `solve`, `src/runner.rs` |
| how a Python problem's members reach the traits | `src/python_problem.rs` |

Read only the parts the Python code uses. A problem written in Python also needs
`references/python-problems.md`, which says how to model it in Rust.

## 1. Take stock

Read the Python code and list every optopus use:

- each problem and how it is built (constructor, file, generator and its `seed`),
- each heuristic, nested ones included, with its `neighbor`, parameters and `stop`,
- each `run(problem, runs, seed)` call, and what is read off the report,
- for a Python problem: `minimize`, `new_solution`, `objective`, every neighborhood and which
  of `delta`, `random_neighbor`, `tabu_keys` it has, and any `crossover`, `distance`, ruin
  methods or `repair_around`.

Code around the optimization (argument parsing, file I/O, plotting, pandas) is not optopus.
Ask the user whether to port it, keep it in Python, or drop it, unless they already said.

## 2. Model the problems

- Built-in problems map one to one onto the upstream type their `#[pyclass]` wraps.
- A Python problem: follow `python-problems.md`, section 1, and take the first modeling that
  fits (built-in type → `IntegerProblem` → `FormulaProblem` → hand-written traits). Tell the
  user which one you took and why, and whether the neighborhoods differ from the Python ones.

## 3. Generate the crate

Default location `<python-file-stem>-rs/` next to the Python code, unless the user named one.

```bash
git -C vendor/optopus rev-parse HEAD   # the optopus commit the binding builds against
```

Copy `templates/` (`Cargo.toml`, `src/main.rs`, `src/harness.rs`) there and fill in
`{{NAME}}`, `{{OPTOPUS_REV}}` (the commit above) and `{{SOURCE}}`. Pinning that commit is what
lets the Rust and Python runs be compared: an optopus-py wheel from a different release may
run a different core, so check the Python side runs this repository's build (step 6).

Leave `src/harness.rs` as is. It reproduces the binding's run loop (`derive_seed`, a fresh
heuristic per run, the report arithmetic); changing it breaks the comparison.

## 4. Port

Write `src/main.rs` (and modules for a hand-written problem), reading the binding as in the
table above.

- Each `heuristic.run(problem, runs=r, seed=s)` becomes one `harness::run_all(...)` call,
  followed by `report.emit("<label>")`. Copy `minimize`, `obj` and `encode` from the
  problem's `run_all(...)` call in `solve` (`encode` is its `decode` written as `json!`); a
  wrong `obj` makes every comparison fail.
- Nested heuristics are `Box<dyn Heuristic<P>>`; build the tree inside the closure passed to
  `run_all`, since each run needs a fresh one.
- Keep parameter values, `runs` and `seed` exactly as in Python. Argument orders often differ
  between the Python signature and the upstream constructor; follow the call the binding
  makes, not the Python order.
- Several `run` calls in one script: emit each with its own label and run with
  `--json 'out/{label}.json'`, which writes one report per label.
- Comments and identifiers in English, as everything in this repository.

## 5. Build

In the new crate:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Fix the code, not the lints. A trait-bound error on a heuristic usually means the move lacks
something that heuristic needs (upstream `docs/guide/custom_problem.md`, "Which heuristic
needs what").

## 6. Compare with the Python original

Make the Python side run this repository's core, built in release mode so the timings mean
something, then run both:

```bash
uv run maturin develop --release
PYTHONPATH=.claude/skills/port-to-rust/scripts uv run --no-sync python <driver.py>
(cd <crate> && cargo run --release -- --json rs.json)
uv run --no-sync python .claude/skills/port-to-rust/scripts/compare.py py.json <crate>/rs.json --mode <mode>
```

`<driver.py>` is a short script in the scratchpad, not in the user's code. It builds the
problem and heuristic as the original does (import the original module when it is
importable, copy the lines otherwise), runs it, and writes the report with
`compare.dump_report(report, "py.json", minimize=...)`.

Pick the mode:

| Case | Mode | Pass means |
|---|---|---|
| built-in problem, `stop` without `max_duration_secs`, fixed `seed` | `exact` | every run has the same seed, objectives, iteration counts and solution |
| a Python problem, any time limit, or `seed=None` | `stat` | the Rust mean is no worse than the Python mean beyond twice the spread plus 1% |

For a Python problem also check the ported objective: in the driver, after the Rust run,
call `compare.check_objective("<crate>/rs.json", problem.objective, decode=...)` (`decode`
turns the JSON solution back into what `objective` takes, e.g. `tuple`). Every run must
match. If `stat` fails with the direction reversed or far off, check `minimize` and the
`Evaluate` impls first (`python-problems.md`, section 4).

On an `exact` mismatch, the first differing field points at the cause:
`initial_objective` → the problem or `obj` differs; `seed` → `runs`/`seed` differ;
`n_accepted`/`best_iteration` → the heuristic, its parameters or the neighbor differ.
Fix and go back to step 4. Do not loosen the mode to make a port pass; if a mismatch cannot
be explained, report it.

Use time-limited runs only for the speed comparison, after the port has passed.

## 7. Report

- The crate path and its files.
- The mapping decisions: the modeling chosen for each Python problem, neighborhoods that
  differ, code left in Python or dropped.
- The compare output (mode, PASS/FAIL, objective table) and the speed ratio from it.
- How to run it: `cargo run --release` in the crate.

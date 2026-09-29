---
name: model-problem
description: Build an optimization problem interactively from a task described in words — interview the user for decisions, objective, constraints and data, choose between a built-in problem, Formula and a problem written in Python, write a runnable optopus-py script, and check it on a tiny instance against brute force. Use when the user wants to formulate or model something as an optimization problem, or to solve a task with optopus ("最適化問題を作りたい", "定式化して", "シフトを optopus で組みたい", "model this as an optimization problem"). For porting the resulting script to Rust, use the port-to-rust skill.
allowed-tools: Bash(uv run *) Read Write Edit AskUserQuestion
---

# Model a task as an optopus-py problem

The user brings a task in words; the result is a script that solves it and a formulation they
have confirmed. Talk with the user in their language; the script's identifiers and comments
default to English unless they ask otherwise.

The API is the binding's and the docs', not this file's: read it there each time.

| To find | Read |
|---|---|
| which representation fits, and each built-in problem's objective, solution encoding, neighborhoods, crossovers and problem-specific heuristics | `docs/source/problems.md` |
| which constraints a built-in problem penalizes itself, and how to check them | `docs/source/problems.md`, "Constraints a problem handles itself" |
| what `run` reports per problem (penalties included, direction, `Formula`'s sign) | `docs/source/problems.md`, "What a run reports" |
| `Formula`'s polynomial, constraint and bounds format, how it penalizes a violation, and choosing a weight | `docs/source/formula.md` |
| the Python problem protocol, and what each heuristic needs from it | `docs/source/python_problems.md` |
| which heuristic to propose and how to set its main parameters | `docs/source/heuristics.md` |
| every constructor argument and default | the `#[pyclass]` / `#[pymethods]` docstrings in `src/*.rs`, which the API reference renders |

Read only the parts the task needs.

## 1. Interview

Ask one topic per round, with the agent's question tool if it has one (AskUserQuestion in Claude
Code), and offer concrete options built from what the user already said rather than open
questions. Skip anything they have answered.

- **Decisions**: what is chosen — yes/no per item, a value per item, an order, a grouping.
- **Objective**: what is measured and whether more or less is better. Several goals: ask for
  weights or a strict priority.
- **Constraints**: list each one and ask whether it is hard (a solution breaking it is useless)
  or soft (allowed at a cost, and what cost).
- **Data**: the realistic size, and the form of every input — columns of each file, and inputs
  given separately (a depot, a budget, pairs that conflict).
- **Budget**: how long the user will wait for one solve.

Then check that the realistic data can satisfy the hard constraints at all (total demand
against total capacity, enough staff for every slot). If it cannot, ask what gives before going
on.

Restate the problem twice, in plain words and as math in the terms the user decides in (for
example "who works night n", not one-hot variables), and get it confirmed. The encoding comes in
step 2.

## 2. Choose the representation

Take the first rung that fits the confirmed formulation exactly:

1. **A built-in problem**, when the task is that problem and nothing more: capacitated routing
   → `Vrp`, one tour → `Tsp`, operations on machines → `JobShopScheduling`, a bipartition →
   `MaxCut`, conflict-free labels → `GraphColoring`, a binary quadratic → `Qubo`, clauses →
   `Sat`, covering edges → `VertexCover`. Compare its objective and constraints with the
   confirmed ones; a side constraint it lacks moves the task down a rung. `Qubo` has no
   constraints, so a linear inequality such as a budget means `Formula`.
2. **`Formula`**, when every variable is a bounded integer and the objective and constraints
   are polynomials in them. Fix what the user rules out through bounds, e.g. `(0, 0)`, rather
   than a penalty.
3. **A problem written in Python** otherwise: non-polynomial objectives, structured solutions,
   side constraints on routes or orders. Give each neighborhood `delta` and `random_neighbor`
   (the speed section of `python_problems.md` says why), and `tabu_keys` if `TabuSearch` may be
   used. Encode solutions as tuples so `apply` cannot mutate its input.

Tell the user which rung, why the higher ones do not fit, the encoding, and the cost: a
built-in problem is fastest, a Python problem slowest.

Hard constraints the representation does not enforce become penalties; a built-in problem that
penalizes a constraint itself (its docstring says so) needs nothing added. `Formula` charges
weight × violation, so make the weight larger than the most one move can gain on the objective
divided by the smallest violation a move can cause, and write that reasoning as a comment next
to the weight. Soft constraints use the cost the user gave. An `Eq` constraint such as "exactly
one per night" makes every move between feasible solutions pass through an infeasible one:
prefer `TabuSearch` there, or a Python problem that keeps it by construction.

## 3. Choose the heuristic

Propose a default and one alternative with their parameters (and `neighbor`, for the heuristics
that take one) and say why. Prefer a problem-specific heuristic when the built-in problem has
one (`docs/source/heuristics.md` lists them). Let the user pick.

`runs` execute one after another, so the user waits about `runs × max_duration_secs`: split the
budget. A `TabuSearch` tenure of 1 only forbids undoing the last move and cycles; start from a
few moves and grow it with the instance. Use a fixed `seed`, so a rerun reproduces the result.

## 4. Write the script

Ask where to save it; not under `src/`, `tests/` or `docs/`. Lay it out as:

- a loader for the real data, and a tiny instance behind a `--demo` flag;
- building the problem;
- the run;
- picking the best run from `report.runs` (the report holds no best solution). Check which way
  `best_objective` points for this problem first: for `Formula` it is direction-corrected,
  higher is better, so a minimized objective comes out negated;
- decoding the best solution into the user's terms (names, not indices);
- printing every run's best, the objective, and a check of every hard constraint as the user
  stated it, computed from the decoded solution, not from the penalty.

## 5. Verify

```bash
uv run maturin develop                          # rebuild only if src/ or vendor/ changed
uv run --no-sync python <script> --demo
```

On the tiny instance:

- Write a brute force in a temporary directory, not in the user's code, over the user's
  decision space (who works each night, which items are chosen, how customers split into
  routes), not over the encoding, and find the optimum from the objective the user stated.
  Where the problem has its own evaluator (`evaluate_routes`, `eval_objective` and
  `eval_penalty`, a Python problem's `objective`), check it agrees on every candidate. Search
  the whole encoded space for an infeasible solution that scores better only when that space is
  small.
- Every run should reach that optimum and pass every hard-constraint check. If only some do,
  find out why before handing the script over.
- Show the user the decoded demo result and ask whether it is a sensible answer to their task.
  A no usually means a constraint was never stated: go back to step 1, not only to the code.

Then run once at the realistic size, on synthetic data if the real data is not at hand, and
check it finishes within the budget with every hard-constraint check passing. Compare with a
bound when one is cheap (a knapsack DP, an ideal even split).

When a check fails, find the cause before touching a penalty weight, a parameter or the stop
condition: a check that passes only after blind tuning proves nothing. Report what you could
not explain.

## 6. Report

- The script path and how to run it on the real data.
- The confirmed formulation, the rung, encoding and heuristic chosen and why, and every penalty
  or approximation with its weight.
- The verification output: the brute-force optimum next to every run's result, and the
  realistic-size run's time and checks.
- That the `port-to-rust` skill can port the script when it needs to run faster.

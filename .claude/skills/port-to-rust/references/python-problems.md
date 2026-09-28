# Porting a problem written in Python

A Python problem follows the protocol in `docs/source/python_problems.md`. The binding wraps it
in `PyProblem` / `PySolution` / `PyMove<K>` (`src/python_problem.rs`), and those impls are the
reference for what each Python member means to the optopus traits. The port replaces that
wrapper with types of its own.

This file holds only the judgment calls. For the APIs themselves, read the sources, which stay
current:

- upstream `docs/guide/custom_problem.md` (skeleton, which heuristic needs which trait),
  `docs/traits.md` (signatures) and `examples/custom_problem.rs` (a complete problem),
- upstream `docs/problems/integer.md` and `docs/problems/formula.md` for the two ready-made
  problem types,
- `src/python_problem.rs` for how the binding answered each question below.

## 1. Pick the modeling

Go down the list and take the first that fits. Each step down is more code, and more that can
go wrong.

1. **A built-in problem in disguise** (a Max Cut, TSP, QUBO… written by hand): use the built-in
   type and its moves. Tell the user, since it replaces their neighborhoods with optopus's.
2. **Integer variables in fixed ranges, or one permutation, with the objective a function of
   the values**: `IntegerProblem`. Port a Python `delta` to its delta builders; without them
   every move re-evaluates the whole objective.
3. **Objective and constraints that are arithmetic over integer variables**: `FormulaProblem`,
   which derives every delta itself.
4. **Anything else** (a structured solution, moves the integer ones cannot express, a delta that
   needs a cache in the solution): implement the traits. If only the delta needs a cache,
   `IntAssignment` on your own solution keeps the ready-made integer moves.

When a ready-made move does not match the Python neighborhood exactly (Python's 2-opt may keep
city 0 fixed, the integer reversal does not), the search differs slightly. Say so in the report;
`stat` mode compares quality, not trajectories, so it still verifies the port.

## 2. Writing the traits

Map each Python member to the trait method `PyMove` / `PyProblem` implements for it, then write
it natively. What the Python protocol leaves implicit and the port has to decide:

- **Direction** lives in the `Evaluable::Minimize` / `Maximize` both `Evaluate` impls (solution
  and move) wrap their value in. Python's `minimize` has no other home.
- **The objective is stored in the solution** and updated by `apply_to_solution` with the move's
  delta, so `evaluate` is O(1). Python recomputed it; Rust must not.
- **A neighborhood without `delta`**: the binding priced moves by applying and re-scoring. Do
  that only if the Python one really had no cheaper form.
- **`tabu_keys`** becomes an `EnabledTabu` impl, and the `MoveToNeighbor` impl also needs
  `fn tabu_policy(&self) -> Option<&dyn EnabledTabu> { Some(self) }`. Without that line the
  move compiles and silently runs with no tabu list.
- **No `distance`**: the binding used 0 for equal solutions and 1 otherwise. Keep that or write
  a real one.
- **`partial_objective`** (ruin methods) becomes `partial_energy`, which returns the value with
  the direction applied, as `PyProblem` does.
- Moves used by `TabuSearch` or `ReinforcementLearningSearch` need `Clone`, and `iter` must
  return a `Send` iterator (collect into a `Vec` if needed).

Implement only the traits the ported heuristics need.

## 3. Randomness

Draw only from the `rng` the traits hand you, never from `rand::rng()` or a thread RNG, or a
seeded run no longer reproduces. Python's `random.Random` and Rust's `SmallRng` draw different
numbers, so a Python problem is verified with `compare.py --mode stat` plus `check_objective`,
never `--mode exact`.

## 4. Things that go wrong

- **Direction**: an `Evaluate` impl that wraps in the wrong variant runs the search backwards
  and still compiles. `check_objective` passes, `stat` fails badly.
- **Stale objective**: `apply_to_solution` forgot to update the stored objective.
  `check_objective` catches it.
- **Delta sign**: a Python `delta` is objective after minus before, in the plain direction; so
  is the Rust one, and the `Evaluable` wrapper applies the direction.
- **Integer width**: Python ints are unbounded; check the instance fits the Rust type chosen.
- **Aliasing in ruin and recreate**: `to_partial` must still copy.

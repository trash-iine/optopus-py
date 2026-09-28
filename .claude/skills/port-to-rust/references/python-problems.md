# Porting a problem written in Python

A Python problem follows the protocol in `docs/source/python_problems.md`; the binding wraps it
in `PyProblem` / `PySolution` / `PyMove<K>` in `src/python_problem.rs`, which is the reference
for what each Python member means to the Rust traits. The port replaces that wrapper with
types of its own.

## 1. Pick the modeling

Go down the list and take the first that fits. Each step down is more code, and more that can
go wrong.

1. **The Python is a built-in problem in disguise** (a Max Cut, TSP, QUBO… written by hand):
   use the built-in type and its moves (`api-mapping.md`). Say so to the user, since it changes
   the neighborhood from theirs to optopus's.
2. **Variables are integers in fixed ranges, or one permutation, and the objective is a
   function of the values**: `IntegerProblem` (upstream `docs/problems/integer.md`).
   - `IntVars::new(vec![IntVar::new(lo, hi); n])`, or `IntVars::permutation(n)` for a tour.
   - `IntegerProblem::minimize(vars, |x: &[i64]| …)` / `::maximize`.
   - Python neighborhoods map to `IntChangeNeighbor` (set one variable), `IntSwapNeighbor`
     (exchange two), `IntReverseNeighbor` (reverse `i..=j`, 2-opt). A Python `delta` becomes
     `.with_delta(|sol, i, value| …)`, `.with_swap_delta(|sol, i, j| …)` or
     `.with_reverse_delta(|sol, i, j| …)`, the change of the plain objective, new minus old.
     Without them every move re-evaluates the whole objective (and warns once).
   - GA: `IntCrossover`; `IntSolution` already has `Distance`.
   - Solutions are `IntSolution`; `obj` is `s.evaluate().minimized()` for minimize and
     `0.0 - s.evaluate().minimized()` for maximize, `encode` is `json!(s.values())`.
3. **The objective and constraints are arithmetic over integer variables**: `FormulaProblem`
   (upstream `docs/problems/formula.md`), which derives every delta itself.
4. **Anything else** (a solution with structure, moves the three above cannot express, a delta
   that needs a cache kept in the solution): implement the traits, section 2. If only the delta
   needs a cache, implement `IntAssignment` on your own solution instead and keep the three
   integer moves (upstream `docs/problems/integer.md`, "A solution of your own").

When the Python neighborhood does not map exactly onto an integer move (e.g. Python's 2-opt
keeps city 0 fixed, `IntReverseNeighbor` does not), the search differs slightly. Say so in the
report; it is usually fine because `stat` mode compares quality, not trajectories.

## 2. Protocol → traits

Upstream `docs/guide/custom_problem.md` has the skeleton and `examples/custom_problem.rs` a
complete example; `docs/traits.md` lists the full signatures.

| Python member | Rust |
|---|---|
| problem class | `struct MyProblem`, holding the instance data |
| `minimize` | the `Evaluable::Minimize` / `Evaluable::Maximize` both `Evaluate` impls below wrap their value in |
| `new_solution(rng)` | `impl ProblemTrait for MyProblem { type Solution = MySolution; fn new_solution(&self, rng: &mut impl rand::Rng) -> MySolution }` |
| solution object | `#[derive(Clone)] struct MySolution`, carrying its objective so `evaluate` is O(1) |
| `objective(solution)` | a function computing it, called in `new_solution` and kept up to date by `apply_to_solution`; `impl Evaluate for MySolution { fn evaluate(&self) -> Evaluable<f64> }` returns the stored value |
| one entry of `neighborhoods` | one move type `struct MyMove { …coordinates…, delta: f64 }` with `impl MoveToNeighbor<MyProblem>` |
| `neighbors(problem, solution)` | `fn iter(prob, sol) -> impl Iterator<Item = Self> + Send`, each move priced as it is built |
| `apply(problem, solution, move)` | `fn apply_to_solution(&self, prob, sol: &mut MySolution) -> Result<(), OptError>`, in place (Rust clones for you where the search keeps copies), updating the stored objective by `delta` |
| `delta(problem, solution, move)` | the `delta` field computed in `iter`; `impl Evaluate for MyMove` returns it wrapped in the direction |
| (no `delta`) | compute it by applying to a clone and differencing, as the binding does. Slow; write a real delta when the Python one was omitted only for brevity |
| `random_neighbor(problem, solution, rng)` | `fn random_neighbor(prob, sol, rng: &mut SmallRng) -> Option<Self>`; the default samples `iter`, fine only for small neighborhoods |
| — | `fn move_to_be_better_than(&self, _prob, src, other) -> bool { self.evaluate().improves_over(src.evaluate(), other.evaluate()) }`, what `PyMove` does |
| `tabu_keys(problem, move)` | `impl EnabledTabu for MyMove` over `optopus::common::{TabuKey, TabuMemory}`: `is_move_enabled` is `keys.iter().all(\|&k\| tabu.is_enabled(k, iteration))`, `add_to_tabu_map` calls `tabu.forbid(k, iteration, rng)` per key; an int is `TabuKey::Var(i)`, a pair `TabuKey::Pair(i, j)`, a triple `TabuKey::Triple(i, j, k)`. Also add `fn tabu_policy(&self) -> Option<&dyn EnabledTabu> { Some(self) }` to the `MoveToNeighbor` impl: without it the move compiles and silently has no tabu list |
| `crossover(a, b, rng)` | `struct MyCrossover; impl Crossover<MyProblem> for MyCrossover { fn crossover(&mut self, prob, a, b, rng: &mut SmallRng) -> Result<MySolution, OptError> }` |
| `distance(a, b)` | `impl Distance for MySolution { fn distance(&self, other: &Self) -> usize }`. Without a Python `distance`, the binding used `0` if equal else `1`; do the same, or write a real one |
| ruin methods (`to_partial`, `finish`, `elements`, …) | `impl Ruinable for MyProblem` (upstream `src/trait_defs/ruinable.rs`); each Python method has the Rust method of the same name (`elements` fills an out `Vec`). `partial_objective` becomes `partial_energy`, which returns the objective with the direction applied, `Evaluable::…(v).minimized()` |
| `repair_around(partial, anchors, rng)` | `impl LocalRepair<MyProblem> for MyRepair`, passed with `.with_local_repair(Box::new(MyRepair))` |

Which heuristic needs which of these: upstream `docs/guide/custom_problem.md`, "Which heuristic
needs what". Implement only what the ported code runs.

Bounds the traits carry that Python did not: moves used by `TabuSearch` and
`ReinforcementLearningSearch` need `Clone`; `iter` must return a `Send` iterator, so collect into
a `Vec` first if the lazy form borrows something that is not `Sync`.

## 3. Randomness

The Python `rng` is a `random.Random`; Rust's is a `SmallRng` seeded by `SearchState`. Use
`rand` 0.9 methods on it (`rng.random_range(a..b)`, `rng.random_bool(p)`,
`slice.shuffle(rng)` with `rand::seq::SliceRandom`). Never reach for `rand::rng()` or a
thread RNG: a seeded run must reproduce.

Python and Rust draw different numbers, so the port cannot reproduce the Python trajectory. That
is why a ported Python problem is verified with `compare.py --mode stat` plus
`check_objective`, never `--mode exact`.

## 4. Things that go wrong

- **Direction**: an `Evaluate` impl that wraps in the wrong variant makes the search run the
  other way and still compile. `check_objective` passes, `stat` fails badly.
- **Stale objective**: `apply_to_solution` must update the stored objective. `check_objective`
  catches this.
- **Delta sign**: Python `delta` is objective after minus before in the plain direction, and so
  is the Rust one; the `Evaluable` wrapper applies the direction.
- **Integer vs float**: Python ints are unbounded; pick `i64` / `usize` and check the instance
  sizes fit.
- **Mutable aliasing**: Python solutions were immutable tuples; a Rust solution is cloned by
  value, so no care is needed there, but a `Partial` in ruin-and-recreate must still be a copy.

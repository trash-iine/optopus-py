# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

Tracks optopus up to `8de6180`, which reworked large parts of the library's public API. Several
entries below are breaking; see **Changed** and **Removed**.

### Added

- `GeneticAlgorithm(crossover="SubProblem", sub_heuristic=...)` runs on a problem written in
  Python that defines `extract_sub_problem(a, b)`, a Python problem over what the parents
  disagree on, and `lift_solution(a, b, sub_solution)`. The sub-problem declares the
  problem's neighborhoods, and an exception in it is raised from `run` (ADR 0011).
- `Formula(..., permutation=True)` makes the variables a permutation of `0..n_vars`, so an
  ordering such as a linear arrangement is a `Formula` without "all different" constraints.
  The search keeps every solution a permutation through the `"Swap"` and `"Reverse"` moves and
  the order crossover; `"Change"`, `BranchAndBound` and the `"SubProblem"` crossover raise
  `ValueError` there. `Formula.is_permutation()` reads it back.
- `RunResult.betas` and `exchange_acceptance` report a `ParallelTempering` run's temperature
  ladder and the share of replica exchanges it accepted, for tuning the ladder, and
  `RunResult.policy_weights` the weights a `ReinforcementLearningSearch` run ended with, which
  `policy_weights=` takes back to continue from what it learned. All three are `None` for other
  heuristics.
- `AdaptiveLargeNeighborhoodSearch` takes `scores`, `segment_length`, `reaction` and
  `max_removal`, the operator rewards and weight adaptation of Ropke and Pisinger and the ceiling
  on one iteration's removals, and `local_repair=False` to run without the descent after each
  repair. The defaults are optopus's, so runs that set none of them do not change.
- `IteratedGreedy` runs on `Tsp`, `Vrp`, `BinPacking` and Python problems with the ruin
  methods, not only `FlowShop`. It takes `temperature=` (required off the flow shop) and
  `local_repair=`.
- `HybridGeneticSearch` takes `n_elite`, `n_closest`, `penalty_adaptation`,
  `repair_probability`, `initial_population` and `descent_passes`.
- `EigenvalueRelaxation(max_iterations=, lanczos_steps=, certify_limit=)` sets the limits of
  `BranchAndBound`'s eigenvalue bound on `MaxCut`; `relaxation="Eigenvalue"` is it with the
  defaults.
- `StopCondition`'s `max_iteration`, `max_duration_secs` and `max_failed_update` can be read
  back.
- Scoring without a run: `MaxCut.cut_size`, `Qubo.energy`, `Sat.num_satisfied`,
  `VertexCover.evaluate_cover`, `Tsp.tour_length` and `JobShopScheduling.makespan` /
  `completion_times`. Each takes a solution in the form `RunResult.solution` has, through the
  same checks as `initial_solution`.
- Reading an instance back: `Sat.num_vars` / `num_clauses`, `VertexCover.penalty_weight`,
  `Tsp.distance`, `coordinates` and `edge_weight_type`, `JobShopScheduling.num_jobs` /
  `num_machines`, and `FlowShop.processing_time`.
- `BinPacking.first_fit(order=None)` and `Vrp.split_giant_tour(giant)`, the constructions
  optopus's heuristics start from, as solutions to score or to start a run from.
- `Tsp.from_coordinates` takes `edge_weight_type=` (`"EUC_2D"`, `"CEIL_2D"`, `"ATT"`,
  `"GEO"`, so lengths match TSPLIB's optima) and `nearest_neighbors=`, and
  `Vrp.from_coordinates` takes `nearest_neighbors=`: only each node's `k` nearest distances are
  kept, with the same distances in less memory.
- `load_file` for `MaxCut`, `VertexCover` and `GraphColoring` (G-set style edge lists), `Qubo`
  (sparse matrix files), `Sat` (DIMACS CNF) and `JobShopScheduling` (Taillard / OR-Library),
  so every built-in problem reads an instance file. `MaxCut`, `VertexCover` and `GraphColoring`
  return their graph with `graph()`.
- `Graph.load_file` and `write_to_file`, which round-trip a graph exactly, and the queries
  `degree`, `neighbors`, `has_edge`, `weight` and `induced_subgraph`.
- `initial_solution=` on every heuristic's `run` starts each run from a solution you give
  instead of a random one, in the form `RunResult.solution` has for the problem, so the best
  solution of one call continues in the next. It works on every built-in problem, `Formula`
  and problems written in Python, and a solution that does not fit the problem raises
  `ValueError`. `MaxCutKernel.project` now has a use: a solution of the original starts a
  search on the kernel.
- `FlowShop`, the permutation flow shop (minimize the makespan), with the `Relocate` (insertion)
  and `Swap` neighborhoods and the `Order` crossover. Build it from processing times or read a
  Taillard-layout file with `load_file`; `makespan(sequence)` scores an order and `neh()`
  returns the NEH construction's.
- `Qap`, the quadratic assignment problem, with the `Swap` neighborhood, under which
  `TabuSearch` applies Taillard's tabu rule, and the `Order` crossover. Build it from the flow
  and distance matrices or read a QAPLIB file; `cost(assignment)` scores an assignment.
- `BinPacking`, one-dimensional bin packing, with the `Relocate` and `Swap` neighborhoods and
  the `Group` crossover. Build it from the capacity and item sizes or read an instance file;
  `lower_bound()` and `evaluate_bins(bins)` read it back. The search ranks a packing by
  `num_bins − mean((load / capacity)²)`, so fuller bins break ties.
- `IteratedGreedy`, iterated greedy for `FlowShop`: a few jobs out at random, each back where
  the makespan grows least, an insertion descent, and acceptance at a constant temperature.
- `GreedyRandomizedConstruction`, GRASP's construction step, which builds one solution from
  nothing by the randomized greedy rule. GRASP is `Iterated` with it as the perturbation. It
  runs on `Tsp`, `Vrp`, `FlowShop`, `BinPacking` and Python problems with the ruin methods.
- `AdaptiveLargeNeighborhoodSearch` runs on `FlowShop`, with an insertion descent after each
  repair, and on `BinPacking`.
- `ParallelTempering`, replica exchange Monte Carlo: `num_replicas` replicas on a geometric
  ladder of inverse temperatures from `beta_min` to `beta_max` run Metropolis sweeps, and
  neighbouring replicas trade places. A generic heuristic, so it takes a `neighbor` and runs on
  every problem, Python problems included.
- `BranchAndBound`, which proves the best solution optimal on `MaxCut`, `Qubo`, `Sat`,
  `VertexCover` and `Formula`. Another heuristic searches each node, the problem's own included
  (`WalkSat`, `BreakoutLocalSearch`), and a relaxation prunes it: `"Binary"` on the binary
  problems, `"Eigenvalue"` (the certified eigenvalue bound) on `MaxCut`, `"Interval"` on
  `Formula`, or a Python callable taking the node's ranges. `RunResult` gains
  `proven_optimal` and `dual_bound`, set on a `BranchAndBound` run and `None` otherwise.
- `Vrp`, the capacitated vehicle routing problem, with the `Relocate`, `Swap` and `TwoOpt`
  neighborhoods. Build it from coordinates, an explicit distance matrix, or a CVRPLIB file;
  `num_vehicles=0` lets optopus pick a fleet size. `evaluate_routes` scores a route partition
  and reports the raw distance and the capacity overload separately, since the objective a
  heuristic minimizes is `distance + penalty_weight * overload`.
- A mixed fleet for `Vrp`: `VehicleType` describes a vehicle type (capacity, speed, fixed and
  per-distance cost, minimum and maximum count, route-time limit), and `Vrp.with_fleet` builds
  an instance from a list of them, with service times at the customers, a `"TotalTime"` or
  `"Makespan"` objective and a `cost_weight`; `Vrp.load_file` reads the same from optopus's
  TOML format. Capacity, route-time limit and minimum count are penalized like the capacity of
  a CVRP. `vehicle_types()`, `slot_types()` (the vehicle type of each route), `service_times()`,
  `objective_mode()`, `cost_weight()` and `with_objective_mode()` read the fleet back, and
  `evaluate_routes` reports per-route distances and times, vehicles used per type, total time,
  makespan, route-time excess, minimum-count shortfall and cost. Every `Vrp` heuristic and
  neighborhood runs on a mixed fleet.
- `GraphColoring`, minimum proper coloring, with the `Recolor` and `Swap` neighborhoods and an
  `evaluate_colors` that reports colors used and conflicts. The palette defaults to
  `max_degree + 1` and can be set explicitly.
- Problem-specific heuristics: `WalkSat` (SAT), `BreakoutLocalSearch` (Max Cut),
  `LinKernighanHelsgaun` (TSP), `AdaptiveLargeNeighborhoodSearch` (VRP and TSP) and
  `HybridGeneticSearch` (VRP). These take no `neighbor` argument and raise `ValueError` when run
  on a problem they do not apply to.
- Problems written in Python. Any object with `minimize`, `new_solution`, `objective` and a
  `neighborhoods` mapping can be passed to the generic heuristics, `PopulationAnnealing` and
  `VariableNeighborhoodSearch`. Exceptions raised in its methods, and Ctrl-C, stop the run and
  propagate from `run`.
- `VariableNeighborhoodSearch`, which alternates an intensifying search heuristic with a list of
  increasingly disruptive shakes. Steps are ordinary heuristic instances, so each may use its own
  neighborhood and stopping criterion.
- Composed heuristics built from other heuristic instances: `Sequential`, `Iterated`, `Restart`,
  `GeneticAlgorithm` (with each problem's crossover, including `"SubProblem"` over any
  `sub_heuristic`, and `Tournament`, `DistantTopK` or `BiasedFitness` parent selection), and
  `BreakoutLocalSearch.from_parts`, which assembles breakout local search from a descent and
  perturbations for any problem.
- `ReinforcementLearningSearch`, a generic heuristic with a softmax policy over move features.
- Python problems run under the composed heuristics too. `GeneticAlgorithm` uses their `crossover`
  method, solutions are compared with their optional `distance` method, and a problem with the
  ruin methods runs under `AdaptiveLargeNeighborhoodSearch`.
- `PlantedMaxCut`, which generates Max Cut instances whose optimum is exact by construction
  (`tile_planting_2d`, `tile_planting_3d`, `wishart`), plus `verify` and `has_exact_optimum` —
  the latter says whether "did the run reach the optimum" is an equality test or needs a
  tolerance.
- `MaxCutKernel`, exact kernelization of a Max Cut instance. `kernel()` is an ordinary `MaxCut`,
  so any heuristic runs on it unchanged, and `lift` / `project` / `offset` carry solutions and
  objectives between the two. `is_trivial()` reports when the rules did not fire, which is the
  case on regular and dense graphs.
- `Graph`, with the Erdős-Rényi, Barabási-Albert and Watts-Strogatz random graph generators, the
  `grid_torus_2d` / `grid_torus_3d` periodic lattices and `with_random_weights`, plus
  `MaxCut.from_graph`, `VertexCover.from_graph` and `GraphColoring.from_graph`.
- `Tsp.from_distance_matrix` and `Tsp.load_file`, so a tour no longer has to come from 2D
  coordinates.
- `PopulationAnnealing` accepts `sweep_length`, pinning the proposals per sweep. Counting the
  neighborhood is O(n) for a single-variable move but O(n²) for a pairwise one such as
  `"TwoOpt"`, where pinning a length is worth it.
- `Formula` accepts a `bounds` argument giving each variable an integer range, and the `Reverse`
  neighborhood alongside `Change` and `Swap`. `eval_objective` and `eval_penalty` score an
  assignment without running a search.
- `RunResult.iterations` and `RunReport.avg_iterations`, the iterations a run made in the
  heuristic's own unit. It counts what `n_accepted + n_rejected` misses, such as
  `HybridGeneticSearch`'s initial population.
- `optopus.__version__`, the package version as a string.
- `Vrp.penalty_weight()`, the weight the objective charges per unit of overload, matching
  `GraphColoring.penalty_weight()`.
- The Sphinx documentation is now published to GitHub Pages at
  <https://trash-iine.github.io/optopus-py/>, rebuilt from `main` on every push.
- Three guide pages in the documentation. "Built-in problems" compares the problems, the
  constraints each penalizes itself and what a run reports for each, including that
  `best_objective` carries the penalty for `VertexCover`, `Vrp`, `GraphColoring` and `Formula`
  and that a minimizing `Formula` reports it negated. "Modeling with Formula" covers the
  polynomial and constraint format, how a violation is penalized and how to choose a penalty
  weight. "Choosing a heuristic" covers which heuristic to use, what one iteration costs and how
  to set the main parameters.
- More guidance from a second simulated first use: a stop condition is checked between units of
  work, so `HybridGeneticSearch` always builds its initial population; measuring deltas to set
  a `SimulatedAnnealing` temperature, including on a secondary term and under `"Eq"`
  constraints; `AdaptiveLargeNeighborhoodSearch`'s removal cap, per-iteration cost and cooling
  for a time budget; a `Vrp` variable neighborhood search; a one-hot assignment with
  `Formula`; the `tabu_keys` contract (`(i, j)` is one key, `[i, j]` two) and the ruin methods'
  container numbering.
- Documentation meant to be enough without reading the code: the top page shows how to install
  and the three steps of every use; "Running a search" covers `run`, seeds, `StopCondition` and
  every `RunReport` and `RunResult` field; four pages give each heuristic its arguments with
  their defaults and a runnable example (generic, composed, problem-specific, exact search);
  "Built-in problems" builds and solves a small instance of every problem with its
  constructors and evaluators; "Graphs and test instances" covers `Graph`, `PlantedMaxCut` and
  `MaxCutKernel`.
- A Japanese edition of the documentation at
  [trash-iine.github.io/optopus-py/ja/](https://trash-iine.github.io/optopus-py/ja/), linked
  from every page's sidebar. It translates every page; the API reference lists the same
  classes with their English docstrings.
- `RunResult.trajectory`, every improvement of the run's best solution as
  `(elapsed_secs, iteration, objective)`, for plotting how a run converged.
- `on_improvement`, an optional argument of every heuristic's `run`: a callable that receives
  `(run, elapsed_secs, objective)` while a run improves, at once for the first improvement and
  then at most every 0.1 seconds.

### Changed

- `Tsp.from_coordinates` raises `ValueError` for a non-finite coordinate, as
  `Vrp.from_coordinates` already did, instead of building an instance with NaN distances.
- **`load_file` raises `OSError` for a file it cannot open**, `FileNotFoundError` when the file
  does not exist, with the path in `filename`, as Python's own `open` does. It used to raise
  `ValueError`. Catch `OSError` (or `FileNotFoundError`) where you caught `ValueError` for a missing
  file; a file that does not parse still raises `ValueError`.
- `Vrp.evaluate_routes` raises `ValueError` unless it gets one route per vehicle, empty routes
  included, instead of scoring a plan the fleet cannot run. Pad a plan with `[]` up to
  `num_vehicles()`. This follows optopus, now vendored at `41ef2c4`, whose `Vrp` also covers a
  mixed fleet (see **Added**).
- `Vrp.load_file` reads a `.toml` file as a mixed fleet, as optopus does; any other file is
  still read as CVRPLIB. `Vrp.capacity()` raises `ValueError` on a fleet of several vehicle
  types, which have no single capacity; read `vehicle_types()` instead.
- Wheels are built with the release settings optopus measured, fat LTO and a single codegen
  unit, which the binding had never applied. `TabuSearch` runs about 1.9x faster on a
  2000-vertex MaxCut; the other heuristics measured gain little.
- Updated the vendored optopus to `8de6180`, then to `594f473`, and later to `1314a94`.
- `BinPacking`'s `"Relocate"` neighborhood also moves an item that shares its bin into a new bin
  of its own, so a tight packing still has a move. `SimulatedAnnealing` and
  `LateAcceptanceHillClimbing` used to stop early once the packing got that tight. Runs on
  `BinPacking` give different results for the same seed. Changed in optopus (`313e1e1`).
- `AdaptiveLargeNeighborhoodSearch`'s greedy repair puts back the element whose cheapest place
  costs least first, re-pricing the rest after each insertion, as the original paper's basic
  greedy does; it used to insert them in a random order. Runs on `Vrp`, `Tsp` and Python
  problems give different results for the same seed, and an iteration calls
  `insertion_cost` about `k² · P / 2` times rather than `k · P`. Changed in optopus
  (`6c63f1a`).
- **`PopulationAnnealing` is now a generic heuristic**: optopus generalized it off Max Cut, so it
  runs on every problem type and takes a `neighbor` argument. It defaults to `"Flip"` and comes
  after the existing parameters, so calls that did not name it keep working. Its `cluster_moves`
  argument is gone, the operator having been dropped upstream.
- **`BreakoutLocalSearch` lost its `plateau_prob` argument**, which upstream removed.
- **`BreakoutLocalSearch`'s `tabu_tenure` now means the same prohibition length it does under
  `TabuSearch`.** Benlic and Hao's γ is counted twice inside the algorithm, so reproducing their
  `rand[3, |V|/10]` means passing `(6, |V| // 5)`.
- **`Formula` variables are integers, not booleans.** A solution is reported as a `list[int]`
  rather than a `list[bool]`; for binary variables the entries are 0 and 1, which compare equal
  to `False` and `True`. Variables are still binary unless `bounds` says otherwise.
- **`Formula`'s `Flip` neighborhood is now named `Change`**, matching optopus's integer moves.
  `Flip` remains accepted, because on a binary variable a change *is* a flip.
- `AdaptiveLargeNeighborhoodSearch` now also runs on `Tsp`, `FlowShop` and `BinPacking`, so its
  restriction message reads "only available for Vrp, Tsp, FlowShop, BinPacking and Python
  problems with the ruin methods".
- `pyo3/extension-module` is now an opt-in `extension-module` cargo feature rather than being
  always on, so plain `cargo build` and `cargo test` link against libpython and succeed on macOS.
  maturin still enables it for wheels via `features` in `pyproject.toml`, so built wheels are
  unchanged.
- Disabled the doctest pass (`doctest = false`); the `///` comments are Python docstrings rather
  than Rust doc examples, and rustdoc otherwise fails on the lib sharing its name with the
  `optopus` dependency.

- Inputs the search would price wrongly now raise `ValueError` instead of running: an
  asymmetric, negative or non-finite distance matrix in `Tsp.from_distance_matrix` and
  `Vrp.from_distance_matrix`, a negative `Vrp` demand, and a negative or non-finite
  `Formula` `penalty_weight` or a `Clamp` range with `lo` above `hi`. Symmetrize an
  asymmetric matrix, for example with `(d[i][j] + d[j][i]) / 2`, and state a reward as a
  positive weight on the opposite constraint instead of a negative weight.
- `run` on a built-in problem releases the GIL while it searches, so other Python threads keep
  running, and several runs started from a thread pool proceed in parallel. A problem written
  in Python still holds it, since every step calls back into Python.

### Removed

- **`TspWithCoordinates`, renamed to `Tsp`.** optopus renamed the problem when it gained
  non-Euclidean distances, and the binding follows without keeping an alias. Rename the call;
  nothing else about the class changed.
- **`RlBreakoutLocalSearch`.** Upstream moved the contextual-bandit perturbation controller out of
  the library and into `examples/rl_bls.rs`, so there is no longer a type to bind. The generic
  `BreakoutLocalSearch` covers the same algorithm with a fixed schedule.

### Fixed

- A run that a heuristic could not continue ended silently: `run` returned the numbers of the
  search so far as if it had finished, and the later runs went ahead. It now raises
  `RuntimeError` with optopus's message, such as `SimulatedAnnealing` or
  `LateAcceptanceHillClimbing` on a solution whose neighborhood is empty, and no later run starts.
- The docs said one `PopulationAnnealing` iteration is a whole temperature step. It is one
  sweep of every replica, so a step counts `sweeps_per_step` iterations and a short run with the
  defaults stays hot.
- `BreakoutLocalSearch`'s `q` was documented as a decay rate. It is the share of directed
  perturbations that flip a vertex rather than swap two. `GeneticAlgorithm`'s docstring now
  lists the default crossover of every problem.
- The guide to problems written in Python said `PopulationAnnealing` ends the run when
  `random_neighbor` returns `None`. It skips that proposal and carries on, as
  `ParallelTempering` does.
- `Sat` counts a clause that holds a variable twice, repeated as in `[2, 2]` or negated as in
  `[1, -1]`, as one clause. Moves priced such a clause wrongly, so a run on one could report
  an objective other than the number of clauses its solution satisfies. Fixed in optopus
  (`a45ed19`).
- The signatures of `Tsp.from_coordinates`, `Tsp.from_distance_matrix`,
  `JobShopScheduling.from_jobs`, `Vrp.from_coordinates`, `Vrp.from_distance_matrix` and
  `PlantedMaxCut.wishart` show their string defaults (`name=''`, `name='vrp'`,
  `couplers='Gaussian'`) instead of `Ellipsis`.
- `Vrp.from_coordinates` raises `ValueError` for a non-finite coordinate instead of failing
  with a `PanicException` from the core.
- `Sat.from_clauses` raises `ValueError` for a literal of 0 or one naming a variable above
  `n_vars`, and `StopCondition` for a negative or non-finite `max_duration_secs`. Both used to
  fail with a `PanicException` from the core.
- `StopCondition`'s repr prints `None` and plain numbers instead of Rust's `Some(...)`.
- A large int returned by `tabu_keys`, such as `item * 10**15 + bin`, no longer aborts the
  Python process. Int keys used to index an array sized by the largest key; they are now kept
  in a map, through optopus's `TabuKey::Var`, with the same tabu behavior.
- `Formula` raises `ValueError` when a constraint reads a variable index outside `[0, n_vars)`,
  as it already did for the objective. It used to fail with a `PanicException` from the core.
- `help(optopus.Formula)` and the API reference now show the constructor's arguments, which
  were written where Python never reads them, and the signatures of `Formula`,
  `PopulationAnnealing` and `HybridGeneticSearch` show their real defaults instead of
  `Ellipsis`.
- The docstrings of `Iterated`, `Restart`, `Sequential` and `VariableNeighborhoodSearch`
  described their stop conditions and `Iterated`'s acceptance wrongly. The outer `stop` counts
  the steps' iterations and is checked between steps, `Restart`'s `restart` is read against the
  whole run, and `Iterated` does not undo a worse round.

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

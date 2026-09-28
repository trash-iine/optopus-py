# optopus-py → optopus API mapping

Read off the binding: `src/problem.rs` and `src/graph.rs` (problems), `src/runner.rs`
(`build_*`, `build_generic`, `build_nested`, `solve`, `run_all`), `src/heuristic.rs` (Python
signatures and defaults), `src/result.rs` (report). When those change, change this file in the
same PR. When this file and the binding disagree, the binding is right: read it and fix this file.

Everything named here comes from `use optopus::prelude::*;` unless a path is given.

## Problems

| Python | Rust |
|---|---|
| `MaxCut.from_edges(edges)` | `MaxCut::from_edges(edges)` — `edges: Vec<(usize, usize, f32)>` |
| `MaxCut.from_graph(g)` | `MaxCut::new(graph)` |
| `MaxCutKernel.reduce(mc)` | `optopus::problem::MaxCutKernel::new(&mc)`; `.kernel()`, `.offset()`, `.lift(&x)`, `.project(&x)` |
| `PlantedMaxCut.wishart(n, alpha, couplers, seed)` | `optopus::problem::PlantedMaxCut::wishart(n, alpha, WishartCouplers::…, &mut seeded_rng(seed))`; the instance is `.problem` |
| `PlantedMaxCut.tile_planting_2d(l, p1, p2, p3, seed)` | `PlantedMaxCut::tile_planting_2d(l, TileProbs2d::new(p1, p2, p3), &mut seeded_rng(seed))` |
| `PlantedMaxCut.tile_planting_3d(l, p_2fp, p_4fp, seed)` | `PlantedMaxCut::tile_planting_3d(l, TileProbs3d::new(p_2fp, p_4fp), &mut seeded_rng(seed))` |
| `Qubo.from_entries(entries)` | `Qubo::from_entries(entries)` — `(usize, usize, i32)` |
| `Sat.from_clauses(n_vars, clauses)` | `let mut s = Sat::new(n_vars); for c in clauses { s.add_clause(c); }` — DIMACS literals, `i64` |
| `VertexCover.from_edges(edges)` / `.from_graph(g)` | `VertexCover::new(Graph::from_edges(edges))` / `VertexCover::new(graph)` |
| `Tsp.from_coordinates(coords, name="")` | `Tsp::new(name, coords)` — note the argument order |
| `Tsp.from_distance_matrix(m, name="")` | `Tsp::from_distance_matrix(name, m)?` |
| `Tsp.load_file(path)` | `Tsp::load_file(path)?` |
| `JobShopScheduling.from_jobs(jobs, name="")` | `JobShopScheduling::new(name, n_machines, jobs)` with `n_machines` = largest machine index + 1 |
| `Vrp.from_coordinates(coords, demands, capacity, num_vehicles=0, name="vrp", rounded=False)` | `Vrp::new(name, coords, demands, capacity, num_vehicles)`, or `Vrp::with_rounding(...)` when `rounded=True` |
| `Vrp.from_distance_matrix(m, demands, capacity, num_vehicles=0, name="vrp")` | `Vrp::from_distance_matrix(name, m, demands, capacity, num_vehicles)?` |
| `Vrp.load_file(path)` | `Vrp::load_file(path)?` |
| `GraphColoring.from_edges(edges, num_colors=None)` / `.from_graph(g, …)` | `let mut gc = GraphColoring::new(graph); gc.k = num_colors;` (skip the assignment for `None`) |
| `Formula(n_vars, objective, direction, constraints, bounds)` | `FormulaProblem::maximize(vars, expr)` / `::minimize`, then `.with_constraint(c)` per constraint. See "Formula" below |

### Graph generators

A `seed=s` in Python is `seeded_rng(s)` in Rust. `seed=None` draws one from the clock, so
there is nothing to match against; pass a fixed seed on both sides while verifying.

| Python | Rust |
|---|---|
| `Graph.from_edges(edges)` | `Graph::from_edges(edges)` |
| `Graph.erdos_renyi(n, p, seed)` | `Graph::erdos_renyi(n, p, &mut seeded_rng(seed))` |
| `Graph.barabasi_albert(n, m, seed)` | `Graph::barabasi_albert(n, m, &mut seeded_rng(seed))` |
| `Graph.watts_strogatz(n, k, beta, seed)` | `Graph::watts_strogatz(n, k, beta, &mut seeded_rng(seed))` |
| `Graph.grid_torus_2d(l)` / `grid_torus_3d(l)` | `Graph::grid_torus_2d(l)` / `Graph::grid_torus_3d(l)` |
| `g.with_random_weights((lo, hi), seed)` | `graph.with_random_weights((lo, hi), &mut seeded_rng(seed))` (consumes `graph`) |

### Formula

Each Python monomial `(vars, coeff)` is `Expr::Mul(vec![Expr::Const(coeff), Expr::Var(v)…])`,
a bare constant is `Expr::Const(coeff)`, and the polynomial is `Expr::Add(terms)` (one term
stands alone, none is `Expr::Const(0.0)`) — `poly_to_expr` in `src/problem.rs`.

- Variables: `IntVars::new(vec![IntVar::binary(); n])` without `bounds`, else
  `IntVars::new(bounds.map(|(lo, hi)| IntVar::new(lo, hi)))`.
- `(lhs, rel, rhs, w)` is `Constraint::Comparison { lhs, rel: ConstraintRel::{Lt,Le,Eq,Ge,Gt}, rhs, penalty_weight: w }`.
- `(expr, "Clamp", (lo, hi), w)` is `Constraint::Clamp { expr, lo, hi, penalty_weight: w }`.
- `direction`: `"Maximize"`/`"max"` → `maximize`, `"Minimize"`/`"min"` → `minimize`.

When the port has no reason to keep the polynomial form, writing the expression directly with
`Expr` is clearer; the objective values are the same.

## Neighbor names

The Python `neighbor` string picks the Rust move type `N` in `Heuristic::<N>`.

| Problem | Python name → Rust type |
|---|---|
| MaxCut | `"Flip"` → `MaxCutFlipNeighbor`, `"Swap"` → `MaxCutSwapNeighbor` |
| Qubo | `"Flip"` → `QuboFlipNeighbor`, `"Swap"` → `QuboSwapNeighbor` |
| Sat | `"Flip"` → `SatFlipNeighbor`, `"Swap"` → `SatSwapNeighbor` |
| VertexCover | `"Flip"` → `VertexCoverFlipNeighbor`, `"Swap"` → `VertexCoverSwapNeighbor` |
| Tsp | `"TwoOpt"` → `TspTwoOptNeighbor`, `"Relocate"` → `TspRelocateNeighbor` |
| JobShopScheduling | `"Swap"` → `JobShopSwapNeighbor`, `"Relocate"` → `JobShopRelocateNeighbor` |
| Vrp | `"Relocate"` → `VrpRelocateNeighbor`, `"Swap"` → `VrpSwapNeighbor`, `"TwoOpt"` → `VrpTwoOptNeighbor` |
| GraphColoring | `"Recolor"` → `GraphColoringRecolorNeighbor`, `"Swap"` → `GraphColoringSwapNeighbor` |
| Formula | `"Change"`/`"Flip"` → `IntChangeNeighbor`, `"Swap"` → `IntSwapNeighbor`, `"Reverse"` → `IntReverseNeighbor` |

## Heuristics

`stop` is always the first Rust argument, converted as in "StopCondition". The harness wants
each heuristic boxed: `Box::new(…)`.

### On a neighbor (`build_generic`)

| Python | Rust |
|---|---|
| `LocalSearch(neighbor, stop)` | `LocalSearch::<N>::new(stop)` |
| `SimulatedAnnealing(neighbor, initial_temperature, cooling_rate, stop)` | `SimulatedAnnealing::<N>::new(stop, initial_temperature, cooling_rate)` |
| `TabuSearch(neighbor, tabu_tenure, stop)` | `TabuSearch::<N>::new(stop, (lo, hi))` |
| `LateAcceptanceHillClimbing(neighbor, history_length, stop)` | `LateAcceptanceHillClimbing::<N>::new(stop, history_length)` |
| `RandomWalk(neighbor, stop)` | `RandomWalk::<N>::new(stop)` |
| `BangBangSimulatedAnnealing(neighbor, t0, cooling, min_wave, max_wave, stop)` | `BangBangSimulatedAnnealing::<N>::new(stop, t0, cooling, min_wave, max_wave)` |
| `BeamSearch(neighbor, beam_width, stop)` | `BeamSearch::<P, N>::new(stop, beam_width)` — `P` is the problem type |
| `ReinforcementLearningSearch(neighbor, stop, learning_rate=0.01, softmax_temperature=1.0, reward_shaping="Normalized", policy_weights=None, max_candidates=None)` | `ReinforcementLearningSearch::<N>::new(stop, lr, temp, RewardShaping::{Normalized,Raw,BestImprovement}, max_candidates)`, then `.with_policy_weights(w)` when given (`[f64; NUM_FEATURES]`, `optopus::heuristic::reinforcement_learning::feature::NUM_FEATURES`) |
| `PopulationAnnealing(population_size, stop, initial_beta=0.1, delta_beta=0.02, sweeps_per_step=50, reset_period=400, neighbor="Flip", sweep_length=None)` | `optopus::heuristic::PopulationAnnealing::<P, N>::new(stop, population_size, initial_beta, delta_beta, sweeps_per_step, reset_period)` with Python `0` → `None`, else `Some(n)`; then `.with_sweep_length(n)` when given |

### Composed (`build_nested`)

Every part is a `Box<dyn Heuristic<P>>`, built by the same tables. Write a small
`fn part(...) -> Box<dyn Heuristic<P>>` per part when the tree is deep.

| Python | Rust |
|---|---|
| `VariableNeighborhoodSearch(search, shakes, stop)` | `VariableNeighborhoodSearch::new(stop, search, shakes)` |
| `Sequential(steps, stop)` | `Sequential::new(stop, steps)` |
| `Iterated(search, perturbation, stop)` | `Iterated::new(stop, search, perturbation)` |
| `Restart(heuristic, restart, stop)` | `Restart::new(stop, heuristic, restart)` — `restart` is a second `StopCondition` |
| `GeneticAlgorithm(population_size, mutation, stop, crossover=None, sub_heuristic=None, init_improvement=None, parent_selection="Tournament", parent_top_k=None, n_elite=4, n_closest=5)` | `GeneticAlgorithm::new(stop, population_size, crossover, mutation, selection)`, then `.with_init_improvement(h)` when given. `selection`: `ParentSelection::Tournament`, `::DistantTopK { top_k }`, `::BiasedFitness { n_elite, n_closest }` |
| `BreakoutLocalSearch.from_parts(descent, random, directed, tabu_tenure, t, l0, p0, stop)` | `BreakoutLocalSearch::new(stop, tabu_tenure, descent, AdaptivePerturbation::new(t, l0, p0, random, directed))`, `directed: Vec<(Box<dyn Heuristic<P>>, f64)>` |

GA crossover by problem (`crossover=None` is the first one listed; types live in
`optopus::problem`):

| Problem | Python name → Rust |
|---|---|
| MaxCut, Qubo, Sat, VertexCover | `"Uniform"` → `MaxCutUniformCrossover` / `QuboUniformCrossover` / `SatUniformCrossover` / `VertexCoverUniformCrossover`; `"SubProblem"` → `SubProblemBasedCrossover { sub_heuristic }` |
| Tsp | `"Order"` → `TspOrderCrossover`; `"SubProblem"` as above |
| JobShopScheduling | `"Ppx"` → `JobShopPpxCrossover` |
| Vrp | `"Order"` → `VrpOrderCrossover` |
| GraphColoring | `"Uniform"` → `GraphColoringUniformCrossover` |
| Formula | `"Uniform"` → `IntCrossover`; `"SubProblem"` as above |

### Problem-specific

| Python | Rust |
|---|---|
| `WalkSat(stop, noise=0.3, adaptive=False)` on Sat | `WalkSatForSat::new(stop, noise, adaptive)` |
| `BreakoutLocalSearch(tabu_tenure, t, l0, p0, q, stop)` on MaxCut | `bls_for_max_cut(stop, tabu_tenure, t, l0, p0, q)` |
| `LinKernighanHelsgaun(stop, num_neighbors=5, max_depth=5)` on Tsp | `LinKernighanHelsgaunForTsp::new(stop, num_neighbors, max_depth)` |
| `AdaptiveLargeNeighborhoodSearch(stop, removal_fraction=0.15, cooling_rate=0.9995)` on Tsp / Vrp | `alns_for_tsp(stop, removal_fraction, cooling_rate)` / `alns_for_vrp(…)` |
| the same on a Python problem | `AdaptiveLargeNeighborhoodSearch::<P>::new(stop, removal_fraction, cooling_rate)`, plus `.with_local_repair(Box::new(r))` when it had `repair_around` |
| `HybridGeneticSearch(stop, min_population_size=25, generation_size=40, granularity=20, target_feasible=0.2, restart_generations=20000)` on Vrp | `HybridGeneticSearchForVrp::new(stop, min_population_size, generation_size, granularity, target_feasible, restart_generations)` (`Option<u64>`) |

## StopCondition

`StopCondition(max_iteration=a, max_duration_secs=b, max_failed_update=c)` is
`StopCondition::new(a, b.map(Duration::from_secs_f64), c)` with each argument an `Option`.
`StopCondition::iterations(n)`, `::duration(d)`, `::failed_updates(n)` and the `.with_*`
builders say the same more readably.

## Running and the report

`heuristic.run(problem, runs=r, seed=s)` is `harness::run_all(&problem, || Box::new(…), minimize, r, s, obj, encode)`
from `templates/src/harness.rs`, which copies the binding's loop: a fresh heuristic per run,
`SearchState::new_with_seed(&problem, derive_seed(s, i))` (or `SearchState::new` for
`seed=None`), and the same report arithmetic (`std_objective` is the population deviation).

### Objective per problem

`minimize`, `obj` and `encode` must match what the binding passes in `solve`, or the numbers
will not line up with the Python report.

| Problem | `minimize` | `obj` | `encode` (the Python `solution`) |
|---|---|---|---|
| MaxCut | false | `s.objective as f64` | `json!(s.x)` |
| Qubo | true | `s.objective as f64` | `json!(s.x)` |
| Sat | false | `s.n_satisfied as f64` | `json!(s.x)` |
| VertexCover | true | `s.objective as f64` | `json!(s.x)` |
| Tsp | true | `s.objective` | `json!(s.tour)` |
| JobShopScheduling | true | `s.objective as f64` | `json!(s.operations)` |
| Vrp | true | `s.objective` | `json!(s.routes)` |
| GraphColoring | true | `s.objective as f64` | `json!(s.colors)` |
| Formula | **false** | `0.0 - s.evaluate().minimized()` (higher is better whichever the direction) | `json!(s.values())` |
| Python problem, ported | its `minimize` | the ported objective | whatever `objective` in Python takes, as JSON |

### Result fields

| Python | Rust |
|---|---|
| `report.best_objective` / `avg_` / `worst_` / `std_objective` | `report.best_objective()` / `avg_objective()` / `worst_objective()` / `std_objective()` |
| `report.runs[i].solution` | `report.runs[i].solution` (JSON); keep the typed best solution in the closure if the port needs more |
| `report.runs[i].best_iteration`, `n_accepted`, `n_rejected`, `n_best_updates` | `state.best_iteration`, … (copied into `RunResult`) |
| `report.runs[i].time_to_best_secs`, `total_time_secs` | the same names in `RunResult` |
| `vrp.evaluate_routes(routes)`, `gc.evaluate_colors(colors)`, `formula.eval_objective(values)` … | read the method in `src/problem.rs` and call the upstream function it wraps |

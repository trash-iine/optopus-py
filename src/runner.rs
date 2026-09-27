use std::time::Instant;

use optopus::heuristic::PopulationAnnealing as OptPopulationAnnealing;
use optopus::prelude::{
    BangBangSimulatedAnnealing, BeamSearch, EnabledTabu, Evaluate, Heuristic,
    HybridGeneticSearchForVrp, LateAcceptanceHillClimbing, LinKernighanHelsgaunForTsp, LocalSearch,
    MoveToNeighbor, ProblemTrait, RandomWalk, Rankable, SearchState, SimulatedAnnealing,
    TabuSearch, VariableNeighborhoodSearch, WalkSatForSat, alns_for_tsp, alns_for_vrp,
    bls_for_max_cut,
};
use optopus::problem::{
    FormulaProblem, GraphColoring as OptGraphColoring, GraphColoringRecolorNeighbor,
    GraphColoringSwapNeighbor, IntChangeNeighbor, IntReverseNeighbor, IntSwapNeighbor,
    JobShopRelocateNeighbor, JobShopScheduling as OptJobShop, JobShopSwapNeighbor,
    MaxCut as OptMaxCut, MaxCutFlipNeighbor, MaxCutSwapNeighbor, Qubo as OptQubo, QuboFlipNeighbor,
    QuboSwapNeighbor, Sat as OptSat, SatFlipNeighbor, SatSwapNeighbor, Tsp as OptTsp,
    TspRelocateNeighbor, TspTwoOptNeighbor, VertexCover as OptVc, VertexCoverFlipNeighbor,
    VertexCoverSwapNeighbor, Vrp as OptVrp, VrpRelocateNeighbor, VrpSwapNeighbor,
    VrpTwoOptNeighbor,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::heuristic as py_heuristic;
use crate::problem::{
    Formula, GraphColoring, JobShopScheduling, MaxCut, Qubo, Sat, Tsp, VertexCover, Vrp,
};
use crate::result::{RunReport, RunResult};
use crate::stop_condition::StopCondition;

/// Algorithm selection plus algorithm-specific parameters.
///
/// Variants split into three groups: generic heuristics that work on any
/// problem through a neighborhood, the metaheuristic
/// [`HeuristicKind::VariableNeighborhoodSearch`] that nests other specs, and
/// problem-specific algorithms that only one `build_*` function can construct.
pub enum HeuristicKind {
    LocalSearch,
    SimulatedAnnealing {
        initial_temperature: f64,
        cooling_rate: f64,
    },
    TabuSearch {
        tabu_tenure: (u64, u64),
    },
    LateAcceptance {
        history_length: usize,
    },
    RandomWalk,
    BangBangSimulatedAnnealing {
        initial_temperature: f64,
        cooling_rate: f64,
        min_wave_threshold: f64,
        max_wave_threshold: f64,
    },
    BeamSearch {
        beam_width: usize,
    },
    VariableNeighborhoodSearch {
        search: Box<HeuristicSpec>,
        shakes: Vec<HeuristicSpec>,
    },
    WalkSat {
        noise: f64,
        adaptive: bool,
    },
    PopulationAnnealing {
        population_size: usize,
        initial_beta: f64,
        delta_beta: f64,
        sweeps_per_step: usize,
        reset_period: Option<usize>,
    },
    BreakoutLocalSearch {
        tabu_tenure: (u64, u64),
        t: u64,
        l0: u64,
        p0: f64,
        q: f64,
    },
    LinKernighanHelsgaun {
        num_neighbors: usize,
        max_depth: usize,
    },
    AdaptiveLargeNeighborhoodSearch {
        removal_fraction: f64,
        cooling_rate: f64,
    },
    HybridGeneticSearch {
        min_population_size: usize,
        generation_size: usize,
        granularity: usize,
        target_feasible: f64,
        restart_generations: Option<u64>,
    },
}

impl HeuristicKind {
    /// The Python class name, used to build error messages.
    fn name(&self) -> &'static str {
        match self {
            Self::LocalSearch => "LocalSearch",
            Self::SimulatedAnnealing { .. } => "SimulatedAnnealing",
            Self::TabuSearch { .. } => "TabuSearch",
            Self::LateAcceptance { .. } => "LateAcceptanceHillClimbing",
            Self::RandomWalk => "RandomWalk",
            Self::BangBangSimulatedAnnealing { .. } => "BangBangSimulatedAnnealing",
            Self::BeamSearch { .. } => "BeamSearch",
            Self::VariableNeighborhoodSearch { .. } => "VariableNeighborhoodSearch",
            Self::WalkSat { .. } => "WalkSat",
            Self::PopulationAnnealing { .. } => "PopulationAnnealing",
            Self::BreakoutLocalSearch { .. } => "BreakoutLocalSearch",
            Self::LinKernighanHelsgaun { .. } => "LinKernighanHelsgaun",
            Self::AdaptiveLargeNeighborhoodSearch { .. } => "AdaptiveLargeNeighborhoodSearch",
            Self::HybridGeneticSearch { .. } => "HybridGeneticSearch",
        }
    }

    /// The problem this kind is restricted to, or `None` when it is generic.
    fn only_for(&self) -> Option<&'static str> {
        match self {
            Self::WalkSat { .. } => Some("Sat"),
            Self::BreakoutLocalSearch { .. } => Some("MaxCut"),
            Self::LinKernighanHelsgaun { .. } => Some("Tsp"),
            Self::AdaptiveLargeNeighborhoodSearch { .. } => Some("Vrp or Tsp"),
            Self::HybridGeneticSearch { .. } => Some("Vrp"),
            _ => None,
        }
    }
}

/// Fully-resolved heuristic specification handed to the dispatcher.
pub struct HeuristicSpec {
    pub kind: HeuristicKind,
    pub neighbor: String,
    pub stop: StopCondition,
}

/// Builds a boxed heuristic for a single problem type. Each problem has exactly
/// one of these; it is also the recursion step for nested specs.
type BuildFn<P> = fn(&HeuristicSpec) -> Result<Box<dyn Heuristic<P>>, String>;

/// Builds the heuristics that work on any problem `P` through neighbor type `N`.
fn build_generic<P, N>(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<P>>, String>
where
    P: ProblemTrait + 'static,
    P::Solution: Evaluate,
    N: MoveToNeighbor<P> + Rankable + Evaluate + Clone + EnabledTabu + 'static,
{
    let cond = spec.stop.to_opt();
    match &spec.kind {
        HeuristicKind::LocalSearch => Ok(Box::new(LocalSearch::<N>::new(cond))),
        HeuristicKind::SimulatedAnnealing {
            initial_temperature,
            cooling_rate,
        } => Ok(Box::new(SimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
        ))),
        HeuristicKind::TabuSearch { tabu_tenure } => {
            Ok(Box::new(TabuSearch::<N>::new(cond, *tabu_tenure)))
        }
        HeuristicKind::LateAcceptance { history_length } => Ok(Box::new(
            LateAcceptanceHillClimbing::<N>::new(cond, *history_length),
        )),
        HeuristicKind::RandomWalk => Ok(Box::new(RandomWalk::<N>::new(cond))),
        HeuristicKind::BangBangSimulatedAnnealing {
            initial_temperature,
            cooling_rate,
            min_wave_threshold,
            max_wave_threshold,
        } => Ok(Box::new(BangBangSimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
            *min_wave_threshold,
            *max_wave_threshold,
        ))),
        HeuristicKind::BeamSearch { beam_width } => {
            Ok(Box::new(BeamSearch::<P, N>::new(cond, *beam_width)))
        }
        HeuristicKind::PopulationAnnealing {
            population_size,
            initial_beta,
            delta_beta,
            sweeps_per_step,
            reset_period,
        } => Ok(Box::new(OptPopulationAnnealing::<P, N>::new(
            cond,
            *population_size,
            *initial_beta,
            *delta_beta,
            *sweeps_per_step,
            *reset_period,
        ))),
        other => Err(unsupported(other)),
    }
}

/// Builds the kinds that carry no neighborhood of their own, so they can be
/// resolved before a problem picks its concrete neighbor type. Returns `None`
/// when `spec` has to go through that neighbor match instead.
fn build_nested<P>(
    spec: &HeuristicSpec,
    recur: BuildFn<P>,
) -> Option<Result<Box<dyn Heuristic<P>>, String>>
where
    P: ProblemTrait + 'static,
{
    match &spec.kind {
        HeuristicKind::VariableNeighborhoodSearch { search, shakes } => {
            Some(build_vns(spec.stop.to_opt(), search, shakes, recur))
        }
        _ => None,
    }
}

fn build_vns<P>(
    cond: optopus::prelude::StopCondition,
    search: &HeuristicSpec,
    shakes: &[HeuristicSpec],
    recur: BuildFn<P>,
) -> Result<Box<dyn Heuristic<P>>, String>
where
    P: ProblemTrait + 'static,
{
    if shakes.is_empty() {
        return Err("'shakes' must contain at least one heuristic".to_string());
    }
    let search = recur(search)?;
    let shakes = shakes
        .iter()
        .map(recur)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Box::new(VariableNeighborhoodSearch::new(
        cond, search, shakes,
    )))
}

/// Error text for a problem-specific heuristic applied to the wrong problem.
fn unsupported(kind: &HeuristicKind) -> String {
    match kind.only_for() {
        Some(problem) => format!("{} is only available for {problem}", kind.name()),
        None => format!("{} is not supported for this problem", kind.name()),
    }
}

/// Error for a spec that fell through a problem's neighbor match.
///
/// Problem-specific kinds carry no neighborhood, so reaching here means the
/// problem simply does not implement them; they get the "wrong problem" message
/// rather than a confusing complaint about an empty neighbor.
fn neighbor_error(spec: &HeuristicSpec, problem: &str, valid: &str) -> String {
    if spec.kind.only_for().is_some() {
        return unsupported(&spec.kind);
    }
    let given = &spec.neighbor;
    format!("invalid neighbor '{given}' for {problem} (use {valid})")
}

fn build_max_cut(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptMaxCut>>, String> {
    if let Some(result) = build_nested(spec, build_max_cut) {
        return result;
    }
    match &spec.kind {
        HeuristicKind::BreakoutLocalSearch {
            tabu_tenure,
            t,
            l0,
            p0,
            q,
        } => Ok(Box::new(bls_for_max_cut(
            spec.stop.to_opt(),
            *tabu_tenure,
            *t,
            *l0,
            *p0,
            *q,
        ))),
        _ => match spec.neighbor.as_str() {
            "Flip" => build_generic::<OptMaxCut, MaxCutFlipNeighbor>(spec),
            "Swap" => build_generic::<OptMaxCut, MaxCutSwapNeighbor>(spec),
            _ => Err(neighbor_error(spec, "MaxCut", "'Flip' or 'Swap'")),
        },
    }
}

fn build_qubo(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptQubo>>, String> {
    if let Some(result) = build_nested(spec, build_qubo) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Flip" => build_generic::<OptQubo, QuboFlipNeighbor>(spec),
        "Swap" => build_generic::<OptQubo, QuboSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "Qubo", "'Flip' or 'Swap'")),
    }
}

fn build_sat(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptSat>>, String> {
    if let Some(result) = build_nested(spec, build_sat) {
        return result;
    }
    match &spec.kind {
        HeuristicKind::WalkSat { noise, adaptive } => Ok(Box::new(WalkSatForSat::new(
            spec.stop.to_opt(),
            *noise,
            *adaptive,
        ))),
        _ => match spec.neighbor.as_str() {
            "Flip" => build_generic::<OptSat, SatFlipNeighbor>(spec),
            "Swap" => build_generic::<OptSat, SatSwapNeighbor>(spec),
            _ => Err(neighbor_error(spec, "Sat", "'Flip' or 'Swap'")),
        },
    }
}

fn build_vertex_cover(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptVc>>, String> {
    if let Some(result) = build_nested(spec, build_vertex_cover) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Flip" => build_generic::<OptVc, VertexCoverFlipNeighbor>(spec),
        "Swap" => build_generic::<OptVc, VertexCoverSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "VertexCover", "'Flip' or 'Swap'")),
    }
}

fn build_tsp(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptTsp>>, String> {
    if let Some(result) = build_nested(spec, build_tsp) {
        return result;
    }
    match &spec.kind {
        HeuristicKind::LinKernighanHelsgaun {
            num_neighbors,
            max_depth,
        } => Ok(Box::new(LinKernighanHelsgaunForTsp::new(
            spec.stop.to_opt(),
            *num_neighbors,
            *max_depth,
        ))),
        HeuristicKind::AdaptiveLargeNeighborhoodSearch {
            removal_fraction,
            cooling_rate,
        } => Ok(Box::new(alns_for_tsp(
            spec.stop.to_opt(),
            *removal_fraction,
            *cooling_rate,
        ))),
        _ => match spec.neighbor.as_str() {
            "TwoOpt" => build_generic::<OptTsp, TspTwoOptNeighbor>(spec),
            "Relocate" => build_generic::<OptTsp, TspRelocateNeighbor>(spec),
            _ => Err(neighbor_error(spec, "Tsp", "'TwoOpt' or 'Relocate'")),
        },
    }
}

fn build_job_shop(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptJobShop>>, String> {
    if let Some(result) = build_nested(spec, build_job_shop) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Swap" => build_generic::<OptJobShop, JobShopSwapNeighbor>(spec),
        "Relocate" => build_generic::<OptJobShop, JobShopRelocateNeighbor>(spec),
        _ => Err(neighbor_error(
            spec,
            "JobShopScheduling",
            "'Swap' or 'Relocate'",
        )),
    }
}

fn build_vrp(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptVrp>>, String> {
    if let Some(result) = build_nested(spec, build_vrp) {
        return result;
    }
    let cond = spec.stop.to_opt();
    match &spec.kind {
        HeuristicKind::AdaptiveLargeNeighborhoodSearch {
            removal_fraction,
            cooling_rate,
        } => Ok(Box::new(alns_for_vrp(
            cond,
            *removal_fraction,
            *cooling_rate,
        ))),
        HeuristicKind::HybridGeneticSearch {
            min_population_size,
            generation_size,
            granularity,
            target_feasible,
            restart_generations,
        } => Ok(Box::new(HybridGeneticSearchForVrp::new(
            cond,
            *min_population_size,
            *generation_size,
            *granularity,
            *target_feasible,
            *restart_generations,
        ))),
        _ => match spec.neighbor.as_str() {
            "Relocate" => build_generic::<OptVrp, VrpRelocateNeighbor>(spec),
            "Swap" => build_generic::<OptVrp, VrpSwapNeighbor>(spec),
            "TwoOpt" => build_generic::<OptVrp, VrpTwoOptNeighbor>(spec),
            _ => Err(neighbor_error(
                spec,
                "Vrp",
                "'Relocate', 'Swap' or 'TwoOpt'",
            )),
        },
    }
}

fn build_formula(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<FormulaProblem>>, String> {
    if let Some(result) = build_nested(spec, build_formula) {
        return result;
    }
    match spec.neighbor.as_str() {
        // On a binary variable an integer change *is* a flip, which upstream says in as many
        // words, so "Flip" stays accepted as the name it had before the integer layer.
        "Change" | "Flip" => build_generic::<FormulaProblem, IntChangeNeighbor>(spec),
        "Swap" => build_generic::<FormulaProblem, IntSwapNeighbor>(spec),
        "Reverse" => build_generic::<FormulaProblem, IntReverseNeighbor>(spec),
        _ => Err(neighbor_error(
            spec,
            "Formula",
            "'Change' (or its alias 'Flip'), 'Swap' or 'Reverse'",
        )),
    }
}

fn build_graph_coloring(
    spec: &HeuristicSpec,
) -> Result<Box<dyn Heuristic<OptGraphColoring>>, String> {
    if let Some(result) = build_nested(spec, build_graph_coloring) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Recolor" => build_generic::<OptGraphColoring, GraphColoringRecolorNeighbor>(spec),
        "Swap" => build_generic::<OptGraphColoring, GraphColoringSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "GraphColoring", "'Recolor' or 'Swap'")),
    }
}

/// Derives a deterministic per-run seed from a master seed. Run index 0 maps to
/// the master itself, so a single seeded run reproduces with `new_with_seed`.
fn derive_seed(master: u64, run_index: usize) -> u64 {
    master.wrapping_add((run_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Runs the heuristic `runs` times on `instance` and aggregates a report.
fn run_all<P>(
    instance: &P,
    build: impl Fn() -> Result<Box<dyn Heuristic<P>>, String>,
    minimize: bool,
    runs: usize,
    seed: Option<u64>,
    obj: impl Fn(&P::Solution) -> f64,
    decode: impl Fn(&P::Solution, Python<'_>) -> Py<PyAny>,
) -> Result<RunReport, String>
where
    P: ProblemTrait + 'static,
{
    let mut results = Vec::with_capacity(runs);
    for run_index in 0..runs {
        let run_seed = seed.map(|m| derive_seed(m, run_index));
        let mut heuristic = build()?;

        let start = Instant::now();
        let mut state = match run_seed {
            Some(s) => SearchState::new_with_seed(instance, s),
            None => SearchState::new(instance),
        };
        let initial_objective = obj(&state.initial_solution);
        let _ = heuristic.run(&mut state);
        let total_time = start.elapsed();

        let best_objective = obj(&state.best_solution);
        let raw_diff = best_objective - initial_objective;
        let improvement = if minimize { -raw_diff } else { raw_diff };

        let solution = Python::attach(|py| decode(&state.best_solution, py));

        results.push(RunResult {
            best_objective,
            solution,
            best_iteration: state.best_iteration,
            time_to_best_secs: state
                .best_time
                .saturating_duration_since(start)
                .as_secs_f64(),
            total_time_secs: total_time.as_secs_f64(),
            initial_objective,
            improvement,
            n_accepted: state.n_accepted,
            n_rejected: state.n_rejected,
            n_best_updates: state.n_best_updates,
            seed: run_seed,
        });
    }

    Ok(RunReport::from_runs(results, minimize))
}

fn decode_bools(x: &[bool], py: Python<'_>) -> Py<PyAny> {
    PyList::new(py, x).unwrap().into()
}

fn decode_usizes(x: &[usize], py: Python<'_>) -> Py<PyAny> {
    PyList::new(py, x).unwrap().into()
}

fn decode_i64s(x: &[i64], py: Python<'_>) -> Py<PyAny> {
    PyList::new(py, x).unwrap().into()
}

/// Decodes a VRP route partition into a `list[list[int]]`.
fn decode_routes(routes: &[Vec<usize>], py: Python<'_>) -> Py<PyAny> {
    let rows: Vec<Py<PyAny>> = routes.iter().map(|r| decode_usizes(r, py)).collect();
    PyList::new(py, rows).unwrap().into()
}

/// Rebuilds a `HeuristicSpec` from any heuristic instance handed in from Python.
///
/// Used by `VariableNeighborhoodSearch`, whose search and shake steps are
/// ordinary Python heuristic objects.
pub fn spec_from_py(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<HeuristicSpec> {
    macro_rules! try_heuristics {
        ($($ty:ident),+ $(,)?) => {
            $(
                if let Ok(h) = obj.extract::<PyRef<'_, py_heuristic::$ty>>() {
                    return h.to_spec(py);
                }
            )+
        };
    }

    try_heuristics!(
        LocalSearch,
        SimulatedAnnealing,
        TabuSearch,
        LateAcceptanceHillClimbing,
        RandomWalk,
        BangBangSimulatedAnnealing,
        BeamSearch,
        VariableNeighborhoodSearch,
        WalkSat,
        PopulationAnnealing,
        BreakoutLocalSearch,
        LinKernighanHelsgaun,
        AdaptiveLargeNeighborhoodSearch,
        HybridGeneticSearch,
    );

    Err(PyTypeError::new_err(
        "expected a heuristic instance (e.g. LocalSearch, SimulatedAnnealing, RandomWalk)",
    ))
}

/// Entry point: dispatch on the concrete Python problem type and run.
pub fn solve(
    problem: &Bound<'_, PyAny>,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> PyResult<RunReport> {
    if runs == 0 {
        return Err(PyValueError::new_err("'runs' must be at least 1"));
    }

    let report = if let Ok(mc) = problem.extract::<PyRef<'_, MaxCut>>() {
        run_all(
            &mc.inner,
            || build_max_cut(spec),
            false,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )
    } else if let Ok(q) = problem.extract::<PyRef<'_, Qubo>>() {
        run_all(
            &q.inner,
            || build_qubo(spec),
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )
    } else if let Ok(s) = problem.extract::<PyRef<'_, Sat>>() {
        run_all(
            &s.inner,
            || build_sat(spec),
            false,
            runs,
            seed,
            |s| s.n_satisfied as f64,
            |s, py| decode_bools(&s.x, py),
        )
    } else if let Ok(v) = problem.extract::<PyRef<'_, VertexCover>>() {
        run_all(
            &v.inner,
            || build_vertex_cover(spec),
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )
    } else if let Ok(t) = problem.extract::<PyRef<'_, Tsp>>() {
        run_all(
            &t.inner,
            || build_tsp(spec),
            true,
            runs,
            seed,
            |s| s.objective,
            |s, py| decode_usizes(&s.tour, py),
        )
    } else if let Ok(j) = problem.extract::<PyRef<'_, JobShopScheduling>>() {
        run_all(
            &j.inner,
            || build_job_shop(spec),
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_usizes(&s.operations, py),
        )
    } else if let Ok(v) = problem.extract::<PyRef<'_, Vrp>>() {
        run_all(
            &v.inner,
            || build_vrp(spec),
            true,
            runs,
            seed,
            |s| s.objective,
            |s, py| decode_routes(&s.routes, py),
        )
    } else if let Ok(g) = problem.extract::<PyRef<'_, GraphColoring>>() {
        run_all(
            &g.inner,
            || build_graph_coloring(spec),
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_usizes(&s.colors, py),
        )
    } else if let Ok(f) = problem.extract::<PyRef<'_, Formula>>() {
        // A FormulaSolution reports its objective as an `Evaluable`, whose `minimized()` is the
        // value with the direction already applied. Negating it recovers the
        // direction-corrected score the report has always published, where higher is better --
        // so the runner tracks it as a maximization whichever way the formula optimizes.
        run_all(
            &f.inner,
            || build_formula(spec),
            false,
            runs,
            seed,
            // Subtracting rather than negating keeps a zero objective reported as 0.0.
            |s| 0.0 - s.evaluate().minimized(),
            |s, py| decode_i64s(s.values(), py),
        )
    } else {
        return Err(PyTypeError::new_err(
            "problem must be a MaxCut, Qubo, Sat, VertexCover, Tsp, JobShopScheduling, Vrp, GraphColoring, or Formula instance",
        ));
    };

    report.map_err(PyValueError::new_err)
}

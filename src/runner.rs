use std::time::Instant;

use optopus::prelude::{
    BangBangSimulatedAnnealing, BeamSearch, EnabledTabu, Evaluate, Heuristic,
    LateAcceptanceHillClimbing, LocalSearch, MoveToNeighbor, ProblemTrait, RandomWalk, Rankable,
    SearchState, SimulatedAnnealing, TabuSearch,
};
use optopus::problem::{
    FormulaFlipNeighbor, FormulaProblem, FormulaSwapNeighbor, JobShopRelocateNeighbor,
    JobShopScheduling as OptJobShop, JobShopSwapNeighbor, MaxCut as OptMaxCut, MaxCutFlipNeighbor,
    MaxCutSwapNeighbor, Qubo as OptQubo, QuboFlipNeighbor, QuboSwapNeighbor, Sat as OptSat,
    SatFlipNeighbor, SatSwapNeighbor, TspRelocateNeighbor, TspTwoOptNeighbor,
    TspWithCoordinates as OptTsp, VertexCover as OptVc, VertexCoverFlipNeighbor,
    VertexCoverSwapNeighbor,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::problem::{
    Formula, JobShopScheduling, MaxCut, Qubo, Sat, TspWithCoordinates, VertexCover,
};
use crate::result::{RunReport, RunResult};
use crate::stop_condition::StopCondition;

/// Algorithm selection plus algorithm-specific parameters.
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
}

/// Fully-resolved heuristic specification handed to the dispatcher.
pub struct HeuristicSpec {
    pub kind: HeuristicKind,
    pub neighbor: String,
    pub stop: StopCondition,
}

/// Builds a boxed heuristic for problem `P` over the concrete neighbor type `N`.
fn build_heuristic<P, N>(spec: &HeuristicSpec) -> Box<dyn Heuristic<P>>
where
    P: ProblemTrait + 'static,
    N: MoveToNeighbor<P> + Rankable + Evaluate + Clone + EnabledTabu + 'static,
{
    let cond = spec.stop.to_opt();
    match &spec.kind {
        HeuristicKind::LocalSearch => Box::new(LocalSearch::<N>::new(cond)),
        HeuristicKind::SimulatedAnnealing {
            initial_temperature,
            cooling_rate,
        } => Box::new(SimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
        )),
        HeuristicKind::TabuSearch { tabu_tenure } => {
            Box::new(TabuSearch::<N>::new(cond, *tabu_tenure, None))
        }
        HeuristicKind::LateAcceptance { history_length } => {
            Box::new(LateAcceptanceHillClimbing::<N>::new(cond, *history_length))
        }
        HeuristicKind::RandomWalk => Box::new(RandomWalk::<N>::new(cond)),
        HeuristicKind::BangBangSimulatedAnnealing {
            initial_temperature,
            cooling_rate,
            min_wave_threshold,
            max_wave_threshold,
        } => Box::new(BangBangSimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
            *min_wave_threshold,
            *max_wave_threshold,
        )),
        HeuristicKind::BeamSearch { beam_width } => {
            Box::new(BeamSearch::<P, N>::new(cond, *beam_width))
        }
    }
}

/// Derives a deterministic per-run seed from a master seed. Run index 0 maps to
/// the master itself, so a single seeded run reproduces with `new_with_seed`.
fn derive_seed(master: u64, run_index: usize) -> u64 {
    master.wrapping_add((run_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Runs the heuristic `runs` times on `instance` and aggregates a report.
fn run_all<P, N>(
    instance: &P,
    spec: &HeuristicSpec,
    minimize: bool,
    runs: usize,
    seed: Option<u64>,
    obj: impl Fn(&P::Solution) -> f64,
    decode: impl Fn(&P::Solution, Python<'_>) -> Py<PyAny>,
) -> RunReport
where
    P: ProblemTrait + 'static,
    N: MoveToNeighbor<P> + Rankable + Evaluate + Clone + EnabledTabu + 'static,
{
    let mut results = Vec::with_capacity(runs);
    for run_index in 0..runs {
        let run_seed = seed.map(|m| derive_seed(m, run_index));
        let mut heuristic = build_heuristic::<P, N>(spec);

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

    RunReport::from_runs(results, minimize)
}

fn decode_bools(x: &[bool], py: Python<'_>) -> Py<PyAny> {
    PyList::new(py, x).unwrap().into()
}

fn decode_usizes(x: &[usize], py: Python<'_>) -> Py<PyAny> {
    PyList::new(py, x).unwrap().into()
}

fn dispatch_maxcut(
    instance: &OptMaxCut,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "Flip" => Ok(run_all::<OptMaxCut, MaxCutFlipNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        "Swap" => Ok(run_all::<OptMaxCut, MaxCutSwapNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for MaxCut (use 'Flip' or 'Swap')"
        )),
    }
}

fn dispatch_qubo(
    instance: &OptQubo,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "Flip" => Ok(run_all::<OptQubo, QuboFlipNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        "Swap" => Ok(run_all::<OptQubo, QuboSwapNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for Qubo (use 'Flip' or 'Swap')"
        )),
    }
}

fn dispatch_sat(
    instance: &OptSat,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "Flip" => Ok(run_all::<OptSat, SatFlipNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.n_satisfied as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        "Swap" => Ok(run_all::<OptSat, SatSwapNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.n_satisfied as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for Sat (use 'Flip' or 'Swap')"
        )),
    }
}

fn dispatch_vc(
    instance: &OptVc,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "Flip" => Ok(run_all::<OptVc, VertexCoverFlipNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        "Swap" => Ok(run_all::<OptVc, VertexCoverSwapNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_bools(&s.x, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for VertexCover (use 'Flip' or 'Swap')"
        )),
    }
}

fn dispatch_tsp(
    instance: &OptTsp,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "TwoOpt" => Ok(run_all::<OptTsp, TspTwoOptNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective,
            |s, py| decode_usizes(&s.tour, py),
        )),
        "Relocate" => Ok(run_all::<OptTsp, TspRelocateNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective,
            |s, py| decode_usizes(&s.tour, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for TspWithCoordinates (use 'TwoOpt' or 'Relocate')"
        )),
    }
}

fn dispatch_jss(
    instance: &OptJobShop,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    match spec.neighbor.as_str() {
        "Swap" => Ok(run_all::<OptJobShop, JobShopSwapNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_usizes(&s.operations, py),
        )),
        "Relocate" => Ok(run_all::<OptJobShop, JobShopRelocateNeighbor>(
            instance,
            spec,
            true,
            runs,
            seed,
            |s| s.objective as f64,
            |s, py| decode_usizes(&s.operations, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for JobShopScheduling (use 'Swap' or 'Relocate')"
        )),
    }
}

fn dispatch_formula(
    instance: &FormulaProblem,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
) -> Result<RunReport, String> {
    // FormulaSolution.score is direction-corrected (higher is always better);
    // the runner-level objective is therefore tracked as maximization.
    match spec.neighbor.as_str() {
        "Flip" => Ok(run_all::<FormulaProblem, FormulaFlipNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.score,
            |s, py| decode_bools(&s.x, py),
        )),
        "Swap" => Ok(run_all::<FormulaProblem, FormulaSwapNeighbor>(
            instance,
            spec,
            false,
            runs,
            seed,
            |s| s.score,
            |s, py| decode_bools(&s.x, py),
        )),
        other => Err(format!(
            "invalid neighbor '{other}' for Formula (use 'Flip' or 'Swap')"
        )),
    }
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
        dispatch_maxcut(&mc.inner, spec, runs, seed)
    } else if let Ok(q) = problem.extract::<PyRef<'_, Qubo>>() {
        dispatch_qubo(&q.inner, spec, runs, seed)
    } else if let Ok(s) = problem.extract::<PyRef<'_, Sat>>() {
        dispatch_sat(&s.inner, spec, runs, seed)
    } else if let Ok(v) = problem.extract::<PyRef<'_, VertexCover>>() {
        dispatch_vc(&v.inner, spec, runs, seed)
    } else if let Ok(t) = problem.extract::<PyRef<'_, TspWithCoordinates>>() {
        dispatch_tsp(&t.inner, spec, runs, seed)
    } else if let Ok(j) = problem.extract::<PyRef<'_, JobShopScheduling>>() {
        dispatch_jss(&j.inner, spec, runs, seed)
    } else if let Ok(f) = problem.extract::<PyRef<'_, Formula>>() {
        dispatch_formula(&f.inner, spec, runs, seed)
    } else {
        return Err(PyTypeError::new_err(
            "problem must be a MaxCut, Qubo, Sat, VertexCover, TspWithCoordinates, JobShopScheduling, or Formula instance",
        ));
    };

    report.map_err(PyValueError::new_err)
}

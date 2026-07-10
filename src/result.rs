use pyo3::prelude::*;

/// Metrics and solution for a single heuristic run.
///
/// All attributes are read-only.
#[pyclass(module = "optopus", skip_from_py_object)]
pub struct RunResult {
    /// Best objective value found during the run.
    #[pyo3(get)]
    pub best_objective: f64,
    /// Best solution found during the run.
    ///
    /// Shape depends on the problem:
    ///
    /// - ``MaxCut`` / ``Qubo`` / ``Sat`` / ``VertexCover`` / ``Formula`` →
    ///   ``list[bool]`` of length *number of variables/vertices*.
    /// - ``TspWithCoordinates`` → ``list[int]`` permutation of city indices.
    /// - ``JobShopScheduling`` → ``list[int]`` operation sequence.
    #[pyo3(get)]
    pub solution: Py<PyAny>,
    /// Iteration at which the best solution was found.
    #[pyo3(get)]
    pub best_iteration: u64,
    /// Elapsed seconds until the best solution was found.
    #[pyo3(get)]
    pub time_to_best_secs: f64,
    /// Total elapsed seconds for the run.
    #[pyo3(get)]
    pub total_time_secs: f64,
    /// Objective of the random initial solution.
    #[pyo3(get)]
    pub initial_objective: f64,
    /// Improvement from initial to best, sign-corrected (positive = better).
    #[pyo3(get)]
    pub improvement: f64,
    /// Number of accepted moves.
    #[pyo3(get)]
    pub n_accepted: u64,
    /// Number of iterations advanced without applying a move.
    #[pyo3(get)]
    pub n_rejected: u64,
    /// Number of times the best solution was strictly improved.
    #[pyo3(get)]
    pub n_best_updates: u64,
    /// Seed actually used for the run, if one was provided.
    #[pyo3(get)]
    pub seed: Option<u64>,
}

#[pymethods]
impl RunResult {
    fn __repr__(&self) -> String {
        format!(
            "RunResult(best_objective={}, best_iteration={}, time_to_best_secs={:.6}, total_time_secs={:.6})",
            self.best_objective, self.best_iteration, self.time_to_best_secs, self.total_time_secs
        )
    }
}

/// Aggregated report over one or more runs of the same heuristic on a problem.
///
/// Returned by every heuristic ``run`` call. All attributes are read-only.
/// ``best_objective`` / ``worst_objective`` follow the problem direction
/// (maximum for Max Cut, minimum for QUBO).
#[pyclass(module = "optopus")]
pub struct RunReport {
    /// Individual run results, ordered by run index.
    #[pyo3(get)]
    pub runs: Vec<Py<RunResult>>,
    /// Number of runs performed.
    #[pyo3(get)]
    pub num_runs: usize,
    /// Best objective across all runs (maximum for Max Cut, minimum for QUBO).
    #[pyo3(get)]
    pub best_objective: f64,
    /// Mean best objective across runs.
    #[pyo3(get)]
    pub avg_objective: f64,
    /// Worst objective across all runs.
    #[pyo3(get)]
    pub worst_objective: f64,
    /// Population standard deviation of the best objective across runs.
    #[pyo3(get)]
    pub std_objective: f64,
    /// Fastest time-to-best across runs, in seconds.
    #[pyo3(get)]
    pub best_time_to_best_secs: f64,
    /// Mean time-to-best across runs, in seconds.
    #[pyo3(get)]
    pub avg_time_to_best_secs: f64,
    /// Mean total run time across runs, in seconds.
    #[pyo3(get)]
    pub avg_total_time_secs: f64,
    /// Mean objective of the random initial solution across runs.
    #[pyo3(get)]
    pub avg_initial_objective: f64,
    /// Mean improvement from initial to best across runs (sign-corrected; positive = better).
    #[pyo3(get)]
    pub avg_improvement: f64,
    /// Mean number of accepted moves across runs.
    #[pyo3(get)]
    pub avg_n_accepted: f64,
    /// Mean number of rejected iterations across runs.
    #[pyo3(get)]
    pub avg_n_rejected: f64,
    /// Mean number of best-solution improvements across runs.
    #[pyo3(get)]
    pub avg_n_best_updates: f64,
}

#[pymethods]
impl RunReport {
    fn __repr__(&self) -> String {
        format!(
            "RunReport(num_runs={}, best_objective={}, avg_objective={}, worst_objective={}, std_objective={})",
            self.num_runs, self.best_objective, self.avg_objective, self.worst_objective, self.std_objective
        )
    }
}

impl RunReport {
    /// Aggregates run results into a report. `minimize` selects the direction
    /// for best/worst objective. Assumes `runs` is non-empty.
    pub fn from_runs(runs: Vec<RunResult>, minimize: bool) -> Self {
        let n = runs.len();
        let nf = n as f64;

        let objectives: Vec<f64> = runs.iter().map(|r| r.best_objective).collect();
        let (best, worst) = if minimize {
            (
                objectives.iter().cloned().fold(f64::INFINITY, f64::min),
                objectives.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        } else {
            (
                objectives.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                objectives.iter().cloned().fold(f64::INFINITY, f64::min),
            )
        };
        let avg = objectives.iter().sum::<f64>() / nf;
        let variance = objectives.iter().map(|&x| (x - avg).powi(2)).sum::<f64>() / nf;
        let std = variance.sqrt();

        let times: Vec<f64> = runs.iter().map(|r| r.time_to_best_secs).collect();
        let best_ttb = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let avg_ttb = times.iter().sum::<f64>() / nf;
        let avg_total = runs.iter().map(|r| r.total_time_secs).sum::<f64>() / nf;
        let avg_initial = runs.iter().map(|r| r.initial_objective).sum::<f64>() / nf;
        let avg_improvement = runs.iter().map(|r| r.improvement).sum::<f64>() / nf;
        let avg_accepted = runs.iter().map(|r| r.n_accepted as f64).sum::<f64>() / nf;
        let avg_rejected = runs.iter().map(|r| r.n_rejected as f64).sum::<f64>() / nf;
        let avg_best_updates = runs.iter().map(|r| r.n_best_updates as f64).sum::<f64>() / nf;

        let py_runs: Vec<Py<RunResult>> = Python::attach(|py| {
            runs.into_iter()
                .map(|r| Py::new(py, r).expect("failed to allocate Py<RunResult>"))
                .collect()
        });

        Self {
            runs: py_runs,
            num_runs: n,
            best_objective: best,
            avg_objective: avg,
            worst_objective: worst,
            std_objective: std,
            best_time_to_best_secs: best_ttb,
            avg_time_to_best_secs: avg_ttb,
            avg_total_time_secs: avg_total,
            avg_initial_objective: avg_initial,
            avg_improvement,
            avg_n_accepted: avg_accepted,
            avg_n_rejected: avg_rejected,
            avg_n_best_updates: avg_best_updates,
        }
    }
}

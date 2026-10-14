use pyo3::prelude::*;

/// Metrics and solution for a single heuristic run.
///
/// All attributes are read-only.
#[pyclass(module = "optopus", skip_from_py_object)]
pub struct RunResult {
    /// Best objective value found during the run, on the scale the search ranks solutions by.
    ///
    /// For ``VertexCover``, ``Vrp``, ``GraphColoring`` and ``Formula`` it includes the
    /// constraint penalty, for ``Sat`` it counts the satisfied clauses, for ``BinPacking`` it is
    /// the bin count less a fill term in ``(0, 1]``, and a ``Formula`` reports it
    /// higher-is-better, negated when the formula minimizes.
    #[pyo3(get)]
    pub best_objective: f64,
    /// Best solution found during the run.
    ///
    /// Shape depends on the problem:
    ///
    /// - ``MaxCut`` / ``Qubo`` / ``Sat`` / ``VertexCover`` → ``list[bool]`` of length
    ///   *number of variables/vertices*.
    /// - ``Tsp`` → ``list[int]`` permutation of city indices.
    /// - ``JobShopScheduling`` → ``list[int]`` operation sequence.
    /// - ``FlowShop`` → ``list[int]``, the jobs in the order they run.
    /// - ``Qap`` → ``list[int]``, the location of each facility.
    /// - ``BinPacking`` → ``list[int]``, the bin of each item, numbered from 0 in order of each
    ///   bin's first item.
    /// - ``Vrp`` → ``list[list[int]]``, one route of customer indices per vehicle. The list
    ///   has ``Vrp.num_vehicles()`` entries, route ``i`` is driven by vehicle type
    ///   ``Vrp.slot_types()[i]``, the depot is implicit at both ends of each route, and empty
    ///   routes may appear.
    /// - ``GraphColoring`` → ``list[int]``, each vertex's color in ``[0, num_colors)``.
    /// - ``Formula`` → ``list[int]``, one value per variable. Binary variables report 0 and 1,
    ///   which compare equal to ``False`` and ``True``.
    /// - A problem written in Python → the solution object its methods produced, as it is.
    #[pyo3(get)]
    pub solution: Py<PyAny>,
    /// Iteration at which the best solution was found.
    #[pyo3(get)]
    pub best_iteration: u64,
    /// Iterations the run made, in the heuristic's own unit (see "Stopping" in the docs).
    ///
    /// It includes what a heuristic charges as iterations without accepting or rejecting a
    /// move, such as ``HybridGeneticSearch``'s initial population, so it can exceed
    /// ``n_accepted + n_rejected``.
    #[pyo3(get)]
    pub iterations: u64,
    /// Elapsed seconds until the best solution was found.
    ///
    /// On a floating-point objective (``Vrp``, ``Tsp``) a gain at rounding level counts as a new
    /// best, which can push this toward the end of the run; see ``n_best_updates``.
    #[pyo3(get)]
    pub time_to_best_secs: f64,
    /// Total elapsed seconds for the run.
    #[pyo3(get)]
    pub total_time_secs: f64,
    /// Objective of the random initial solution, on the same scale as ``best_objective``.
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
    ///
    /// Any strict gain counts, including one of rounding size on a floating-point objective.
    /// Thousands of updates late in a run usually mean the search reached its best long before.
    #[pyo3(get)]
    pub n_best_updates: u64,
    /// Seed actually used for the run, if one was provided.
    #[pyo3(get)]
    pub seed: Option<u64>,
    /// Whether the run proved its best solution optimal, as far as the relaxation is a bound.
    ///
    /// Set only when the heuristic run is a ``BranchAndBound``, and ``None`` otherwise. It is
    /// ``True`` when the search emptied its open nodes before its stop condition.
    #[pyo3(get)]
    pub proven_optimal: Option<bool>,
    /// The best objective a solution not yet ruled out could reach, on the same scale as
    /// ``best_objective``.
    ///
    /// Set only when the heuristic run is a ``BranchAndBound``, and ``None`` otherwise. It equals
    /// ``best_objective`` once ``proven_optimal`` is ``True``, and the gap between the two bounds
    /// how far a stopped search can be from the optimum. It is infinite when the search stopped
    /// before the whole problem had been split once.
    #[pyo3(get)]
    pub dual_bound: Option<f64>,
    /// Every improvement of the best solution, as ``(elapsed_secs, iteration, objective)``
    /// tuples in the order they happened.
    ///
    /// ``elapsed_secs`` is measured on the clock ``time_to_best_secs`` uses and ``objective`` is
    /// on the scale of ``best_objective``, so the last entry, if any, carries the run's best.
    /// Each entry is strictly better than the one before it. The list is empty when the run
    /// never improved on its initial solution.
    #[pyo3(get)]
    pub trajectory: Vec<(f64, u64, f64)>,
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
/// ``best_objective`` / ``worst_objective`` follow the problem direction: the lowest value
/// is the best for a minimizing problem such as ``Qubo``, the highest for a maximizing one such
/// as ``MaxCut``. A ``Formula`` always reports higher-is-better, whichever way it optimizes.
///
/// The report holds no best solution of its own: pick the run whose ``best_objective`` equals
/// the report's and read its ``solution``.
#[pyclass(module = "optopus")]
pub struct RunReport {
    /// Individual run results, ordered by run index.
    #[pyo3(get)]
    pub runs: Vec<Py<RunResult>>,
    /// Number of runs performed.
    #[pyo3(get)]
    pub num_runs: usize,
    /// Best objective across all runs, in the problem's direction.
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
    /// Mean number of iterations across runs.
    #[pyo3(get)]
    pub avg_iterations: f64,
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
            self.num_runs,
            self.best_objective,
            self.avg_objective,
            self.worst_objective,
            self.std_objective
        )
    }
}

impl RunReport {
    /// Aggregates run results into a report. `minimize` selects the direction
    /// for best/worst objective. Assumes `runs` is non-empty.
    pub fn from_runs(runs: Vec<RunResult>, minimize: bool) -> Self {
        let n = runs.len();
        let nf = n as f64;

        let mean = |f: fn(&RunResult) -> f64| runs.iter().map(f).sum::<f64>() / nf;
        let objectives = || runs.iter().map(|r| r.best_objective);

        let lowest = objectives().fold(f64::INFINITY, f64::min);
        let highest = objectives().fold(f64::NEG_INFINITY, f64::max);
        let (best, worst) = if minimize {
            (lowest, highest)
        } else {
            (highest, lowest)
        };
        let avg = mean(|r| r.best_objective);
        let variance = objectives().map(|x| (x - avg).powi(2)).sum::<f64>() / nf;
        let std = variance.sqrt();

        let best_ttb = runs
            .iter()
            .map(|r| r.time_to_best_secs)
            .fold(f64::INFINITY, f64::min);
        let avg_ttb = mean(|r| r.time_to_best_secs);
        let avg_total = mean(|r| r.total_time_secs);
        let avg_initial = mean(|r| r.initial_objective);
        let avg_improvement = mean(|r| r.improvement);
        let avg_iterations = mean(|r| r.iterations as f64);
        let avg_accepted = mean(|r| r.n_accepted as f64);
        let avg_rejected = mean(|r| r.n_rejected as f64);
        let avg_best_updates = mean(|r| r.n_best_updates as f64);

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
            avg_iterations,
            avg_n_accepted: avg_accepted,
            avg_n_rejected: avg_rejected,
            avg_n_best_updates: avg_best_updates,
        }
    }
}

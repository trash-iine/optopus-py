use optopus::heuristic::{NUM_CONTEXT_FEATURES, NUM_PERTURBATION_TYPES};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::result::RunReport;
use crate::runner::{self, HeuristicKind, HeuristicSpec};
use crate::stop_condition::StopCondition;

/// Rejects a parameter that must lie within `[0, 1]`.
fn check_unit_interval(name: &str, value: f64) -> PyResult<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be within [0.0, 1.0], got {value}"
        )));
    }
    Ok(())
}

/// Rejects a parameter that must be strictly positive.
fn check_positive(name: &str, value: f64) -> PyResult<()> {
    if value <= 0.0 || value.is_nan() {
        return Err(PyValueError::new_err(format!(
            "'{name}' must be greater than 0, got {value}"
        )));
    }
    Ok(())
}

/// Greedy best-improving local search; halts at a local optimum.
///
/// Args:
///     neighbor (str): Neighborhood move, ``"Flip"`` (flip a single
///         variable/vertex) or ``"Swap"`` (swap the values of two
///         variables/vertices). ``"Swap"`` preserves the number of set bits, so
///         it explores a constrained neighborhood; ``"Flip"`` is the usual
///         default. An unknown value raises ``ValueError`` when ``run`` is called.
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct LocalSearch {
    neighbor: String,
    stop: StopCondition,
}

impl LocalSearch {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::LocalSearch,
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl LocalSearch {
    #[new]
    fn new(neighbor: String, stop: StopCondition) -> Self {
        Self { neighbor, stop }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not valid for ``problem``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a problem instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Simulated annealing with ``exp(-Δ/T)`` acceptance and multiplicative cooling.
///
/// Args:
///     neighbor (str): Neighborhood move, ``"Flip"`` or ``"Swap"`` (see
///         ``LocalSearch`` for the difference).
///     initial_temperature (float): Starting temperature ``T``.
///     cooling_rate (float): Multiplicative cooling factor applied each step
///         (e.g. ``0.99``).
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct SimulatedAnnealing {
    neighbor: String,
    initial_temperature: f64,
    cooling_rate: f64,
    stop: StopCondition,
}

impl SimulatedAnnealing {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::SimulatedAnnealing {
                initial_temperature: self.initial_temperature,
                cooling_rate: self.cooling_rate,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl SimulatedAnnealing {
    #[new]
    fn new(
        neighbor: String,
        initial_temperature: f64,
        cooling_rate: f64,
        stop: StopCondition,
    ) -> Self {
        Self {
            neighbor,
            initial_temperature,
            cooling_rate,
            stop,
        }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not valid for ``problem``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a problem instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Tabu search; best non-tabu neighbor with aspiration on global-best improvement.
///
/// Args:
///     neighbor (str): Neighborhood move, ``"Flip"`` or ``"Swap"`` (see
///         ``LocalSearch`` for the difference).
///     tabu_tenure (tuple[int, int]): Tabu tenure range ``(min, max)``.
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct TabuSearch {
    neighbor: String,
    tabu_tenure: (u64, u64),
    stop: StopCondition,
}

impl TabuSearch {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::TabuSearch {
                tabu_tenure: self.tabu_tenure,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl TabuSearch {
    #[new]
    fn new(neighbor: String, tabu_tenure: (u64, u64), stop: StopCondition) -> Self {
        Self {
            neighbor,
            tabu_tenure,
            stop,
        }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not valid for ``problem``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a problem instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Late acceptance hill climbing; accepts a move no worse than the objective
/// ``history_length`` steps ago.
///
/// Args:
///     neighbor (str): Neighborhood move, ``"Flip"`` or ``"Swap"`` (see
///         ``LocalSearch`` for the difference).
///     history_length (int): Length of the acceptance history (``>= 1``).
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct LateAcceptanceHillClimbing {
    neighbor: String,
    history_length: usize,
    stop: StopCondition,
}

impl LateAcceptanceHillClimbing {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::LateAcceptance {
                history_length: self.history_length,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl LateAcceptanceHillClimbing {
    #[new]
    fn new(neighbor: String, history_length: usize, stop: StopCondition) -> Self {
        Self {
            neighbor,
            history_length,
            stop,
        }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not valid for ``problem``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a problem instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Random walk: at each step, applies a uniformly chosen neighbor move with no acceptance
/// criterion. Useful as a baseline, and as a shake step for ``VariableNeighborhoodSearch``.
///
/// Args:
///     neighbor (str): Neighborhood move name (problem-dependent, e.g. ``"Flip"``).
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct RandomWalk {
    neighbor: String,
    stop: StopCondition,
}

impl RandomWalk {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::RandomWalk,
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl RandomWalk {
    #[new]
    fn new(neighbor: String, stop: StopCondition) -> Self {
        Self { neighbor, stop }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Bang-bang simulated annealing: a simulated-annealing variant whose temperature oscillates
/// between ``min_wave_threshold`` (cool floor) and ``max_wave_threshold`` (hot ceiling),
/// reversing direction whenever a threshold is crossed.
///
/// Args:
///     neighbor (str): Neighborhood move name (problem-dependent).
///     initial_temperature (float): Starting temperature ``T``.
///     cooling_rate (float): Multiplicative cooling factor applied each step (e.g. ``0.99``).
///     min_wave_threshold (float): Cool floor; reaching it flips into a reheating phase.
///     max_wave_threshold (float): Hot ceiling; reaching it flips back into a cooling phase.
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct BangBangSimulatedAnnealing {
    neighbor: String,
    initial_temperature: f64,
    cooling_rate: f64,
    min_wave_threshold: f64,
    max_wave_threshold: f64,
    stop: StopCondition,
}

impl BangBangSimulatedAnnealing {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::BangBangSimulatedAnnealing {
                initial_temperature: self.initial_temperature,
                cooling_rate: self.cooling_rate,
                min_wave_threshold: self.min_wave_threshold,
                max_wave_threshold: self.max_wave_threshold,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl BangBangSimulatedAnnealing {
    #[new]
    fn new(
        neighbor: String,
        initial_temperature: f64,
        cooling_rate: f64,
        min_wave_threshold: f64,
        max_wave_threshold: f64,
        stop: StopCondition,
    ) -> Self {
        Self {
            neighbor,
            initial_temperature,
            cooling_rate,
            min_wave_threshold,
            max_wave_threshold,
            stop,
        }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Beam search: maintains a beam of ``beam_width`` candidate solutions; at each step expands
/// every candidate's neighborhood and keeps the top ``beam_width`` by quality.
///
/// Args:
///     neighbor (str): Neighborhood move name (problem-dependent).
///     beam_width (int): Beam width (``>= 1``).
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct BeamSearch {
    neighbor: String,
    beam_width: usize,
    stop: StopCondition,
}

impl BeamSearch {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::BeamSearch {
                beam_width: self.beam_width,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl BeamSearch {
    #[new]
    fn new(neighbor: String, beam_width: usize, stop: StopCondition) -> PyResult<Self> {
        if beam_width == 0 {
            return Err(PyValueError::new_err("'beam_width' must be at least 1"));
        }
        Ok(Self {
            neighbor,
            beam_width,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Variable neighborhood search: alternates an intensifying ``search`` heuristic with
/// increasingly disruptive ``shakes``. Each shake escapes the current local optimum; the
/// shake index resets to 0 whenever the search finds an improvement and advances otherwise.
///
/// The steps are ordinary heuristic instances, so each may use its own neighborhood and its
/// own stopping criterion. Problem-specific heuristics work as steps too, as long as the
/// problem matches.
///
/// Args:
///     search (object): Heuristic applied after every shake to intensify the search
///         (e.g. ``LocalSearch``).
///     shakes (list[object]): Perturbation heuristics, ordered from weakest to strongest
///         (e.g. ``RandomWalk`` instances with growing iteration counts). Must not be empty.
///     stop (StopCondition): Stopping criterion for the outer loop.
#[pyclass(module = "optopus")]
pub struct VariableNeighborhoodSearch {
    search: Py<PyAny>,
    shakes: Vec<Py<PyAny>>,
    stop: StopCondition,
}

impl VariableNeighborhoodSearch {
    pub(crate) fn to_spec(&self, py: Python<'_>) -> PyResult<HeuristicSpec> {
        let search = runner::spec_from_py(py, self.search.bind(py))?;
        let shakes = self
            .shakes
            .iter()
            .map(|shake| runner::spec_from_py(py, shake.bind(py)))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(HeuristicSpec {
            kind: HeuristicKind::VariableNeighborhoodSearch {
                search: Box::new(search),
                shakes,
            },
            // The nested steps carry their own neighborhoods.
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl VariableNeighborhoodSearch {
    #[new]
    fn new(search: Py<PyAny>, shakes: Vec<Py<PyAny>>, stop: StopCondition) -> PyResult<Self> {
        if shakes.is_empty() {
            return Err(PyValueError::new_err(
                "'shakes' must contain at least one heuristic",
            ));
        }
        Ok(Self {
            search,
            shakes,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem: The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If a step's ``neighbor`` is not valid for ``problem``, a step is a
    ///         problem-specific heuristic for a different problem, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a problem instance, or a step is not a heuristic.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// WalkSAT/SKC: focused local search for SAT. Each step picks an unsatisfied clause and
/// flips either its lowest-break variable or, with probability ``noise``, a random one.
///
/// Only applies to ``Sat`` problems. Terminates early once every clause is satisfied.
///
/// Args:
///     stop (StopCondition): Stopping criterion.
///     noise (float): Random-walk probability in ``[0, 1]``. Defaults to 0.3.
///     adaptive (bool): Adapt ``noise`` during the search following Hoos (2002).
///         Defaults to False.
#[pyclass(module = "optopus")]
pub struct WalkSat {
    noise: f64,
    adaptive: bool,
    stop: StopCondition,
}

impl WalkSat {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::WalkSat {
                noise: self.noise,
                adaptive: self.adaptive,
            },
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl WalkSat {
    #[new]
    #[pyo3(signature = (stop, noise=0.3, adaptive=false))]
    fn new(stop: StopCondition, noise: f64, adaptive: bool) -> PyResult<Self> {
        check_unit_interval("noise", noise)?;
        Ok(Self {
            noise,
            adaptive,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (Sat): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``problem`` is not a ``Sat`` instance, or ``runs`` is 0.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Population annealing for MaxCut: evolves a population of replicas along an increasing
/// inverse-temperature schedule, resampling them in proportion to their Boltzmann weight
/// and equilibrating each with Metropolis sweeps.
///
/// Only applies to ``MaxCut`` problems.
///
/// Args:
///     population_size (int): Number of replicas (``>= 2``).
///     stop (StopCondition): Stopping criterion. One iteration advances the whole
///         population by ``sweeps_per_step`` sweeps.
///     initial_beta (float): Starting inverse temperature ``β`` (``> 0``). Defaults to 0.1.
///     delta_beta (float): Increment added to ``β`` at each step (``> 0``). Defaults to 0.02.
///     sweeps_per_step (int): Metropolis sweeps per replica per step (``>= 1``).
///         Defaults to 50.
///     reset_period (int): Steps between annealing-schedule resets; 0 disables resetting.
///         Defaults to 400.
///     cluster_moves (bool): Enable non-local iso-site cluster moves. Defaults to True.
#[pyclass(module = "optopus")]
pub struct PopulationAnnealing {
    population_size: usize,
    initial_beta: f64,
    delta_beta: f64,
    sweeps_per_step: usize,
    reset_period: Option<usize>,
    cluster_moves: bool,
    stop: StopCondition,
}

impl PopulationAnnealing {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::PopulationAnnealing {
                population_size: self.population_size,
                initial_beta: self.initial_beta,
                delta_beta: self.delta_beta,
                sweeps_per_step: self.sweeps_per_step,
                reset_period: self.reset_period,
                cluster_moves: self.cluster_moves,
            },
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl PopulationAnnealing {
    #[new]
    #[pyo3(signature = (
        population_size,
        stop,
        initial_beta=0.1,
        delta_beta=0.02,
        sweeps_per_step=50,
        reset_period=400,
        cluster_moves=true,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        population_size: usize,
        stop: StopCondition,
        initial_beta: f64,
        delta_beta: f64,
        sweeps_per_step: usize,
        reset_period: usize,
        cluster_moves: bool,
    ) -> PyResult<Self> {
        if population_size < 2 {
            return Err(PyValueError::new_err(
                "'population_size' must be at least 2",
            ));
        }
        check_positive("initial_beta", initial_beta)?;
        check_positive("delta_beta", delta_beta)?;
        if sweeps_per_step == 0 {
            return Err(PyValueError::new_err(
                "'sweeps_per_step' must be at least 1",
            ));
        }
        Ok(Self {
            population_size,
            initial_beta,
            delta_beta,
            sweeps_per_step,
            reset_period: (reset_period > 0).then_some(reset_period),
            cluster_moves,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (MaxCut): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``problem`` is not a ``MaxCut`` instance, or ``runs`` is 0.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Breakout local search for MaxCut: tabu descent interleaved with adaptive perturbations
/// whose strength grows the longer the search stagnates.
///
/// Only applies to ``MaxCut`` problems.
///
/// Args:
///     tabu_tenure (tuple[int, int]): Tabu tenure range ``(min, max)``.
///     t (int): Stagnation threshold; beyond it the perturbation turns strongly diversifying.
///     l0 (int): Base perturbation strength (number of moves), ``>= 1``.
///     p0 (float): Floor on the probability of a directed (rather than random) perturbation.
///     q (float): Decay rate of that probability as stagnation grows.
///     stop (StopCondition): Stopping criterion.
///     plateau_prob (float): Probability of taking a zero-gain plateau move, in ``[0, 1]``.
///         Defaults to 0.0, which reproduces the original algorithm.
#[pyclass(module = "optopus")]
pub struct BreakoutLocalSearch {
    tabu_tenure: (u64, u64),
    t: u64,
    l0: u64,
    p0: f64,
    q: f64,
    plateau_prob: f64,
    stop: StopCondition,
}

impl BreakoutLocalSearch {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::BreakoutLocalSearch {
                tabu_tenure: self.tabu_tenure,
                t: self.t,
                l0: self.l0,
                p0: self.p0,
                q: self.q,
                plateau_prob: self.plateau_prob,
            },
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl BreakoutLocalSearch {
    #[new]
    #[pyo3(signature = (tabu_tenure, t, l0, p0, q, stop, plateau_prob=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        tabu_tenure: (u64, u64),
        t: u64,
        l0: u64,
        p0: f64,
        q: f64,
        stop: StopCondition,
        plateau_prob: f64,
    ) -> PyResult<Self> {
        if l0 == 0 {
            return Err(PyValueError::new_err("'l0' must be at least 1"));
        }
        check_unit_interval("plateau_prob", plateau_prob)?;
        Ok(Self {
            tabu_tenure,
            t,
            l0,
            p0,
            q,
            plateau_prob,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (MaxCut): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``problem`` is not a ``MaxCut`` instance, or ``runs`` is 0.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Breakout local search for MaxCut whose perturbation is chosen by a contextual softmax
/// bandit instead of a fixed rule. The bandit learns online which (perturbation type,
/// strength) pair pays off in the current search context, and keeps its weights across
/// restarts within a single run.
///
/// Only applies to ``MaxCut`` problems.
///
/// Args:
///     tabu_tenure (tuple[int, int]): Tabu tenure range ``(min, max)``.
///     t (int): Stagnation threshold used to build the context features.
///     l0 (int): Base perturbation strength (number of moves), ``>= 1``.
///     stop (StopCondition): Stopping criterion.
///     strength_bins (list[float]): Multipliers on ``l0`` the bandit can choose from; every
///         entry must be positive. Defaults to ``[1.0, 2.0, 4.0]``.
///     learning_rate (float): Policy-gradient step size (``>= 0``; 0 freezes learning).
///         Defaults to 0.1.
///     softmax_temperature (float): Softmax temperature over action scores (``> 0``).
///         Defaults to 1.0.
///     exploration (float): Probability of a uniformly random action, in ``[0, 1]``.
///         Defaults to 0.05.
///     policy_weights (list[float] | None): Pre-trained weights, flattened row-major with
///         ``5 * len(strength_bins) * 8`` entries. Defaults to None (start from zero).
#[pyclass(module = "optopus")]
pub struct RlBreakoutLocalSearch {
    tabu_tenure: (u64, u64),
    t: u64,
    l0: u64,
    strength_bins: Vec<f64>,
    learning_rate: f64,
    softmax_temperature: f64,
    exploration: f64,
    policy_weights: Option<Vec<f64>>,
    stop: StopCondition,
}

impl RlBreakoutLocalSearch {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::RlBreakoutLocalSearch {
                tabu_tenure: self.tabu_tenure,
                t: self.t,
                l0: self.l0,
                strength_bins: self.strength_bins.clone(),
                learning_rate: self.learning_rate,
                softmax_temperature: self.softmax_temperature,
                exploration: self.exploration,
                policy_weights: self.policy_weights.clone(),
            },
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl RlBreakoutLocalSearch {
    #[new]
    #[pyo3(signature = (
        tabu_tenure,
        t,
        l0,
        stop,
        strength_bins=vec![1.0, 2.0, 4.0],
        learning_rate=0.1,
        softmax_temperature=1.0,
        exploration=0.05,
        policy_weights=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        tabu_tenure: (u64, u64),
        t: u64,
        l0: u64,
        stop: StopCondition,
        strength_bins: Vec<f64>,
        learning_rate: f64,
        softmax_temperature: f64,
        exploration: f64,
        policy_weights: Option<Vec<f64>>,
    ) -> PyResult<Self> {
        if l0 == 0 {
            return Err(PyValueError::new_err("'l0' must be at least 1"));
        }
        if strength_bins.is_empty() {
            return Err(PyValueError::new_err(
                "'strength_bins' must not be empty (e.g. [1.0, 2.0, 4.0])",
            ));
        }
        for bin in &strength_bins {
            check_positive("strength_bins", *bin)?;
        }
        if learning_rate < 0.0 {
            return Err(PyValueError::new_err(format!(
                "'learning_rate' must not be negative, got {learning_rate}"
            )));
        }
        check_positive("softmax_temperature", softmax_temperature)?;
        check_unit_interval("exploration", exploration)?;
        if let Some(weights) = &policy_weights {
            let expected = NUM_PERTURBATION_TYPES * strength_bins.len() * NUM_CONTEXT_FEATURES;
            if weights.len() != expected {
                return Err(PyValueError::new_err(format!(
                    "'policy_weights' must have {expected} entries for {} strength bins, got {}",
                    strength_bins.len(),
                    weights.len()
                )));
            }
        }
        Ok(Self {
            tabu_tenure,
            t,
            l0,
            strength_bins,
            learning_rate,
            softmax_temperature,
            exploration,
            policy_weights,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (MaxCut): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``problem`` is not a ``MaxCut`` instance, or ``runs`` is 0.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

/// Lin-Kernighan-Helsgaun for the Euclidean TSP: variable-depth edge exchange restricted to
/// each city's ``num_neighbors`` nearest candidates.
///
/// Only applies to ``TspWithCoordinates`` problems.
///
/// Args:
///     stop (StopCondition): Stopping criterion.
///     num_neighbors (int): Candidate neighbors kept per city. Defaults to 5.
///     max_depth (int): Maximum depth of the sequential edge exchange. Defaults to 5.
#[pyclass(module = "optopus")]
pub struct LinKernighanHelsgaun {
    num_neighbors: usize,
    max_depth: usize,
    stop: StopCondition,
}

impl LinKernighanHelsgaun {
    pub(crate) fn to_spec(&self, _py: Python<'_>) -> PyResult<HeuristicSpec> {
        Ok(HeuristicSpec {
            kind: HeuristicKind::LinKernighanHelsgaun {
                num_neighbors: self.num_neighbors,
                max_depth: self.max_depth,
            },
            neighbor: String::new(),
            stop: self.stop.clone(),
        })
    }
}

#[pymethods]
impl LinKernighanHelsgaun {
    #[new]
    #[pyo3(signature = (stop, num_neighbors=5, max_depth=5))]
    fn new(stop: StopCondition, num_neighbors: usize, max_depth: usize) -> PyResult<Self> {
        if num_neighbors == 0 {
            return Err(PyValueError::new_err("'num_neighbors' must be at least 1"));
        }
        if max_depth == 0 {
            return Err(PyValueError::new_err("'max_depth' must be at least 1"));
        }
        Ok(Self {
            num_neighbors,
            max_depth,
            stop,
        })
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (TspWithCoordinates): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``problem`` is not a ``TspWithCoordinates`` instance, or ``runs`` is 0.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        py: Python<'_>,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        runner::solve(problem, &self.to_spec(py)?, runs, seed)
    }
}

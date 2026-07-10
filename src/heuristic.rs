use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::result::RunReport;
use crate::runner::{self, HeuristicKind, HeuristicSpec};
use crate::stop_condition::StopCondition;

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

#[pymethods]
impl LocalSearch {
    #[new]
    fn new(neighbor: String, stop: StopCondition) -> Self {
        Self { neighbor, stop }
    }

    /// Run the heuristic on a problem and return an aggregated report.
    ///
    /// Args:
    ///     problem (MaxCut | Qubo): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not ``"Flip"`` or ``"Swap"``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a ``MaxCut`` or ``Qubo`` instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::LocalSearch,
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
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
    ///     problem (MaxCut | Qubo): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not ``"Flip"`` or ``"Swap"``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a ``MaxCut`` or ``Qubo`` instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::SimulatedAnnealing {
                initial_temperature: self.initial_temperature,
                cooling_rate: self.cooling_rate,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
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
    ///     problem (MaxCut | Qubo): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not ``"Flip"`` or ``"Swap"``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a ``MaxCut`` or ``Qubo`` instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::TabuSearch {
                tabu_tenure: self.tabu_tenure,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
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
    ///     problem (MaxCut | Qubo): The problem instance to solve.
    ///     runs (int): Number of independent runs to perform. Defaults to 1.
    ///     seed (int | None): Master seed. When set, runs are deterministic
    ///         (run 0 uses ``seed`` directly).
    ///
    /// Returns:
    ///     RunReport: Aggregated statistics over all runs.
    ///
    /// Raises:
    ///     ValueError: If ``neighbor`` is not ``"Flip"`` or ``"Swap"``, or ``runs`` is 0.
    ///     TypeError: If ``problem`` is not a ``MaxCut`` or ``Qubo`` instance.
    #[pyo3(signature = (problem, runs=1, seed=None))]
    fn run(
        &self,
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::LateAcceptance {
                history_length: self.history_length,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
    }
}

/// Random walk: at each step, applies a uniformly chosen neighbor move with no acceptance
/// criterion. Useful as a baseline.
///
/// Args:
///     neighbor (str): Neighborhood move name (problem-dependent, e.g. ``"Flip"``).
///     stop (StopCondition): Stopping criterion.
#[pyclass(module = "optopus")]
pub struct RandomWalk {
    neighbor: String,
    stop: StopCondition,
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
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::RandomWalk,
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
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
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::BangBangSimulatedAnnealing {
                initial_temperature: self.initial_temperature,
                cooling_rate: self.cooling_rate,
                min_wave_threshold: self.min_wave_threshold,
                max_wave_threshold: self.max_wave_threshold,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
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
        problem: &Bound<'_, PyAny>,
        runs: usize,
        seed: Option<u64>,
    ) -> PyResult<RunReport> {
        let spec = HeuristicSpec {
            kind: HeuristicKind::BeamSearch {
                beam_width: self.beam_width,
            },
            neighbor: self.neighbor.clone(),
            stop: self.stop.clone(),
        };
        runner::solve(problem, &spec, runs, seed)
    }
}

use std::time::Duration;

use optopus::prelude::StopCondition as OptStopCondition;
use pyo3::prelude::*;

/// Stopping criteria for a heuristic run.
///
/// A run stops as soon as **any** of the set conditions is met. A parameter left
/// as ``None`` is not used as a stopping criterion; at least one should be set.
///
/// Args:
///     max_iteration (int | None): Stop after this many iterations.
///     max_duration_secs (float | None): Stop after this wall-clock duration, in seconds.
///     max_failed_update (int | None): Stop after this many consecutive iterations
///         without improving the best solution.
#[pyclass(module = "optopus", from_py_object)]
#[derive(Clone)]
pub struct StopCondition {
    pub max_iteration: Option<u64>,
    pub max_duration_secs: Option<f64>,
    pub max_failed_update: Option<u64>,
}

#[pymethods]
impl StopCondition {
    #[new]
    #[pyo3(signature = (max_iteration=None, max_duration_secs=None, max_failed_update=None))]
    fn new(
        max_iteration: Option<u64>,
        max_duration_secs: Option<f64>,
        max_failed_update: Option<u64>,
    ) -> Self {
        Self {
            max_iteration,
            max_duration_secs,
            max_failed_update,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "StopCondition(max_iteration={:?}, max_duration_secs={:?}, max_failed_update={:?})",
            self.max_iteration, self.max_duration_secs, self.max_failed_update
        )
    }
}

impl StopCondition {
    pub fn to_opt(&self) -> OptStopCondition {
        OptStopCondition::new(
            self.max_iteration,
            self.max_duration_secs.map(Duration::from_secs_f64),
            self.max_failed_update,
        )
    }
}

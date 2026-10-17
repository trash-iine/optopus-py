//! The glue behind the `BranchAndBound` class: the relaxations a Python caller names, a
//! relaxation written in Python, and the per-run outcome a run reports.
//!
//! `run_all` only sees a `Box<dyn Heuristic<P>>`, so the search's own answers (whether it
//! proved its best optimal, and the bound still open) cannot be read off the box after a run.
//! [`Reported`] records them in a thread-local when its run returns, and the runner takes them
//! from there. A run stays on the thread that started it, so the slot is never shared.

use std::cell::RefCell;

use optopus::building_blocks::representation::IntVars;
use optopus::error::OptError;
use optopus::prelude::{BranchAndBound, Evaluable, Heuristic, SearchState, StopCondition};
use optopus::problem::EigenvalueRelaxation as OptEigenvalueRelaxation;
use optopus::trait_defs::{BranchSpace, Relaxation};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::validate::check_at_least;

/// The relaxation a `BranchAndBound` prunes by, as the caller named it.
pub enum RelaxationSpec {
    /// The problem's default: `Binary` on the binary problems, `Interval` on `Formula`.
    Default,
    Binary,
    /// The certified eigenvalue bound on Max Cut, with its limits.
    Eigenvalue {
        max_iterations: usize,
        lanczos_steps: usize,
        certify_limit: usize,
    },
    Interval,
    /// A Python callable taking the node's ranges and returning a bound.
    Python(Py<PyAny>),
}

impl RelaxationSpec {
    /// Parses the `relaxation` argument: `None`, a name, or a callable.
    pub fn from_py(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let Some(obj) = obj else {
            return Ok(Self::Default);
        };
        if let Ok(eigen) = obj.extract::<PyRef<'_, EigenvalueRelaxation>>() {
            return Ok(Self::Eigenvalue {
                max_iterations: eigen.max_iterations,
                lanczos_steps: eigen.lanczos_steps,
                certify_limit: eigen.certify_limit,
            });
        }
        if let Ok(name) = obj.extract::<String>() {
            return match name.as_str() {
                "Binary" => Ok(Self::Binary),
                "Eigenvalue" => Ok(Self::Eigenvalue {
                    max_iterations: OptEigenvalueRelaxation::MAX_ITERATIONS,
                    lanczos_steps: OptEigenvalueRelaxation::LANCZOS_STEPS,
                    certify_limit: OptEigenvalueRelaxation::CERTIFY_LIMIT,
                }),
                "Interval" => Ok(Self::Interval),
                other => Err(PyValueError::new_err(format!(
                    "invalid relaxation '{other}' (use 'Binary', 'Eigenvalue', 'Interval', an \
                     EigenvalueRelaxation or a callable)"
                ))),
            };
        }
        if obj.is_callable() {
            return Ok(Self::Python(obj.clone().unbind()));
        }
        Err(PyTypeError::new_err(
            "'relaxation' must be None, a relaxation name, an EigenvalueRelaxation or a callable",
        ))
    }

    /// The error for a relaxation named on a problem it does not apply to.
    pub fn unsupported(&self, problem: &str) -> String {
        let (name, only) = match self {
            Self::Binary => ("Binary", "MaxCut, Qubo, Sat and VertexCover"),
            Self::Eigenvalue { .. } => ("Eigenvalue", "MaxCut"),
            Self::Interval => ("Interval", "Formula"),
            Self::Default | Self::Python(_) => unreachable!("applies to every problem"),
        };
        format!("relaxation '{name}' is only available for {only}, not {problem}")
    }
}

/// The certified eigenvalue bound for ``BranchAndBound`` on ``MaxCut``, with its limits set.
///
/// ``relaxation="Eigenvalue"`` is this relaxation with the defaults. The bound comes from the
/// largest eigenvalue of the node's graph Laplacian, shifted by a vector tuned by subgradient
/// steps, and is certified by a Cholesky factorization; a node whose folded graph is too large
/// to certify falls back to the sum of its positive weights.
///
/// Args:
///     max_iterations (int): Subgradient steps per bound, ``>= 1``. More steps tighten the
///         bound at a higher cost per node. Defaults to 100.
///     lanczos_steps (int): Lanczos steps per eigenvalue estimate, ``>= 1``. Defaults to 30.
///     certify_limit (int): The largest folded graph, in vertices, the bound is certified on.
///         Defaults to 3000.
///
/// Raises:
///     ValueError: If ``max_iterations`` or ``lanczos_steps`` is 0.
///
/// Example:
///     >>> BranchAndBound(LocalSearch("Flip", stop), relaxation=EigenvalueRelaxation(max_iterations=200))
#[pyclass(module = "optopus", frozen)]
pub struct EigenvalueRelaxation {
    max_iterations: usize,
    lanczos_steps: usize,
    certify_limit: usize,
}

#[pymethods]
impl EigenvalueRelaxation {
    #[new]
    #[pyo3(signature = (
        max_iterations=OptEigenvalueRelaxation::MAX_ITERATIONS,
        lanczos_steps=OptEigenvalueRelaxation::LANCZOS_STEPS,
        certify_limit=OptEigenvalueRelaxation::CERTIFY_LIMIT,
    ),
    text_signature = "(max_iterations=100, lanczos_steps=30, certify_limit=3000)"
    )]
    fn new(max_iterations: usize, lanczos_steps: usize, certify_limit: usize) -> PyResult<Self> {
        check_at_least("max_iterations", max_iterations, 1)?;
        check_at_least("lanczos_steps", lanczos_steps, 1)?;
        Ok(Self {
            max_iterations,
            lanczos_steps,
            certify_limit,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "EigenvalueRelaxation(max_iterations={}, lanczos_steps={}, certify_limit={})",
            self.max_iterations, self.lanczos_steps, self.certify_limit
        )
    }
}

/// What a `BranchAndBound` run found out beyond its best solution.
pub struct Outcome {
    pub proven_optimal: bool,
    /// The best objective an assignment not yet ruled out could reach, in the problem's
    /// direction.
    pub dual_bound: Evaluable<f64>,
}

thread_local! {
    static OUTCOME: RefCell<Option<Outcome>> = const { RefCell::new(None) };
    static ERROR: RefCell<Option<PyErr>> = const { RefCell::new(None) };
}

/// Takes the outcome the last `BranchAndBound` run recorded, leaving none.
pub fn take_outcome() -> Option<Outcome> {
    OUTCOME.with(|o| o.borrow_mut().take())
}

/// Takes the first error a `BranchAndBound` run recorded, leaving none.
pub fn take_error() -> Option<PyErr> {
    ERROR.with(|e| e.borrow_mut().take())
}

fn failed() -> bool {
    ERROR.with(|e| e.borrow().is_some())
}

/// Keeps the first error, so the one that stopped the search is the one raised.
fn record(err: PyErr) {
    ERROR.with(|e| {
        e.borrow_mut().get_or_insert(err);
    });
}

/// A `BranchAndBound` that records its [`Outcome`] when its run returns, and turns a
/// configuration error the core reports into a `ValueError` the runner raises.
pub struct Reported<P: BranchSpace, R> {
    inner: BranchAndBound<P, R>,
}

impl<P: BranchSpace, R: Relaxation<P>> Heuristic<P> for Reported<P, R> {
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn stop_condition(&self) -> &StopCondition {
        self.inner.stop_condition()
    }

    /// Done as the search is, or as soon as a Python relaxation has raised. The outcome is
    /// recorded here as well as in `run`, since the `Guarded` around a heuristic on a Python
    /// problem drives the loop through `run_once` and `is_done` and never calls `run`.
    fn is_done<'a>(&self, state: &SearchState<'a, P>) -> bool {
        let done = failed() || self.inner.is_done(state);
        if done {
            self.record_outcome(state);
        }
        done
    }

    fn run_once<'a>(&mut self, state: &mut SearchState<'a, P>) -> Result<(), OptError> {
        self.inner.run_once(state)
    }

    fn run<'a>(&mut self, state: &mut SearchState<'a, P>) -> Result<(), OptError> {
        self.clear();
        let mut result = Ok(());
        while !self.is_done(state) {
            result = self.run_once(state);
            if result.is_err() {
                break;
            }
        }
        if let Err(OptError::Config(msg)) = &result {
            record(PyValueError::new_err(msg.clone()));
        }
        self.record_outcome(state);
        result
    }
}

impl<P: BranchSpace, R: Relaxation<P>> Reported<P, R> {
    fn record_outcome(&self, state: &SearchState<'_, P>) {
        OUTCOME.with(|o| {
            *o.borrow_mut() = Some(Outcome {
                proven_optimal: self.inner.is_proven_optimal(&state.best_solution),
                dual_bound: self.inner.dual_bound(&state.best_solution),
            });
        });
    }
}

/// A `BranchAndBound` searching each node with `heuristic` and pruning by `relaxation`.
pub fn build<P, R>(
    stop: &crate::stop_condition::StopCondition,
    heuristic: Box<dyn Heuristic<P>>,
    relaxation: R,
) -> Box<dyn Heuristic<P>>
where
    P: BranchSpace + 'static,
    R: Relaxation<P> + 'static,
{
    Box::new(Reported {
        inner: BranchAndBound::new(stop.to_opt(), heuristic, relaxation),
    })
}

/// A relaxation written in Python: a callable taking the node's ranges as a list of inclusive
/// `(lower, upper)` pairs and returning a bound on the objective, in the problem's direction.
pub struct PyRelaxation {
    func: Py<PyAny>,
    maximize: bool,
}

impl PyRelaxation {
    pub fn new(func: Py<PyAny>, maximize: bool) -> Self {
        Self { func, maximize }
    }

    /// The bound the callable returns, or `None` once it has raised or returned something
    /// that is not a bound.
    fn call(&self, vars: &IntVars) -> Option<f64> {
        if failed() {
            return None;
        }
        let result = Python::attach(|py| -> PyResult<f64> {
            let ranges: Vec<(i64, i64)> = vars.iter().map(|v| (v.lower(), v.upper())).collect();
            let value: f64 = self
                .func
                .bind(py)
                .call1((PyList::new(py, ranges)?,))?
                .extract()?;
            if value.is_nan() {
                return Err(PyValueError::new_err(
                    "the relaxation returned NaN, which bounds nothing",
                ));
            }
            Ok(value)
        });
        result.map_err(record).ok()
    }
}

impl<P> Relaxation<P> for PyRelaxation {
    fn bound(&mut self, _prob: &P, vars: &IntVars) -> Evaluable<f64> {
        // After an error every node is pruned, so the search ends at once and the error is
        // raised.
        if self.maximize {
            Evaluable::Maximize(self.call(vars).unwrap_or(f64::NEG_INFINITY))
        } else {
            Evaluable::Minimize(self.call(vars).unwrap_or(f64::INFINITY))
        }
    }
}

/// The outcome's dual bound on the report's scale: as the objective on a minimized problem,
/// negated from the minimized value on a maximized one, which is how a `Formula` reports.
pub fn dual_on_report_scale(dual: Evaluable<f64>, minimize: bool) -> f64 {
    let m = dual.minimized();
    if minimize { m } else { 0.0 - m }
}

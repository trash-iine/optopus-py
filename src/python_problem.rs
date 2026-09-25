//! Problems written in Python.
//!
//! Any Python object that follows the protocol below can be handed to the
//! generic heuristics. The Rust side wraps it in [`PyProblem`], its solutions
//! in [`PySolution`] and its moves in [`PyMove`], and implements the optopus
//! traits for those wrappers, so the heuristics run unchanged.
//!
//! The problem object provides
//!
//! - `minimize` (bool), the direction of `objective`,
//! - `new_solution(rng)`, a random starting solution,
//! - `objective(solution)`, a float,
//! - `neighborhoods`, a mapping from a name to a neighborhood object.
//!
//! A neighborhood object provides
//!
//! - `neighbors(problem, solution)`, an iterable of moves,
//! - `apply(problem, solution, move)`, the new solution (the old one is left as is),
//! - optionally `delta(problem, solution, move)`, `objective` after minus before,
//! - optionally `random_neighbor(problem, solution, rng)`, one move or `None`,
//! - optionally `tabu_keys(problem, move)`, needed by `TabuSearch`.
//!
//! Solutions and moves are opaque to the Rust side. `rng` is a
//! `random.Random` seeded from the run's seed, so a seeded run reproduces.
//!
//! A Python exception inside any of these calls, or a pending signal such as
//! Ctrl-C, stops the run at the next step, and the exception is raised from
//! `run` once the heuristic has returned.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use optopus::common::{TabuKey, TabuMemory};
use optopus::error::OptError;
use optopus::prelude::{
    EnabledTabu, Evaluable, Evaluate, Heuristic, MoveToNeighbor, ProblemTrait, SearchState,
    StopCondition,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyMapping, PyTuple};
use rand::Rng;
use rand::rngs::SmallRng;

/// The most neighborhoods one Python problem can declare. Each one is a
/// separate monomorphization of [`PyMove`], so this bounds the code size.
pub const MAX_NEIGHBORHOODS: usize = 8;

/// A neighborhood of a Python problem, with its optional methods looked up once.
struct Neighborhood {
    name: String,
    obj: Py<PyAny>,
    has_delta: bool,
    has_random_neighbor: bool,
    has_tabu_keys: bool,
}

/// A problem implemented in Python.
pub struct PyProblem {
    obj: Py<PyAny>,
    minimize: bool,
    neighborhoods: Vec<Neighborhood>,
    /// The `random.Random` handed to the Python callbacks, reseeded from the
    /// run's RNG every time a run draws its initial solution.
    rng: Mutex<Option<Py<PyAny>>>,
    /// The first Python exception raised during the current run.
    error: Mutex<Option<PyErr>>,
    /// Whether moves read their `tabu_keys` when priced. Only a tabu search
    /// needs them, so the other heuristics skip the extra Python call.
    record_tabu: AtomicBool,
}

impl PyProblem {
    /// Checks `obj` against the protocol and reads its neighborhoods.
    pub fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        for method in ["new_solution", "objective"] {
            if !obj.hasattr(method)? {
                return Err(PyTypeError::new_err(format!(
                    "a Python problem needs a '{method}' method"
                )));
            }
        }
        let minimize: bool = obj
            .getattr("minimize")
            .map_err(|_| PyTypeError::new_err("a Python problem needs a 'minimize' attribute"))?
            .extract()?;
        let mapping = obj.getattr("neighborhoods").map_err(|_| {
            PyTypeError::new_err("a Python problem needs a 'neighborhoods' mapping")
        })?;
        let mapping = mapping.cast::<PyMapping>().map_err(|_| {
            PyTypeError::new_err("'neighborhoods' must be a mapping from name to neighborhood")
        })?;

        let mut neighborhoods = Vec::new();
        for item in mapping.items()?.iter() {
            let (name, nb): (String, Bound<'_, PyAny>) = item.extract()?;
            for method in ["neighbors", "apply"] {
                if !nb.hasattr(method)? {
                    return Err(PyTypeError::new_err(format!(
                        "neighborhood '{name}' needs a '{method}' method"
                    )));
                }
            }
            neighborhoods.push(Neighborhood {
                has_delta: nb.hasattr("delta")?,
                has_random_neighbor: nb.hasattr("random_neighbor")?,
                has_tabu_keys: nb.hasattr("tabu_keys")?,
                obj: nb.unbind(),
                name,
            });
        }
        if neighborhoods.is_empty() {
            return Err(PyValueError::new_err("'neighborhoods' must not be empty"));
        }
        if neighborhoods.len() > MAX_NEIGHBORHOODS {
            return Err(PyValueError::new_err(format!(
                "a Python problem can declare at most {MAX_NEIGHBORHOODS} neighborhoods"
            )));
        }

        Ok(Self {
            obj: obj.clone().unbind(),
            minimize,
            neighborhoods,
            rng: Mutex::new(None),
            error: Mutex::new(None),
            record_tabu: AtomicBool::new(false),
        })
    }

    pub fn minimize(&self) -> bool {
        self.minimize
    }

    /// The index of the neighborhood called `name`, for picking the [`PyMove`] type.
    pub fn neighborhood_index(&self, name: &str) -> Result<usize, String> {
        self.neighborhoods
            .iter()
            .position(|nb| nb.name == name)
            .ok_or_else(|| {
                let valid: Vec<_> = self
                    .neighborhoods
                    .iter()
                    .map(|nb| nb.name.as_str())
                    .collect();
                format!(
                    "invalid neighbor '{name}' for this problem (use {})",
                    valid.join(", ")
                )
            })
    }

    /// Makes the moves of every neighborhood carry their tabu keys, or fails
    /// when the neighborhood at `k` has no `tabu_keys`.
    pub fn enable_tabu(&self, k: usize) -> Result<(), String> {
        let nb = &self.neighborhoods[k];
        if nb.has_tabu_keys {
            self.record_tabu.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!(
                "TabuSearch needs a 'tabu_keys' method on neighborhood '{}'",
                nb.name
            ))
        }
    }

    /// Takes the exception the last run stopped on, if any.
    pub fn take_error(&self) -> Option<PyErr> {
        self.error.lock().unwrap().take()
    }

    fn failed(&self) -> bool {
        self.error.lock().unwrap().is_some()
    }

    /// Keeps the first exception of a run and drops later ones, which are
    /// usually consequences of it.
    fn record(&self, err: PyErr) {
        let mut slot = self.error.lock().unwrap();
        if slot.is_none() {
            *slot = Some(err);
        }
    }

    /// Runs `f`, recording its exception and returning `None` in its place.
    fn call<T>(&self, f: impl FnOnce(Python<'_>) -> PyResult<T>) -> Option<T> {
        if self.failed() {
            return None;
        }
        Python::attach(f).map_err(|e| self.record(e)).ok()
    }

    fn py_rng<'py>(&self, py: Python<'py>) -> Bound<'py, PyAny> {
        self.rng
            .lock()
            .unwrap()
            .as_ref()
            .expect("new_solution seeds the rng before any move is drawn")
            .bind(py)
            .clone()
    }

    fn objective(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<f64> {
        self.obj
            .bind(py)
            .call_method1("objective", (value,))?
            .extract()
    }

    fn solution(&self, py: Python<'_>, value: Bound<'_, PyAny>) -> PyResult<PySolution> {
        Ok(PySolution {
            objective: self.objective(py, &value)?,
            value: value.unbind(),
            minimize: self.minimize,
        })
    }
}

/// A solution of a [`PyProblem`], with its objective cached.
pub struct PySolution {
    pub value: Py<PyAny>,
    pub objective: f64,
    minimize: bool,
}

impl Clone for PySolution {
    /// Solutions are never mutated in place (`apply` returns a new one), so a
    /// clone shares the Python object.
    fn clone(&self) -> Self {
        Self {
            value: Python::attach(|py| self.value.clone_ref(py)),
            objective: self.objective,
            minimize: self.minimize,
        }
    }
}

impl Evaluate for PySolution {
    fn evaluate(&self) -> Evaluable<f64> {
        evaluable(self.minimize, self.objective)
    }
}

fn evaluable(minimize: bool, value: f64) -> Evaluable<f64> {
    if minimize {
        Evaluable::Minimize(value)
    } else {
        Evaluable::Maximize(value)
    }
}

impl ProblemTrait for PyProblem {
    type Solution = PySolution;

    fn new_solution(&self, rng: &mut impl Rng) -> PySolution {
        let seed: u64 = rng.random();
        let made = self.call(|py| {
            let py_rng = py.import("random")?.getattr("Random")?.call1((seed,))?;
            *self.rng.lock().unwrap() = Some(py_rng.clone().unbind());
            let value = self.obj.bind(py).call_method1("new_solution", (py_rng,))?;
            self.solution(py, value)
        });
        // The run stops before it looks at this placeholder, see `Guarded`.
        made.unwrap_or_else(|| PySolution {
            value: Python::attach(|py| py.None()),
            objective: f64::NAN,
            minimize: self.minimize,
        })
    }
}

/// A move in the neighborhood at index `K` of a [`PyProblem`], with its delta.
pub struct PyMove<const K: usize> {
    mv: Py<PyAny>,
    delta: f64,
    minimize: bool,
    /// Empty unless a tabu search is running, see [`PyProblem::enable_tabu`].
    tabu_keys: Vec<TabuKey>,
}

impl<const K: usize> Clone for PyMove<K> {
    fn clone(&self) -> Self {
        Self {
            mv: Python::attach(|py| self.mv.clone_ref(py)),
            delta: self.delta,
            minimize: self.minimize,
            tabu_keys: self.tabu_keys.clone(),
        }
    }
}

impl<const K: usize> Evaluate for PyMove<K> {
    fn evaluate(&self) -> Evaluable<f64> {
        evaluable(self.minimize, self.delta)
    }
}

impl<const K: usize> PyMove<K> {
    /// Prices `mv` against `sol`, with the neighborhood's `delta` when it has
    /// one and by applying it otherwise.
    fn priced(
        prob: &PyProblem,
        py: Python<'_>,
        sol: &PySolution,
        mv: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let nb = &prob.neighborhoods[K];
        let problem = prob.obj.bind(py);
        let delta = if nb.has_delta {
            nb.obj
                .bind(py)
                .call_method1("delta", (problem, sol.value.bind(py), &mv))?
                .extract()?
        } else {
            let next = nb
                .obj
                .bind(py)
                .call_method1("apply", (problem, sol.value.bind(py), &mv))?;
            prob.objective(py, &next)? - sol.objective
        };
        let tabu_keys = if nb.has_tabu_keys && prob.record_tabu.load(Ordering::Relaxed) {
            let keys = nb.obj.bind(py).call_method1("tabu_keys", (problem, &mv))?;
            // One key, or a list of them.
            if keys.cast::<PyList>().is_ok() {
                keys.try_iter()?
                    .map(|k| tabu_key(&k?))
                    .collect::<PyResult<_>>()?
            } else {
                vec![tabu_key(&keys)?]
            }
        } else {
            Vec::new()
        };
        Ok(Self {
            mv: mv.unbind(),
            delta,
            minimize: prob.minimize,
            tabu_keys,
        })
    }
}

/// Reads one tabu key, a non-negative int or a tuple of two or three of them.
fn tabu_key(key: &Bound<'_, PyAny>) -> PyResult<TabuKey> {
    let invalid = || {
        PyTypeError::new_err(
            "tabu_keys must return a non-negative int, a tuple of 2 or 3 of them, or a list of those",
        )
    };
    if let Ok(t) = key.cast::<PyTuple>() {
        let ix: Vec<usize> = t.extract().map_err(|_| invalid())?;
        return match ix[..] {
            [i, j] => Ok(TabuKey::Pair(i, j)),
            [i, j, k] => Ok(TabuKey::Triple(i, j, k)),
            _ => Err(invalid()),
        };
    }
    key.extract::<usize>()
        .map(TabuKey::Var)
        .map_err(|_| invalid())
}

impl<const K: usize> MoveToNeighbor<PyProblem> for PyMove<K> {
    fn apply_to_solution(&self, prob: &PyProblem, sol: &mut PySolution) -> Result<(), OptError> {
        let next = prob.call(|py| {
            let value = prob.neighborhoods[K].obj.bind(py).call_method1(
                "apply",
                (prob.obj.bind(py), sol.value.bind(py), self.mv.bind(py)),
            )?;
            prob.solution(py, value)
        });
        match next {
            Some(next) => {
                *sol = next;
                Ok(())
            }
            None => Err(OptError::InvalidState(
                "a Python callback raised an exception".to_string(),
            )),
        }
    }

    fn iter(prob: &PyProblem, sol: &PySolution) -> impl Iterator<Item = Self> + Send {
        prob.call(|py| {
            prob.neighborhoods[K]
                .obj
                .bind(py)
                .call_method1("neighbors", (prob.obj.bind(py), sol.value.bind(py)))?
                .try_iter()?
                .map(|mv| Self::priced(prob, py, sol, mv?))
                .collect::<PyResult<Vec<_>>>()
        })
        .unwrap_or_default()
        .into_iter()
    }

    fn random_neighbor(prob: &PyProblem, sol: &PySolution, rng: &mut SmallRng) -> Option<Self> {
        let nb = &prob.neighborhoods[K];
        prob.call(|py| {
            let problem = prob.obj.bind(py);
            let value = sol.value.bind(py);
            let mv = if nb.has_random_neighbor {
                let mv = nb
                    .obj
                    .bind(py)
                    .call_method1("random_neighbor", (problem, value, prob.py_rng(py)))?;
                if mv.is_none() {
                    return Ok(None);
                }
                mv
            } else {
                // Pick before pricing, so only the chosen move pays for its delta.
                let moves: Vec<_> = nb
                    .obj
                    .bind(py)
                    .call_method1("neighbors", (problem, value))?
                    .try_iter()?
                    .collect::<PyResult<_>>()?;
                if moves.is_empty() {
                    return Ok(None);
                }
                let i = rng.random_range(0..moves.len());
                moves.into_iter().nth(i).unwrap()
            };
            Self::priced(prob, py, sol, mv).map(Some)
        })
        .flatten()
    }

    fn move_to_be_better_than(
        &self,
        _prob: &PyProblem,
        src: &PySolution,
        other: &PySolution,
    ) -> bool {
        self.evaluate()
            .improves_over(src.evaluate(), other.evaluate())
    }

    fn tabu_policy(&self) -> Option<&dyn EnabledTabu> {
        Some(self)
    }
}

impl<const K: usize> EnabledTabu for PyMove<K> {
    /// A move is allowed while none of its keys is tabu.
    fn is_move_enabled(&self, tabu: &TabuMemory, iteration: u64) -> bool {
        self.tabu_keys
            .iter()
            .all(|&k| tabu.is_enabled(k, iteration))
    }

    fn add_to_tabu_map(&self, tabu: &mut TabuMemory, iteration: u64, rng: &mut SmallRng) {
        for &key in &self.tabu_keys {
            tabu.forbid(key, iteration, rng);
        }
    }
}

/// Stops a heuristic on a [`PyProblem`] as soon as a Python callback has
/// raised or a signal such as Ctrl-C is pending.
///
/// Every heuristic built for a Python problem is wrapped, nested ones
/// included, so a VNS step stops as promptly as the VNS itself.
pub struct Guarded {
    pub inner: Box<dyn Heuristic<PyProblem>>,
}

impl Heuristic<PyProblem> for Guarded {
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn stop_condition(&self) -> &StopCondition {
        self.inner.stop_condition()
    }

    fn is_done(&self, state: &SearchState<'_, PyProblem>) -> bool {
        let prob = state.instance;
        if let Some(Err(e)) = prob.call(|py| Ok(py.check_signals())) {
            prob.record(e);
        }
        prob.failed() || self.inner.is_done(state)
    }

    fn run_once(&mut self, state: &mut SearchState<'_, PyProblem>) -> Result<(), OptError> {
        self.inner.run_once(state)
    }
}

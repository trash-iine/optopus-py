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
//! Heuristics that need more than moves ask for more.
//!
//! - `GeneticAlgorithm` needs `crossover(a, b, rng)` on the problem, the child
//!   of two solutions.
//! - `BreakoutLocalSearch` and `GeneticAlgorithm` compare solutions through an
//!   optional `distance(a, b)` on the problem, a non-negative int, and fall back
//!   to `0` for `a == b` and `1` otherwise.
//! - `AdaptiveLargeNeighborhoodSearch` needs the ruin methods, see
//!   [`RUIN_METHODS`].
//!
//! Solutions and moves are opaque to the Rust side. `rng` is a
//! `random.Random` seeded from the run's seed, so a seeded run reproduces.
//!
//! A Python exception inside any of these calls, or a pending signal such as
//! Ctrl-C, stops the run at the next step, and the exception is raised from
//! `run` once the heuristic has returned.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use optopus::common::{TabuKey, TabuMemory};
use optopus::error::OptError;
use optopus::prelude::{
    EnabledTabu, Evaluable, Evaluate, Heuristic, MoveToNeighbor, ProblemTrait, SearchState,
    StopCondition,
};
use optopus::trait_defs::{Crossover, Distance};
use optopus::trait_defs::{LocalRepair, Ruinable};
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

/// The methods `AdaptiveLargeNeighborhoodSearch` needs on a Python problem,
/// one for each required method of [`Ruinable`]. Elements are ints, and a
/// partial is any Python object the methods agree on.
///
/// - `to_partial(solution)`, a fresh mutable copy the others edit in place,
/// - `finish(partial)`, the solution it describes,
/// - `elements(partial)`, the placed elements,
/// - `remove_all(partial, elements)`,
/// - `removal_gain(partial, element)`, what taking it out saves,
/// - `relatedness(a, b)`, smaller is more alike,
/// - `num_buckets(partial)`, the containers an element may go into,
/// - `num_places(partial, bucket)`, the positions a container offers,
/// - `insertion_cost(partial, bucket, place, element)`,
/// - `insert(partial, bucket, place, element)`.
///
/// Optional are `partial_objective(partial)`, the objective without calling
/// `finish`, and `repair_around(partial, anchors, rng)`, a local search around
/// the elements just re-inserted.
pub const RUIN_METHODS: [&str; 10] = [
    "to_partial",
    "finish",
    "elements",
    "remove_all",
    "removal_gain",
    "relatedness",
    "num_buckets",
    "num_places",
    "insertion_cost",
    "insert",
];

/// What every solution of one problem needs to read, shared rather than copied.
struct Shared {
    obj: Py<PyAny>,
    minimize: bool,
    has_distance: bool,
    /// The first Python exception raised during the current run.
    error: Mutex<Option<PyErr>>,
}

impl Shared {
    /// Runs `f`, recording its exception and returning `None` in its place.
    fn call<T>(&self, f: impl FnOnce(Python<'_>) -> PyResult<T>) -> Option<T> {
        if self.error.lock().unwrap().is_some() {
            return None;
        }
        Python::attach(f)
            .map_err(|e| {
                let mut slot = self.error.lock().unwrap();
                if slot.is_none() {
                    *slot = Some(e);
                }
            })
            .ok()
    }
}

/// A problem implemented in Python.
pub struct PyProblem {
    obj: Py<PyAny>,
    shared: Arc<Shared>,
    neighborhoods: Vec<Neighborhood>,
    has_crossover: bool,
    has_partial_objective: bool,
    has_repair_around: bool,
    /// The `random.Random` handed to the Python callbacks, reseeded from the
    /// run's RNG every time a run draws its initial solution.
    rng: Mutex<Option<Py<PyAny>>>,
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
            shared: Arc::new(Shared {
                obj: obj.clone().unbind(),
                minimize,
                has_distance: obj.hasattr("distance")?,
                error: Mutex::new(None),
            }),
            neighborhoods,
            has_crossover: obj.hasattr("crossover")?,
            has_partial_objective: obj.hasattr("partial_objective")?,
            has_repair_around: obj.hasattr("repair_around")?,
            rng: Mutex::new(None),
            record_tabu: AtomicBool::new(false),
        })
    }

    pub fn minimize(&self) -> bool {
        self.shared.minimize
    }

    /// Fails unless the problem has every method in [`RUIN_METHODS`].
    pub fn check_ruinable(&self) -> Result<(), String> {
        let missing: Vec<_> = Python::attach(|py| {
            let obj = self.obj.bind(py);
            RUIN_METHODS
                .iter()
                .filter(|m| !obj.hasattr(**m).unwrap_or(false))
                .map(|m| format!("'{m}'"))
                .collect()
        });
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "AdaptiveLargeNeighborhoodSearch needs these methods on the problem: {}",
                missing.join(", ")
            ))
        }
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
        self.shared.error.lock().unwrap().take()
    }

    fn failed(&self) -> bool {
        self.shared.error.lock().unwrap().is_some()
    }

    /// Keeps the first exception of a run and drops later ones, which are
    /// usually consequences of it.
    fn record(&self, err: PyErr) {
        let mut slot = self.shared.error.lock().unwrap();
        if slot.is_none() {
            *slot = Some(err);
        }
    }

    /// Runs `f`, recording its exception and returning `None` in its place.
    fn call<T>(&self, f: impl FnOnce(Python<'_>) -> PyResult<T>) -> Option<T> {
        self.shared.call(f)
    }

    /// Calls the problem's method `name`, with `fallback` in place of a result
    /// once a callback has raised.
    fn call_method<T: for<'a, 'py> FromPyObject<'a, 'py>>(
        &self,
        name: &str,
        args: impl for<'py> FnOnce(Python<'py>) -> Vec<Bound<'py, PyAny>>,
        fallback: T,
    ) -> T {
        self.call(|py| {
            let args = PyTuple::new(py, args(py))?;
            self.obj
                .bind(py)
                .call_method1(name, args)?
                .extract()
                .map_err(Into::into)
        })
        .unwrap_or(fallback)
    }

    /// A solution that stands in for one a failed callback did not return.
    /// The run stops before it looks at it, see [`Guarded`].
    fn placeholder(&self) -> PySolution {
        PySolution {
            value: Python::attach(|py| py.None()),
            objective: f64::NAN,
            shared: self.shared.clone(),
        }
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
            shared: self.shared.clone(),
        })
    }
}

/// A solution of a [`PyProblem`], with its objective cached.
pub struct PySolution {
    pub value: Py<PyAny>,
    pub objective: f64,
    shared: Arc<Shared>,
}

impl Clone for PySolution {
    /// Solutions are never mutated in place (`apply` returns a new one), so a
    /// clone shares the Python object.
    fn clone(&self) -> Self {
        Self {
            value: Python::attach(|py| self.value.clone_ref(py)),
            objective: self.objective,
            shared: self.shared.clone(),
        }
    }
}

impl Evaluate for PySolution {
    fn evaluate(&self) -> Evaluable<f64> {
        evaluable(self.shared.minimize, self.objective)
    }
}

impl Distance for PySolution {
    fn distance(&self, other: &Self) -> usize {
        self.shared
            .call(|py| {
                let (a, b) = (self.value.bind(py), other.value.bind(py));
                if self.shared.has_distance {
                    self.shared
                        .obj
                        .bind(py)
                        .call_method1("distance", (a, b))?
                        .extract()
                } else {
                    Ok(usize::from(!a.eq(b)?))
                }
            })
            .unwrap_or(0)
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
        made.unwrap_or_else(|| self.placeholder())
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
            minimize: prob.shared.minimize,
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
    // An int is whatever the user computed, so it is a `Var`, kept in the map. `DenseVar` would
    // size an array by it, and a key such as `item * 10**15 + bin` aborted the process.
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

/// The GA crossover of a [`PyProblem`], its `crossover(a, b, rng)` method.
pub struct PyCrossover;

impl PyCrossover {
    pub fn new(prob: &PyProblem) -> Result<Self, String> {
        if prob.has_crossover {
            Ok(Self)
        } else {
            Err("GeneticAlgorithm needs a 'crossover' method on the problem".to_string())
        }
    }
}

impl Crossover<PyProblem> for PyCrossover {
    fn crossover(
        &mut self,
        prob: &PyProblem,
        sol1: &PySolution,
        sol2: &PySolution,
        _rng: &mut SmallRng,
    ) -> Result<PySolution, OptError> {
        prob.call(|py| {
            let child = prob.obj.bind(py).call_method1(
                "crossover",
                (sol1.value.bind(py), sol2.value.bind(py), prob.py_rng(py)),
            )?;
            prob.solution(py, child)
        })
        .ok_or_else(|| OptError::InvalidState("a Python callback raised an exception".into()))
    }
}

/// Ruin and recreate on a Python problem, through the methods in
/// [`RUIN_METHODS`]. A failed callback records its exception and answers with a
/// harmless value, one container with one place, so the operators finish their
/// iteration and the run stops at the next step.
impl Ruinable for PyProblem {
    type Element = usize;
    type Partial = Py<PyAny>;

    fn to_partial(&self, sol: &PySolution) -> Py<PyAny> {
        self.call(|py| {
            let p = self
                .obj
                .bind(py)
                .call_method1("to_partial", (sol.value.bind(py),))?;
            Ok(p.unbind())
        })
        .unwrap_or_else(|| Python::attach(|py| py.None()))
    }

    fn finish(&self, partial: &Py<PyAny>) -> PySolution {
        self.call(|py| {
            let value = self
                .obj
                .bind(py)
                .call_method1("finish", (partial.bind(py),))?;
            self.solution(py, value)
        })
        .unwrap_or_else(|| self.placeholder())
    }

    fn elements(&self, partial: &Py<PyAny>, out: &mut Vec<usize>) {
        out.clear();
        if let Some(elements) = self.call(|py| {
            self.obj
                .bind(py)
                .call_method1("elements", (partial.bind(py),))?
                .try_iter()?
                .map(|e| e?.extract::<usize>())
                .collect::<PyResult<Vec<_>>>()
        }) {
            out.extend(elements);
        }
    }

    fn remove_all(&self, partial: &mut Py<PyAny>, set: &[usize]) {
        self.call(|py| {
            self.obj
                .bind(py)
                .call_method1("remove_all", (partial.bind(py), set.to_vec()))
                .map(drop)
        });
    }

    fn removal_gain(&self, partial: &Py<PyAny>, element: usize) -> f64 {
        self.call_method(
            "removal_gain",
            |py| {
                vec![
                    partial.bind(py).clone(),
                    element.into_pyobject(py).unwrap().into_any(),
                ]
            },
            0.0,
        )
    }

    fn relatedness(&self, a: usize, b: usize) -> f64 {
        self.call_method(
            "relatedness",
            |py| {
                vec![
                    a.into_pyobject(py).unwrap().into_any(),
                    b.into_pyobject(py).unwrap().into_any(),
                ]
            },
            0.0,
        )
    }

    fn num_buckets(&self, partial: &Py<PyAny>) -> usize {
        self.call_method("num_buckets", |py| vec![partial.bind(py).clone()], 1)
    }

    fn num_places(&self, partial: &Py<PyAny>, bucket: usize) -> usize {
        self.call_method(
            "num_places",
            |py| {
                vec![
                    partial.bind(py).clone(),
                    bucket.into_pyobject(py).unwrap().into_any(),
                ]
            },
            1,
        )
    }

    fn insertion_cost(
        &self,
        partial: &Py<PyAny>,
        bucket: usize,
        place: usize,
        element: usize,
    ) -> f64 {
        self.call_method(
            "insertion_cost",
            |py| {
                vec![
                    partial.bind(py).clone(),
                    bucket.into_pyobject(py).unwrap().into_any(),
                    place.into_pyobject(py).unwrap().into_any(),
                    element.into_pyobject(py).unwrap().into_any(),
                ]
            },
            0.0,
        )
    }

    fn insert(&self, partial: &mut Py<PyAny>, bucket: usize, place: usize, element: usize) {
        self.call(|py| {
            self.obj
                .bind(py)
                .call_method1("insert", (partial.bind(py), bucket, place, element))
                .map(drop)
        });
    }

    fn partial_energy(&self, partial: &Py<PyAny>) -> f64 {
        let objective = if self.has_partial_objective {
            self.call_method(
                "partial_objective",
                |py| vec![partial.bind(py).clone()],
                f64::NAN,
            )
        } else {
            self.finish(partial).objective
        };
        evaluable(self.shared.minimize, objective).minimized()
    }
}

/// The ALNS local repair of a [`PyProblem`], its `repair_around` method.
pub struct PyRepair;

impl PyRepair {
    /// `None` when the problem has no `repair_around`, so ALNS runs plain ruin
    /// and recreate.
    pub fn new(prob: &PyProblem) -> Option<Self> {
        prob.has_repair_around.then_some(Self)
    }
}

impl LocalRepair<PyProblem> for PyRepair {
    fn repair_around(
        &mut self,
        prob: &PyProblem,
        partial: &mut Py<PyAny>,
        anchors: &[usize],
        _rng: &mut SmallRng,
    ) {
        prob.call(|py| {
            prob.obj
                .bind(py)
                .call_method1(
                    "repair_around",
                    (partial.bind(py), anchors.to_vec(), prob.py_rng(py)),
                )
                .map(drop)
        });
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

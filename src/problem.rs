use optopus::common::{Graph, seeded_rng};
use optopus::problem::{
    Constraint, ConstraintRel, Expr, FormulaProblem, JobShopScheduling as OptJobShop,
    MaxCut as OptMaxCut, MaxCutKernel as OptMaxCutKernel, OptDirection,
    PlantedMaxCut as OptPlantedMaxCut, Qubo as OptQubo, Sat as OptSat, TileProbs2d, TileProbs3d,
    Tsp as OptTsp, VertexCover as OptVc, Vrp as OptVrp, WishartCouplers,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::graph::{Graph as PyGraph, resolve_seed};

/// A polynomial term used by Python-side `Formula` construction:
/// `(variable_indices, coefficient)`. An empty `variable_indices` list means a constant term.
type PyMonomial = (Vec<usize>, f64);
/// A polynomial = sum of monomial terms.
type PyPoly = Vec<PyMonomial>;

/// Builds a flat polynomial [`Expr`] (sum of monomial products) from the Python representation.
fn poly_to_expr(poly: &PyPoly) -> Expr {
    let mut terms: Vec<Expr> = Vec::with_capacity(poly.len());
    for (vars, coeff) in poly {
        let mut factors: Vec<Expr> = Vec::with_capacity(vars.len() + 1);
        factors.push(Expr::Const(*coeff));
        for &v in vars {
            factors.push(Expr::Var(v));
        }
        terms.push(if factors.len() == 1 {
            factors.pop().unwrap()
        } else {
            Expr::Mul(factors)
        });
    }
    if terms.is_empty() {
        Expr::Const(0.0)
    } else if terms.len() == 1 {
        terms.pop().unwrap()
    } else {
        Expr::Add(terms)
    }
}

fn parse_rel(rel: &str) -> PyResult<ConstraintRel> {
    match rel {
        "Lt" | "<" => Ok(ConstraintRel::Lt),
        "Le" | "<=" => Ok(ConstraintRel::Le),
        "Eq" | "==" | "=" => Ok(ConstraintRel::Eq),
        "Ge" | ">=" => Ok(ConstraintRel::Ge),
        "Gt" | ">" => Ok(ConstraintRel::Gt),
        other => Err(PyValueError::new_err(format!(
            "invalid relation '{other}' (use 'Lt', 'Le', 'Eq', 'Ge', or 'Gt')"
        ))),
    }
}

fn parse_direction(direction: &str) -> PyResult<OptDirection> {
    match direction {
        "Maximize" | "max" => Ok(OptDirection::Maximize),
        "Minimize" | "min" => Ok(OptDirection::Minimize),
        other => Err(PyValueError::new_err(format!(
            "invalid direction '{other}' (use 'Maximize' or 'Minimize')"
        ))),
    }
}

/// The Max Cut problem (maximization).
///
/// Partition the vertices of an undirected weighted graph into two sides so that
/// the total weight of edges crossing the partition is as large as possible.
///
/// A solution is encoded as a ``list[bool]`` of length *number of vertices*,
/// where element ``i`` is the side that vertex ``i`` is assigned to.
#[pyclass(module = "optopus")]
pub struct MaxCut {
    pub inner: OptMaxCut,
}

#[pymethods]
impl MaxCut {
    /// Build a Max Cut instance from a weighted edge list.
    ///
    /// The number of vertices is inferred from the largest vertex index that
    /// appears in ``edges``.
    ///
    /// Args:
    ///     edges (list[tuple[int, int, float]]): Weighted edges ``(u, v, weight)``
    ///         with 0-indexed vertices.
    ///
    /// Returns:
    ///     MaxCut: A new problem instance.
    #[staticmethod]
    fn from_edges(edges: Vec<(usize, usize, f32)>) -> Self {
        Self {
            inner: OptMaxCut::from_edges(edges),
        }
    }

    /// Build a Max Cut instance from a ``Graph``.
    ///
    /// Useful together with the random graph generators, e.g.
    /// ``MaxCut.from_graph(Graph.erdos_renyi(100, 0.05, seed=42))``.
    ///
    /// Args:
    ///     graph (Graph): The underlying graph.
    ///
    /// Returns:
    ///     MaxCut: A new problem instance.
    #[staticmethod]
    fn from_graph(graph: &PyGraph) -> Self {
        Self {
            inner: OptMaxCut::new(graph.inner.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "MaxCut(num_vertices={}, num_edges={})",
            self.inner.graph.num_vertices(),
            self.inner.graph.num_edges()
        )
    }
}

/// Parses the coupler distribution accepted by `PlantedMaxCut.wishart`.
fn parse_couplers(s: &str) -> PyResult<WishartCouplers> {
    match s {
        "Gaussian" => Ok(WishartCouplers::Gaussian),
        "Discrete" => Ok(WishartCouplers::Discrete),
        other => Err(PyValueError::new_err(format!(
            "invalid couplers '{other}' (use 'Gaussian' or 'Discrete')"
        ))),
    }
}

/// Rejects a tile-planting side length that upstream would assert on.
fn check_tile_side(l: usize) -> PyResult<()> {
    if l < 4 || !l.is_multiple_of(2) {
        return Err(PyValueError::new_err(format!(
            "'l' must be an even integer >= 4, got {l}"
        )));
    }
    Ok(())
}

/// Rejects tile-class probabilities that are negative or sum past 1.
fn check_tile_probs(probs: &[(&str, f64)]) -> PyResult<()> {
    let mut total = 0.0;
    for (name, value) in probs {
        if *value < 0.0 || value.is_nan() {
            return Err(PyValueError::new_err(format!(
                "'{name}' must not be negative, got {value}"
            )));
        }
        total += *value;
    }
    if total > 1.0 {
        return Err(PyValueError::new_err(format!(
            "tile-class probabilities must sum to at most 1.0, got {total}"
        )));
    }
    Ok(())
}

/// An exact kernelization of a MaxCut instance.
///
/// Applies the isolated-vertex, pendant-vertex, degree-2-path and weight-domination rules to
/// a fixpoint, producing a smaller instance whose optimum maps back exactly:
/// ``kernel_cut(y) + offset == original_cut(lift(y))`` for every kernel assignment ``y``.
///
/// The kernel is a plain :class:`MaxCut`, so any heuristic runs on it unchanged. The rules
/// bite on sparse, irregular graphs and do nothing on regular or dense ones -- check
/// :meth:`is_trivial` before paying for the extra indirection.
///
/// Example:
///     >>> k = MaxCutKernel.reduce(problem)
///     >>> if not k.is_trivial():
///     ...     report = heuristic.run(k.kernel(), runs=8, seed=42)
///     ...     assignment = k.lift(report.runs[0].solution)
///     ...     cut = report.best_objective + k.offset()
#[pyclass(module = "optopus")]
pub struct MaxCutKernel {
    inner: OptMaxCutKernel,
    original_len: usize,
}

#[pymethods]
impl MaxCutKernel {
    /// Reduce a MaxCut instance to its kernel.
    ///
    /// Args:
    ///     problem (MaxCut): The instance to reduce.
    ///
    /// Returns:
    ///     MaxCutKernel: The reduction, holding the kernel and the trace needed to lift.
    #[staticmethod]
    fn reduce(problem: &MaxCut) -> Self {
        Self {
            inner: OptMaxCutKernel::new(&problem.inner),
            original_len: problem.inner.graph.num_vertices(),
        }
    }

    /// The reduced instance, as an ordinary problem.
    ///
    /// Returns:
    ///     MaxCut: The kernel.
    fn kernel(&self) -> MaxCut {
        MaxCut {
            inner: self.inner.kernel().clone(),
        }
    }

    /// Constant to add to a kernel cut value to recover the original one.
    fn offset(&self) -> f32 {
        self.inner.offset()
    }

    /// Whether no vertex could be removed, i.e. the kernel equals the input.
    ///
    /// True for regular and dense graphs. Treat it as "skip the whole mechanism" rather than
    /// solving an identical instance through an extra layer.
    fn is_trivial(&self) -> bool {
        self.inner.is_trivial()
    }

    /// Number of vertices the reduction removed.
    fn removed_vertices(&self) -> usize {
        self.inner.removed_vertices()
    }

    /// Map a kernel assignment back to the original vertex set, re-deriving every removed
    /// vertex optimally for that assignment.
    ///
    /// Args:
    ///     assignment (list[bool]): An assignment over the kernel's vertices.
    ///
    /// Returns:
    ///     list[bool]: The corresponding assignment over the original vertices.
    ///
    /// Raises:
    ///     ValueError: If ``assignment`` is shorter than the kernel's vertex count.
    fn lift(&self, assignment: Vec<bool>) -> PyResult<Vec<bool>> {
        let expected = self.inner.kernel().graph.num_vertices();
        if assignment.len() < expected {
            return Err(PyValueError::new_err(format!(
                "'assignment' must have at least {expected} entries (the kernel's vertex count), got {}",
                assignment.len()
            )));
        }
        Ok(self.inner.lift(&assignment))
    }

    /// Restrict a full assignment to the kernel's vertices, for warm starts.
    ///
    /// Args:
    ///     assignment (list[bool]): An assignment over the original vertices.
    ///
    /// Returns:
    ///     list[bool]: The corresponding assignment over the kernel's vertices.
    ///
    /// Raises:
    ///     ValueError: If ``assignment`` is shorter than the original vertex count.
    fn project(&self, assignment: Vec<bool>) -> PyResult<Vec<bool>> {
        if assignment.len() < self.original_len {
            return Err(PyValueError::new_err(format!(
                "'assignment' must have at least {} entries (the original vertex count), got {}",
                self.original_len,
                assignment.len()
            )));
        }
        Ok(self.inner.project(&assignment))
    }

    fn __repr__(&self) -> String {
        format!(
            "MaxCutKernel(original_vertices={}, kernel_vertices={}, removed={}, offset={})",
            self.original_len,
            self.inner.kernel().graph.num_vertices(),
            self.inner.removed_vertices(),
            self.inner.offset()
        )
    }
}

/// A MaxCut instance whose optimum is known by construction.
///
/// The generators plant an optimal cut and then gauge-transform the instance, so the planted
/// assignment really is optimal rather than merely good. That makes them the right yardstick
/// for "did the heuristic reach the optimum", which random instances cannot answer.
///
/// Example:
///     >>> planted = PlantedMaxCut.wishart(64, 0.75, couplers="Discrete", seed=1)
///     >>> report = heuristic.run(planted.problem(), runs=10, seed=42)
///     >>> hit = report.best_objective == planted.optimum()  # exact: weights are integral
#[pyclass(module = "optopus")]
pub struct PlantedMaxCut {
    inner: OptPlantedMaxCut,
}

#[pymethods]
impl PlantedMaxCut {
    /// Tile planting on a 2D periodic square lattice.
    ///
    /// The lattice has ``l * l`` vertices and ``2 * l * l`` edges and is 4-regular, matching
    /// the topology of the G-set's toroidal group. Each plaquette is drawn from four
    /// frustration classes; ``p1``, ``p2`` and ``p3`` give the first three, and the fourth
    /// takes the remaining probability.
    ///
    /// Args:
    ///     l (int): Side length; must be even and ``>= 4``.
    ///     p1 (float): Probability of tile class 1.
    ///     p2 (float): Probability of tile class 2.
    ///     p3 (float): Probability of tile class 3.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     PlantedMaxCut: The instance, its planted cut and the exact optimum.
    ///
    /// Raises:
    ///     ValueError: If ``l`` is odd or below 4, or the probabilities are negative or sum
    ///         past 1.
    #[staticmethod]
    #[pyo3(signature = (l, p1, p2, p3, seed=None))]
    fn tile_planting_2d(l: usize, p1: f64, p2: f64, p3: f64, seed: Option<u64>) -> PyResult<Self> {
        check_tile_side(l)?;
        check_tile_probs(&[("p1", p1), ("p2", p2), ("p3", p3)])?;
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptPlantedMaxCut::tile_planting_2d(l, TileProbs2d::new(p1, p2, p3), &mut rng),
        })
    }

    /// Tile planting on a 3D periodic cubic lattice.
    ///
    /// The lattice has ``l ** 3`` vertices and ``3 * l ** 3`` edges and is 6-regular. Each cell
    /// is drawn from three frustration classes; ``p_2fp`` and ``p_4fp`` give the 2- and
    /// 4-frustrated-plaquette classes, and the 6-frustrated one takes the remainder.
    ///
    /// Args:
    ///     l (int): Side length; must be even and ``>= 4``.
    ///     p_2fp (float): Probability of the 2-frustrated-plaquette class.
    ///     p_4fp (float): Probability of the 4-frustrated-plaquette class.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     PlantedMaxCut: The instance, its planted cut and the exact optimum.
    ///
    /// Raises:
    ///     ValueError: If ``l`` is odd or below 4, or the probabilities are negative or sum
    ///         past 1.
    #[staticmethod]
    #[pyo3(signature = (l, p_2fp, p_4fp, seed=None))]
    fn tile_planting_3d(l: usize, p_2fp: f64, p_4fp: f64, seed: Option<u64>) -> PyResult<Self> {
        check_tile_side(l)?;
        check_tile_probs(&[("p_2fp", p_2fp), ("p_4fp", p_4fp)])?;
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptPlantedMaxCut::tile_planting_3d(l, TileProbs3d::new(p_2fp, p_4fp), &mut rng),
        })
    }

    /// The Wishart planted ensemble: a dense instance on the complete graph whose hardness is
    /// tuned by ``alpha``.
    ///
    /// Args:
    ///     n (int): Number of vertices, ``>= 2``. The graph is complete, so it has
    ///         ``n * (n - 1) / 2`` edges.
    ///     alpha (float): Ratio of planted constraints to variables, in ``(0, 1)`` exclusive.
    ///         Smaller is harder.
    ///     couplers (str): ``"Gaussian"`` for real-valued couplers (the default, and the only
    ///         one usable at large ``n``), or ``"Discrete"`` for integer ones, which make the
    ///         optimum exact at the cost of a scaling that grows as ``n ** 3``.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     PlantedMaxCut: The instance, its planted cut and the optimum.
    ///
    /// Raises:
    ///     ValueError: If ``n < 2``, ``alpha`` is outside ``(0, 1)``, or ``couplers`` is not a
    ///         recognized value.
    #[staticmethod]
    #[pyo3(signature = (n, alpha, couplers="Gaussian".to_string(), seed=None))]
    fn wishart(n: usize, alpha: f64, couplers: String, seed: Option<u64>) -> PyResult<Self> {
        if n < 2 {
            return Err(PyValueError::new_err(format!(
                "'n' must be at least 2, got {n}"
            )));
        }
        if !(alpha > 0.0 && alpha < 1.0) {
            return Err(PyValueError::new_err(format!(
                "'alpha' must be within (0.0, 1.0), got {alpha}"
            )));
        }
        let couplers = parse_couplers(&couplers)?;
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptPlantedMaxCut::wishart(n, alpha, couplers, &mut rng),
        })
    }

    /// The generated instance, as an ordinary problem to hand to a heuristic.
    ///
    /// Returns:
    ///     MaxCut: The instance.
    fn problem(&self) -> MaxCut {
        MaxCut {
            inner: self.inner.problem.clone(),
        }
    }

    /// The planted assignment, which is an optimal cut.
    ///
    /// Returns:
    ///     list[bool]: One side per vertex.
    fn planted(&self) -> Vec<bool> {
        self.inner.planted.clone()
    }

    /// The cut weight of the planted assignment, i.e. the optimum.
    fn optimum(&self) -> f32 {
        self.inner.optimum
    }

    /// Whether every edge weight is integral, so the optimum survives the ``f32`` objective
    /// exactly.
    ///
    /// When True, "did the run reach the optimum" is an equality against :meth:`optimum`.
    /// When False, compare within a tolerance instead.
    fn has_exact_optimum(&self) -> bool {
        self.inner.has_exact_optimum()
    }

    /// Re-check the construction: no zero-weight edges, and the stored optimum agrees with
    /// the cut weight of the planted assignment.
    ///
    /// Raises:
    ///     ValueError: Describing the first check that fails.
    fn verify(&self) -> PyResult<()> {
        self.inner
            .verify()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PlantedMaxCut(num_vertices={}, num_edges={}, optimum={}, exact={})",
            self.inner.problem.graph.num_vertices(),
            self.inner.problem.graph.num_edges(),
            self.inner.optimum,
            self.inner.has_exact_optimum()
        )
    }
}

/// The QUBO problem (minimization).
///
/// Minimize the quadratic form :math:`x^\top Q x` over binary variables
/// :math:`x \in \{0, 1\}^n`.
///
/// A solution is encoded as a ``list[bool]`` of length *number of variables*,
/// where element ``i`` is the value of variable ``i`` (``True`` = 1).
#[pyclass(module = "optopus")]
pub struct Qubo {
    pub inner: OptQubo,
}

#[pymethods]
impl Qubo {
    /// Build a QUBO instance from matrix entries.
    ///
    /// Diagonal entries ``(i, i, c)`` are the linear terms; off-diagonal entries
    /// ``(i, j, c)`` are the quadratic interaction terms.
    ///
    /// Args:
    ///     entries (list[tuple[int, int, int]]): Matrix entries ``(i, j, coefficient)``
    ///         with 0-indexed variables and integer (``i32``) coefficients.
    ///
    /// Returns:
    ///     Qubo: A new problem instance.
    #[staticmethod]
    fn from_entries(entries: Vec<(usize, usize, i32)>) -> Self {
        Self {
            inner: OptQubo::from_entries(entries),
        }
    }

    fn __repr__(&self) -> String {
        format!("Qubo(dim={})", self.inner.len())
    }
}

/// The MaxSAT problem (maximization).
///
/// Given a CNF formula, find a variable assignment that satisfies as many clauses as possible.
///
/// A solution is encoded as a ``list[bool]`` of length *number of variables*.
#[pyclass(module = "optopus")]
pub struct Sat {
    pub inner: OptSat,
}

#[pymethods]
impl Sat {
    /// Build a SAT instance from a list of clauses.
    ///
    /// Each clause is a list of DIMACS-style literals: a positive integer ``+i`` for variable
    /// ``i`` (1-indexed) appearing positively, and ``-i`` for its negation. Variable ``0`` is
    /// not allowed.
    ///
    /// Args:
    ///     n_vars (int): Number of variables.
    ///     clauses (list[list[int]]): Clauses as lists of non-zero literals.
    ///
    /// Returns:
    ///     Sat: A new problem instance.
    ///
    /// Example:
    ///     ``Sat.from_clauses(3, [[1, -2, 3], [-1, 2], [3]])``
    #[staticmethod]
    fn from_clauses(n_vars: usize, clauses: Vec<Vec<i64>>) -> Self {
        let mut inner = OptSat::new(n_vars);
        for clause in clauses {
            inner.add_clause(clause);
        }
        Self { inner }
    }

    fn __repr__(&self) -> String {
        format!(
            "Sat(n_vars={}, n_clauses={})",
            self.inner.n_vars(),
            self.inner.n_clauses()
        )
    }
}

/// The Minimum Vertex Cover problem (minimization).
///
/// Find the smallest subset of vertices such that every edge has at least one endpoint in the
/// subset.
///
/// A solution is encoded as a ``list[bool]`` of length *number of vertices*, where element
/// ``i`` is ``True`` if vertex ``i`` is in the cover.
#[pyclass(module = "optopus")]
pub struct VertexCover {
    pub inner: OptVc,
}

#[pymethods]
impl VertexCover {
    /// Build a Vertex Cover instance from a weighted edge list.
    ///
    /// The edge weight is currently unused by the search but must be provided to match the
    /// underlying ``Graph`` API; pass ``1.0`` if you don't care.
    ///
    /// Args:
    ///     edges (list[tuple[int, int, float]]): Edges ``(u, v, weight)`` with 0-indexed vertices.
    ///
    /// Returns:
    ///     VertexCover: A new problem instance.
    #[staticmethod]
    fn from_edges(edges: Vec<(usize, usize, f32)>) -> Self {
        Self {
            inner: OptVc::new(Graph::from_edges(edges)),
        }
    }

    /// Build a Vertex Cover instance from a ``Graph``.
    ///
    /// Useful together with the random graph generators, e.g.
    /// ``VertexCover.from_graph(Graph.barabasi_albert(100, 3, seed=42))``.
    ///
    /// Args:
    ///     graph (Graph): The underlying graph.
    ///
    /// Returns:
    ///     VertexCover: A new problem instance.
    #[staticmethod]
    fn from_graph(graph: &PyGraph) -> Self {
        Self {
            inner: OptVc::new(graph.inner.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "VertexCover(num_vertices={}, num_edges={})",
            self.inner.graph.num_vertices(),
            self.inner.graph.num_edges()
        )
    }
}

/// The 2D Travelling Salesman Problem (minimization).
///
/// Given a set of city coordinates, find a tour visiting every city exactly once that minimizes
/// the total Euclidean distance.
///
/// A solution is encoded as a ``list[int]`` permutation of city indices ``[0, n)``.
#[pyclass(module = "optopus")]
pub struct TspWithCoordinates {
    pub inner: OptTsp,
}

#[pymethods]
impl TspWithCoordinates {
    /// Build a TSP instance from city coordinates.
    ///
    /// Args:
    ///     coordinates (list[tuple[float, float]]): ``(x, y)`` coordinates for each city.
    ///     name (str): Optional instance name (default ``""``).
    ///
    /// Returns:
    ///     TspWithCoordinates: A new problem instance using continuous Euclidean distances.
    #[staticmethod]
    #[pyo3(signature = (coordinates, name=String::new()))]
    fn from_coordinates(coordinates: Vec<(f64, f64)>, name: String) -> Self {
        Self {
            inner: OptTsp::new(name, coordinates),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TspWithCoordinates(name={:?}, n_cities={})",
            self.inner.name,
            self.inner.get_n()
        )
    }
}

/// The Job Shop Scheduling problem (minimization, makespan).
///
/// Each job is a sequence of operations; each operation runs on a specific machine for a given
/// duration. Operations within a job must run in order, and each machine processes at most one
/// operation at a time.
///
/// A solution is encoded as a ``list[int]`` operation sequence (a permutation of all operations).
#[pyclass(module = "optopus")]
pub struct JobShopScheduling {
    pub inner: OptJobShop,
}

#[pymethods]
impl JobShopScheduling {
    /// Build a JSS instance from a list of jobs.
    ///
    /// Args:
    ///     jobs (list[list[tuple[int, int]]]): Each job is a list of ``(machine_index, duration)``
    ///         operations in the order they must run. Machine indices are 0-indexed; the number
    ///         of machines is inferred from the largest machine index + 1.
    ///     name (str): Optional instance name (default ``""``).
    ///
    /// Returns:
    ///     JobShopScheduling: A new problem instance.
    #[staticmethod]
    #[pyo3(signature = (jobs, name=String::new()))]
    fn from_jobs(jobs: Vec<Vec<(usize, u32)>>, name: String) -> PyResult<Self> {
        if jobs.is_empty() {
            return Err(PyValueError::new_err("'jobs' must not be empty"));
        }
        let n_machines = jobs
            .iter()
            .flat_map(|j| j.iter())
            .map(|(m, _)| *m)
            .max()
            .map(|m| m + 1)
            .ok_or_else(|| PyValueError::new_err("every job must have at least one operation"))?;
        Ok(Self {
            inner: OptJobShop::new(name, n_machines, jobs),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "JobShopScheduling(name={:?}, n_jobs={}, n_machines={})",
            self.inner.name, self.inner.n_jobs, self.inner.n_machines
        )
    }
}

/// The Capacitated Vehicle Routing Problem (minimization).
///
/// A fleet of identical vehicles based at a depot must serve every customer exactly once
/// without exceeding the vehicle capacity, minimizing the total travel distance.
///
/// Index ``0`` is the depot; customers are ``1..=n``. A solution is encoded as a
/// ``list[list[int]]`` with one route per vehicle, holding the customers in visiting order.
/// The depot is implicit at both ends of every route, and empty routes are allowed.
///
/// The objective a heuristic minimizes is ``distance + penalty_weight * overload``, so a
/// solution that exceeds capacity scores worse than any feasible one but is not rejected
/// outright. Use :meth:`evaluate_routes` to recover the raw distance and the overload
/// separately.
///
/// Example:
///     >>> vrp = Vrp.from_coordinates(
///     ...     coordinates=[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
///     ...     demands=[0, 5, 5],
///     ...     capacity=10,
///     ... )
#[pyclass(module = "optopus")]
pub struct Vrp {
    pub inner: OptVrp,
}

/// Rejects a coordinate/demand pair that upstream would assert on.
fn check_vrp_instance(coordinates: &[(f64, f64)], demands: &[i64], capacity: i64) -> PyResult<()> {
    if coordinates.is_empty() {
        return Err(PyValueError::new_err(
            "'coordinates' must not be empty (index 0 is the depot)",
        ));
    }
    if coordinates.len() != demands.len() {
        return Err(PyValueError::new_err(format!(
            "'coordinates' and 'demands' must have the same length, got {} and {}",
            coordinates.len(),
            demands.len()
        )));
    }
    if capacity <= 0 {
        return Err(PyValueError::new_err(format!(
            "'capacity' must be greater than 0, got {capacity}"
        )));
    }
    Ok(())
}

#[pymethods]
impl Vrp {
    /// Build a CVRP instance from coordinates and demands.
    ///
    /// Args:
    ///     coordinates (list[tuple[float, float]]): ``(x, y)`` per node. Index 0 is the depot.
    ///     demands (list[int]): Demand per node, same length as ``coordinates``. Entry 0 (the
    ///         depot) should be 0.
    ///     capacity (int): Vehicle capacity, ``> 0``.
    ///     num_vehicles (int): Fleet size. Defaults to 0, which asks optopus to pick one by
    ///         first-fit-decreasing bin packing plus a 10% margin -- read the chosen value back
    ///         with :meth:`num_vehicles`.
    ///     name (str): Optional instance name (default ``"vrp"``).
    ///     rounded (bool): When True, distances are rounded to the nearest integer, matching
    ///         CVRPLIB's ``EUC_2D`` convention. Defaults to False (plain Euclidean).
    ///
    /// Returns:
    ///     Vrp: A new problem instance.
    ///
    /// Raises:
    ///     ValueError: If ``coordinates`` is empty, its length differs from ``demands``, or
    ///         ``capacity`` is not positive.
    #[staticmethod]
    #[pyo3(signature = (coordinates, demands, capacity, num_vehicles=0, name="vrp".to_string(), rounded=false))]
    fn from_coordinates(
        coordinates: Vec<(f64, f64)>,
        demands: Vec<i64>,
        capacity: i64,
        num_vehicles: usize,
        name: String,
        rounded: bool,
    ) -> PyResult<Self> {
        check_vrp_instance(&coordinates, &demands, capacity)?;
        let inner = if rounded {
            OptVrp::with_rounding(name, coordinates, demands, capacity, num_vehicles)
        } else {
            OptVrp::new(name, coordinates, demands, capacity, num_vehicles)
        };
        Ok(Self { inner })
    }

    /// Load a CVRP instance from a CVRPLIB-format file.
    ///
    /// Accepts the TSPLIB-style header (``NAME``, ``DIMENSION``, ``EDGE_WEIGHT_TYPE: EUC_2D``,
    /// ``CAPACITY``, and an optional ``COMMENT`` carrying ``No of trucks: K``) followed by the
    /// ``NODE_COORD_SECTION``, ``DEMAND_SECTION`` and ``DEPOT_SECTION``. Node 1 in the file is
    /// re-indexed to 0. Distances are always rounded, as CVRPLIB's ``EUC_2D`` prescribes.
    ///
    /// Args:
    ///     path (str): Path to the instance file.
    ///
    /// Returns:
    ///     Vrp: The loaded problem instance.
    ///
    /// Raises:
    ///     ValueError: If the file cannot be read or does not parse.
    #[staticmethod]
    fn load_file(path: &str) -> PyResult<Self> {
        OptVrp::load_file(path)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Number of customers, excluding the depot.
    fn num_customers(&self) -> usize {
        self.inner.get_n()
    }

    /// Fleet size. When the instance was built with ``num_vehicles=0``, this is the value
    /// optopus picked.
    fn num_vehicles(&self) -> usize {
        self.inner.num_vehicles
    }

    /// Vehicle capacity.
    fn capacity(&self) -> i64 {
        self.inner.capacity
    }

    /// Score a route partition without running a heuristic.
    ///
    /// Args:
    ///     routes (list[list[int]]): One route per vehicle, holding customer indices in
    ///         visiting order. Every customer ``1..=n`` must appear exactly once across all
    ///         routes; the depot must not appear.
    ///
    /// Returns:
    ///     dict: ``{"objective": float, "distance": float, "overload": int,
    ///     "route_loads": list[int]}``. ``objective`` equals ``distance`` exactly when the
    ///     partition is feasible, i.e. when ``overload`` is 0.
    ///
    /// Raises:
    ///     ValueError: If ``routes`` is not a valid partition of the customers.
    fn evaluate_routes<'py>(
        &self,
        py: Python<'py>,
        routes: Vec<Vec<usize>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        self.inner
            .validate_routes(&routes)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let solution = self.inner.solution_from_routes(routes);
        let dict = PyDict::new(py);
        dict.set_item("objective", solution.objective)?;
        dict.set_item("distance", solution.distance)?;
        dict.set_item("overload", solution.overload)?;
        dict.set_item("route_loads", solution.route_loads)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "Vrp(name={:?}, num_customers={}, capacity={}, num_vehicles={})",
            self.inner.name,
            self.inner.get_n(),
            self.inner.capacity,
            self.inner.num_vehicles
        )
    }
}

/// A penalty-method formula problem over binary variables.
///
/// Build by passing the objective and any constraints as flat polynomials. Each polynomial is a
/// list of monomials, and each monomial is a tuple ``(variable_indices, coefficient)``. An empty
/// ``variable_indices`` list denotes a constant term.
///
/// A solution is encoded as a ``list[bool]`` of length ``n_vars``. ``RunResult.best_objective``
/// reports the direction-corrected score including any constraint penalties (higher is always
/// better).
///
/// Example:
///     Minimize ``x0 + x1 - 2*x0*x1`` with no constraints (an XOR-style objective)::
///
///         Formula(
///             n_vars=2,
///             objective=[([0], 1.0), ([1], 1.0), ([0, 1], -2.0)],
///             direction="Minimize",
///         )
#[pyclass(module = "optopus")]
pub struct Formula {
    pub inner: FormulaProblem,
}

#[pymethods]
impl Formula {
    /// Construct a new ``Formula`` problem.
    ///
    /// Args:
    ///     n_vars (int): Number of binary variables.
    ///     objective (list[tuple[list[int], float]]): The objective polynomial.
    ///     direction (str): ``"Maximize"`` or ``"Minimize"`` (default ``"Maximize"``).
    ///     constraints (list[tuple]): Optional list of penalty-weighted constraints. Each entry
    ///         is either a 4-tuple ``(lhs_poly, rel, rhs_poly, penalty_weight)`` where ``rel``
    ///         is one of ``"Lt"``, ``"Le"``, ``"Eq"``, ``"Ge"``, ``"Gt"``, or a 4-tuple
    ///         ``(expr_poly, "Clamp", (lo, hi), penalty_weight)`` for a range constraint.
    #[new]
    #[pyo3(signature = (n_vars, objective, direction="Maximize".to_string(), constraints=Vec::new()))]
    fn new(
        n_vars: usize,
        objective: PyPoly,
        direction: String,
        constraints: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let dir = parse_direction(&direction)?;
        let obj = poly_to_expr(&objective);
        let mut compiled = Vec::with_capacity(constraints.len());
        for c in constraints {
            compiled.push(extract_constraint(&c)?);
        }
        Ok(Self {
            inner: FormulaProblem::new(n_vars, obj, dir, compiled),
        })
    }

    fn __repr__(&self) -> String {
        let dir = match self.inner.direction {
            OptDirection::Maximize => "Maximize",
            OptDirection::Minimize => "Minimize",
        };
        format!(
            "Formula(n_vars={}, direction={dir:?}, n_constraints={})",
            self.inner.n_vars,
            self.inner.constraints.len()
        )
    }
}

/// Extracts a single [`Constraint`] from a Python tuple. Supports two shapes:
/// - ``(lhs_poly, rel, rhs_poly, penalty_weight)`` → [`Constraint::Comparison`].
/// - ``(expr_poly, "Clamp", (lo, hi), penalty_weight)`` → [`Constraint::Clamp`].
fn extract_constraint(c: &Bound<'_, PyAny>) -> PyResult<Constraint> {
    let tup: (Bound<'_, PyAny>, String, Bound<'_, PyAny>, f64) = c.extract().map_err(|_| {
        PyValueError::new_err(
            "constraint must be a 4-tuple (lhs_poly, rel, rhs_poly, penalty_weight) \
             or (expr_poly, 'Clamp', (lo, hi), penalty_weight)",
        )
    })?;
    let (lhs_any, rel, rhs_any, penalty_weight) = tup;
    if rel == "Clamp" {
        let expr_poly: PyPoly = lhs_any.extract()?;
        let (lo, hi): (f64, f64) = rhs_any.extract()?;
        Ok(Constraint::Clamp {
            expr: poly_to_expr(&expr_poly),
            lo,
            hi,
            penalty_weight,
        })
    } else {
        let lhs_poly: PyPoly = lhs_any.extract()?;
        let rhs_poly: PyPoly = rhs_any.extract()?;
        Ok(Constraint::Comparison {
            lhs: poly_to_expr(&lhs_poly),
            rel: parse_rel(&rel)?,
            rhs: poly_to_expr(&rhs_poly),
            penalty_weight,
        })
    }
}

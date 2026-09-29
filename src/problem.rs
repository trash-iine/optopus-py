use optopus::common::{Graph, seeded_rng};
use optopus::problem::{
    Constraint, ConstraintRel, Expr, FormulaProblem, GraphColoring as OptGraphColoring, IntVar,
    IntVars, JobShopScheduling as OptJobShop, MaxCut as OptMaxCut, MaxCutKernel as OptMaxCutKernel,
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
            "invalid relation '{other}' (use 'Lt', 'Le', 'Eq', 'Ge', 'Gt', their symbols \
             '<', '<=', '==', '>=', '>', or 'Clamp')"
        ))),
    }
}

/// Parses the optimization direction into "should maximize", which is the shape upstream's
/// `FormulaProblem::minimize` / `maximize` constructors take.
fn parse_direction(direction: &str) -> PyResult<bool> {
    match direction {
        "Maximize" | "max" => Ok(true),
        "Minimize" | "min" => Ok(false),
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
    /// ``(i, j, c)`` are the quadratic interaction terms. ``(i, j)`` and ``(j, i)`` name the
    /// same coefficient, and an entry given twice replaces the earlier one rather than adding
    /// to it, so sum the terms of a pair into one entry first.
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
    /// Raises:
    ///     ValueError: If a literal is 0 or names a variable above ``n_vars``.
    ///
    /// Example:
    ///     ``Sat.from_clauses(3, [[1, -2, 3], [-1, 2], [3]])``
    #[staticmethod]
    fn from_clauses(n_vars: usize, clauses: Vec<Vec<i64>>) -> PyResult<Self> {
        // Upstream panics on a literal it cannot index, so reject those here.
        for (k, clause) in clauses.iter().enumerate() {
            if let Some(&bad) = clause
                .iter()
                .find(|&&l| l == 0 || l.unsigned_abs() > n_vars as u64)
            {
                return Err(PyValueError::new_err(format!(
                    "clause {k} has literal {bad}, outside ±1..={n_vars}"
                )));
            }
        }
        let mut inner = OptSat::new(n_vars);
        for clause in clauses {
            inner.add_clause(clause);
        }
        Ok(Self { inner })
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

/// The Travelling Salesman Problem (minimization).
///
/// Find a tour visiting every city exactly once that minimizes the total distance. Distances
/// come either from 2D coordinates or from an explicit matrix.
///
/// A solution is encoded as a ``list[int]`` permutation of city indices ``[0, n)``.
#[pyclass(module = "optopus")]
pub struct Tsp {
    pub inner: OptTsp,
}

#[pymethods]
impl Tsp {
    /// Build a TSP instance from city coordinates.
    ///
    /// Args:
    ///     coordinates (list[tuple[float, float]]): ``(x, y)`` coordinates for each city.
    ///     name (str): Optional instance name (default ``""``).
    ///
    /// Returns:
    ///     Tsp: A new problem instance using continuous Euclidean distances.
    ///
    /// Raises:
    ///     ValueError: If ``coordinates`` is empty.
    #[staticmethod]
    #[pyo3(signature = (coordinates, name=String::new()))]
    fn from_coordinates(coordinates: Vec<(f64, f64)>, name: String) -> PyResult<Self> {
        if coordinates.is_empty() {
            return Err(PyValueError::new_err("'coordinates' must not be empty"));
        }
        Ok(Self {
            inner: OptTsp::new(name, coordinates),
        })
    }

    /// Build a TSP instance from an explicit distance matrix.
    ///
    /// Args:
    ///     matrix (list[list[float]]): Square, symmetric matrix of pairwise distances. The
    ///         moves assume a segment is as long in both directions.
    ///     name (str): Optional instance name (default ``""``).
    ///
    /// Returns:
    ///     Tsp: A new problem instance reading distances from the matrix.
    ///
    /// Raises:
    ///     ValueError: If the matrix is empty, not square, not symmetric, or holds a negative
    ///         or non-finite distance.
    #[staticmethod]
    #[pyo3(signature = (matrix, name=String::new()))]
    fn from_distance_matrix(matrix: Vec<Vec<f64>>, name: String) -> PyResult<Self> {
        check_distance_matrix(&matrix)?;
        OptTsp::from_distance_matrix(name, matrix)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Load a TSP instance from a TSPLIB-format file.
    ///
    /// Args:
    ///     path (str): Path to the instance file.
    ///
    /// Returns:
    ///     Tsp: The loaded problem instance.
    ///
    /// Raises:
    ///     ValueError: If the file cannot be read or does not parse.
    #[staticmethod]
    fn load_file(path: &str) -> PyResult<Self> {
        OptTsp::load_file(path)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Number of cities.
    fn num_cities(&self) -> usize {
        self.inner.get_n()
    }

    fn __repr__(&self) -> String {
        format!(
            "Tsp(name={:?}, n_cities={})",
            self.inner.name,
            self.inner.get_n()
        )
    }
}

/// The Graph Coloring problem (minimization).
///
/// Assign a color to every vertex so that no edge joins two vertices of the same color, using
/// as few colors as possible. Conflicts are penalized rather than forbidden, at a weight
/// (``num_vertices + 1``) high enough that removing any one conflict always beats saving a
/// color -- so the global optimum is a proper coloring with the fewest colors.
///
/// A solution is encoded as a ``list[int]`` of length *number of vertices*, holding each
/// vertex's color in ``[0, num_colors)``.
///
/// ``RunResult.best_objective`` reports ``colors_used + penalty_weight * conflicts``, so a
/// value below ``num_vertices + 1`` means the coloring is proper and the value is the number
/// of colors it used.
#[pyclass(module = "optopus")]
pub struct GraphColoring {
    pub inner: OptGraphColoring,
}

#[pymethods]
impl GraphColoring {
    /// Build a Graph Coloring instance from an explicit edge list.
    ///
    /// Args:
    ///     edges (list[tuple[int, int, float]]): Edges as ``(u, v, weight)``. Weights are
    ///         ignored; coloring is unweighted.
    ///     num_colors (int | None): Palette size. Defaults to None, which uses
    ///         ``max_degree + 1`` -- enough for a proper coloring to exist by Brooks' theorem.
    ///
    /// Returns:
    ///     GraphColoring: A new problem instance.
    ///
    /// Raises:
    ///     ValueError: If ``num_colors`` is 0.
    #[staticmethod]
    #[pyo3(signature = (edges, num_colors=None))]
    fn from_edges(edges: Vec<(usize, usize, f32)>, num_colors: Option<usize>) -> PyResult<Self> {
        Self::build(Graph::from_edges(edges), num_colors)
    }

    /// Build a Graph Coloring instance from a :class:`Graph`.
    ///
    /// Args:
    ///     graph (Graph): The graph to color.
    ///     num_colors (int | None): Palette size. Defaults to None (``max_degree + 1``).
    ///
    /// Returns:
    ///     GraphColoring: A new problem instance.
    ///
    /// Raises:
    ///     ValueError: If ``num_colors`` is 0.
    #[staticmethod]
    #[pyo3(signature = (graph, num_colors=None))]
    fn from_graph(graph: &PyGraph, num_colors: Option<usize>) -> PyResult<Self> {
        Self::build(graph.inner.clone(), num_colors)
    }

    /// Palette size: the number of colors a solution may use.
    fn num_colors(&self) -> usize {
        self.inner.k
    }

    /// The penalty charged per conflicting edge.
    fn penalty_weight(&self) -> i64 {
        self.inner.penalty_weight()
    }

    /// Score a color assignment without running a heuristic.
    ///
    /// Args:
    ///     colors (list[int]): One color per vertex, each in ``[0, num_colors)``.
    ///
    /// Returns:
    ///     dict: ``{"objective": int, "colors_used": int, "conflicts": int}``. A ``conflicts``
    ///     of 0 means the coloring is proper.
    ///
    /// Raises:
    ///     ValueError: If ``colors`` has the wrong length or holds a color outside the palette.
    fn evaluate_colors<'py>(
        &self,
        py: Python<'py>,
        colors: Vec<usize>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let n = self.inner.graph.num_vertices();
        if colors.len() != n {
            return Err(PyValueError::new_err(format!(
                "'colors' must have one entry per vertex ({n}), got {}",
                colors.len()
            )));
        }
        if let Some(bad) = colors.iter().find(|&&c| c >= self.inner.k) {
            return Err(PyValueError::new_err(format!(
                "color {bad} is outside the palette of {} colors",
                self.inner.k
            )));
        }
        let solution = self.inner.solution_from_colors(colors);
        let dict = PyDict::new(py);
        dict.set_item("objective", solution.objective)?;
        dict.set_item("colors_used", solution.colors_used)?;
        dict.set_item("conflicts", solution.conflicts)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "GraphColoring(num_vertices={}, num_edges={}, num_colors={})",
            self.inner.graph.num_vertices(),
            self.inner.graph.num_edges(),
            self.inner.k
        )
    }
}

impl GraphColoring {
    /// Shared by both factories: upstream derives the palette from the max degree, and a
    /// caller-supplied size overrides it.
    fn build(graph: Graph, num_colors: Option<usize>) -> PyResult<Self> {
        let mut inner = OptGraphColoring::new(graph);
        if let Some(k) = num_colors {
            if k == 0 {
                return Err(PyValueError::new_err("'num_colors' must be at least 1"));
            }
            inner.k = k;
        }
        Ok(Self { inner })
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
    check_demands(demands)
}

/// Rejects a negative demand, which would let a route carry more than its capacity.
fn check_demands(demands: &[i64]) -> PyResult<()> {
    match demands.iter().position(|&d| d < 0) {
        Some(i) => Err(PyValueError::new_err(format!(
            "demand {i} is negative ({})",
            demands[i]
        ))),
        None => Ok(()),
    }
}

/// Rejects a distance matrix the moves would price wrongly: a negative or non-finite entry, or
/// an asymmetric one, since a reversed segment is priced from the edges at its ends alone. A
/// matrix that is not square is left for upstream to report.
fn check_distance_matrix(matrix: &[Vec<f64>]) -> PyResult<()> {
    let n = matrix.len();
    if matrix.iter().any(|row| row.len() != n) {
        return Ok(());
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, &d) in row.iter().enumerate() {
            if !d.is_finite() || d < 0.0 {
                return Err(PyValueError::new_err(format!(
                    "distance [{i}][{j}] is {d}; distances must be finite and non-negative"
                )));
            }
            let e = matrix[j][i];
            if j > i && (d - e).abs() > 1e-9 * d.abs().max(e.abs()).max(1.0) {
                return Err(PyValueError::new_err(format!(
                    "the matrix must be symmetric, but [{i}][{j}] is {d} and [{j}][{i}] is {e}"
                )));
            }
        }
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
    ///     ValueError: If ``coordinates`` is empty, its length differs from ``demands``,
    ///         ``capacity`` is not positive, or a demand is negative.
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

    /// Build a CVRP instance from an explicit distance matrix.
    ///
    /// Args:
    ///     matrix (list[list[float]]): Square, symmetric matrix of pairwise distances. Index 0
    ///         is the depot. The moves assume a segment is as long in both directions.
    ///     demands (list[int]): Demand per node, same length as ``matrix``.
    ///     capacity (int): Vehicle capacity, ``> 0``.
    ///     num_vehicles (int): Fleet size, or 0 to let optopus pick one.
    ///     name (str): Optional instance name (default ``"vrp"``).
    ///
    /// Returns:
    ///     Vrp: A new problem instance reading distances from the matrix.
    ///
    /// Raises:
    ///     ValueError: If the matrix is empty, not square, not symmetric, or holds a negative
    ///         or non-finite distance, its size differs from ``demands``, ``capacity`` is not
    ///         positive, or a demand is negative.
    #[staticmethod]
    #[pyo3(signature = (matrix, demands, capacity, num_vehicles=0, name="vrp".to_string()))]
    fn from_distance_matrix(
        matrix: Vec<Vec<f64>>,
        demands: Vec<i64>,
        capacity: i64,
        num_vehicles: usize,
        name: String,
    ) -> PyResult<Self> {
        if matrix.len() != demands.len() {
            return Err(PyValueError::new_err(format!(
                "'matrix' and 'demands' must have the same length, got {} and {}",
                matrix.len(),
                demands.len()
            )));
        }
        if capacity <= 0 {
            return Err(PyValueError::new_err(format!(
                "'capacity' must be greater than 0, got {capacity}"
            )));
        }
        check_demands(&demands)?;
        check_distance_matrix(&matrix)?;
        OptVrp::from_distance_matrix(name, matrix, demands, capacity, num_vehicles)
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

    /// The penalty charged per unit of load over capacity.
    ///
    /// It is ``(num_customers + num_vehicles) * longest_edge + 1``, more than any set of routes
    /// can travel, so a solution within capacity always scores better than one over it.
    fn penalty_weight(&self) -> f64 {
        self.inner.penalty_weight()
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

/// A penalty-method formula problem over bounded integer variables.
///
/// Build by passing the objective and any constraints as flat polynomials. Each polynomial is a
/// list of monomials, and each monomial is a tuple ``(variable_indices, coefficient)``. An empty
/// ``variable_indices`` list denotes a constant term.
///
/// Variables are binary by default. Pass ``bounds`` to give them integer ranges instead, which
/// is what optopus's integer layer added -- a formula is no longer restricted to 0/1 variables.
///
/// A solution is encoded as a ``list[int]`` of length ``n_vars``. For binary variables the
/// entries are 0 and 1, which compare equal to ``False`` and ``True``.
/// ``RunResult.best_objective`` reports the direction-corrected score including any constraint
/// penalties (higher is always better).
///
/// Args:
///     n_vars (int): Number of variables.
///     objective (list[tuple[list[int], float]]): The objective polynomial.
///     direction (str): ``"Maximize"`` or ``"Minimize"``. Defaults to ``"Maximize"``.
///     constraints (list[tuple]): Penalty-weighted constraints. Defaults to none. Each entry
///         is a tuple ``(lhs_poly, rel, rhs_poly, penalty_weight)`` where ``rel`` is one of
///         ``"Lt"``, ``"Le"``, ``"Eq"``, ``"Ge"``, ``"Gt"`` or the symbols ``"<"``, ``"<="``,
///         ``"=="`` (or ``"="``), ``">="``, ``">"``, or a tuple
///         ``(expr_poly, "Clamp", (lo, hi), penalty_weight)`` for a range constraint. A
///         violated constraint costs ``penalty_weight`` times the amount of the violation;
///         see "Modeling with Formula" in the documentation.
///     bounds (list[tuple[int, int]] | None): Inclusive ``(lower, upper)`` range per
///         variable. Defaults to None, which makes every variable binary.
///
/// Raises:
///     ValueError: If ``n_vars`` is 0, ``bounds`` has the wrong length, a bound is inverted,
///         the direction or a relation is unrecognized, a ``penalty_weight`` is negative or
///         not finite, a ``Clamp`` range has ``lo`` above ``hi``, or the objective or a
///         constraint reads a variable index outside ``[0, n_vars)``.
///
/// Example:
///     Minimize ``x0 + x1 - 2*x0*x1`` over two binary variables (an XOR-style objective)::
///
///         Formula(
///             n_vars=2,
///             objective=[([0], 1.0), ([1], 1.0), ([0, 1], -2.0)],
///             direction="Minimize",
///         )
///
///     The same objective over variables ranging in ``[0, 5]``::
///
///         Formula(
///             n_vars=2,
///             objective=[([0], 1.0), ([1], 1.0), ([0, 1], -2.0)],
///             direction="Minimize",
///             bounds=[(0, 5), (0, 5)],
///         )
#[pyclass(module = "optopus")]
pub struct Formula {
    pub inner: FormulaProblem,
    /// Whether the problem maximizes. Upstream keeps the direction private and folds it into
    /// the `Evaluable` a solution reports, but the runner needs it up front to aggregate a
    /// report in the right direction.
    pub maximize: bool,
}

#[pymethods]
impl Formula {
    #[new]
    #[pyo3(
        signature = (n_vars, objective, direction="Maximize".to_string(), constraints=Vec::new(), bounds=None),
        text_signature = "(n_vars, objective, direction='Maximize', constraints=[], bounds=None)"
    )]
    fn new(
        n_vars: usize,
        objective: PyPoly,
        direction: String,
        constraints: Vec<Bound<'_, PyAny>>,
        bounds: Option<Vec<(i64, i64)>>,
    ) -> PyResult<Self> {
        if n_vars == 0 {
            return Err(PyValueError::new_err("'n_vars' must be at least 1"));
        }
        let maximize = parse_direction(&direction)?;
        let vars = build_int_vars(n_vars, bounds)?;

        // Upstream panics when an expression reads a variable outside `vars`, so check the
        // indices of the objective and of every constraint here and report them as a
        // `ValueError` instead.
        let mut compiled = Vec::with_capacity(constraints.len());
        for c in constraints {
            compiled.push(extract_constraint(&c, n_vars)?);
        }
        check_var_indices(n_vars, &objective)?;

        let obj = poly_to_expr(&objective);
        let mut inner = if maximize {
            FormulaProblem::maximize(vars, obj)
        } else {
            FormulaProblem::minimize(vars, obj)
        };
        for constraint in compiled {
            inner = inner.with_constraint(constraint);
        }
        Ok(Self { inner, maximize })
    }

    /// Number of variables.
    fn n_vars(&self) -> usize {
        self.inner.variables().len()
    }

    /// The objective expression's value for an assignment, before any constraint penalty.
    ///
    /// Args:
    ///     values (list[int]): One value per variable.
    ///
    /// Returns:
    ///     float: The raw objective.
    ///
    /// Raises:
    ///     ValueError: If ``values`` has the wrong length.
    fn eval_objective(&self, values: Vec<i64>) -> PyResult<f64> {
        self.check_values(&values)?;
        Ok(self.inner.eval_objective(&values))
    }

    /// The total constraint penalty for an assignment. Zero means every constraint holds.
    ///
    /// Args:
    ///     values (list[int]): One value per variable.
    ///
    /// Returns:
    ///     float: The summed penalty.
    ///
    /// Raises:
    ///     ValueError: If ``values`` has the wrong length.
    fn eval_penalty(&self, values: Vec<i64>) -> PyResult<f64> {
        self.check_values(&values)?;
        Ok(self.inner.eval_penalty(&values))
    }

    fn __repr__(&self) -> String {
        let dir = if self.maximize {
            "Maximize"
        } else {
            "Minimize"
        };
        format!(
            "Formula(n_vars={}, direction={dir:?}, binary={})",
            self.inner.variables().len(),
            self.inner
                .variables()
                .iter()
                .all(|v| v.lower() == 0 && v.upper() == 1)
        )
    }
}

impl Formula {
    fn check_values(&self, values: &[i64]) -> PyResult<()> {
        let expected = self.inner.variables().len();
        if values.len() != expected {
            return Err(PyValueError::new_err(format!(
                "'values' must have one entry per variable ({expected}), got {}",
                values.len()
            )));
        }
        Ok(())
    }
}

/// Builds the variable list, binary unless explicit bounds were given.
fn build_int_vars(n_vars: usize, bounds: Option<Vec<(i64, i64)>>) -> PyResult<IntVars> {
    let vars = match bounds {
        None => vec![IntVar::binary(); n_vars],
        Some(bounds) => {
            if bounds.len() != n_vars {
                return Err(PyValueError::new_err(format!(
                    "'bounds' must have one entry per variable ({n_vars}), got {}",
                    bounds.len()
                )));
            }
            bounds
                .into_iter()
                .enumerate()
                .map(|(i, (lower, upper))| {
                    if lower > upper {
                        return Err(PyValueError::new_err(format!(
                            "bound {i} has lower ({lower}) above upper ({upper})"
                        )));
                    }
                    Ok(IntVar::new(lower, upper))
                })
                .collect::<PyResult<Vec<_>>>()?
        }
    };
    Ok(IntVars::new(vars))
}

/// Rejects a polynomial that reads a variable index the problem does not have.
fn check_var_indices(n_vars: usize, poly: &PyPoly) -> PyResult<()> {
    for (vars, _) in poly {
        if let Some(&bad) = vars.iter().find(|&&v| v >= n_vars) {
            return Err(PyValueError::new_err(format!(
                "variable index {bad} is outside [0, {n_vars})"
            )));
        }
    }
    Ok(())
}

/// Extracts a single [`Constraint`] from a Python tuple. Supports two shapes:
/// - ``(lhs_poly, rel, rhs_poly, penalty_weight)`` → [`Constraint::Comparison`].
/// - ``(expr_poly, "Clamp", (lo, hi), penalty_weight)`` → [`Constraint::Clamp`].
///
/// Every polynomial must read only variables in `[0, n_vars)`.
fn extract_constraint(c: &Bound<'_, PyAny>, n_vars: usize) -> PyResult<Constraint> {
    let tup: (Bound<'_, PyAny>, String, Bound<'_, PyAny>, f64) = c.extract().map_err(|_| {
        PyValueError::new_err(
            "constraint must be a 4-tuple (lhs_poly, rel, rhs_poly, penalty_weight) \
             or (expr_poly, 'Clamp', (lo, hi), penalty_weight)",
        )
    })?;
    let (lhs_any, rel, rhs_any, penalty_weight) = tup;
    // A negative weight would reward breaking the constraint.
    if !(penalty_weight >= 0.0 && penalty_weight.is_finite()) {
        return Err(PyValueError::new_err(format!(
            "penalty_weight must be finite and non-negative, got {penalty_weight}"
        )));
    }
    if rel == "Clamp" {
        let expr_poly: PyPoly = lhs_any.extract()?;
        check_var_indices(n_vars, &expr_poly)?;
        let (lo, hi): (f64, f64) = rhs_any.extract()?;
        if lo > hi {
            return Err(PyValueError::new_err(format!(
                "Clamp range has lo ({lo}) above hi ({hi})"
            )));
        }
        Ok(Constraint::Clamp {
            expr: poly_to_expr(&expr_poly),
            lo,
            hi,
            penalty_weight,
        })
    } else {
        let lhs_poly: PyPoly = lhs_any.extract()?;
        let rhs_poly: PyPoly = rhs_any.extract()?;
        check_var_indices(n_vars, &lhs_poly)?;
        check_var_indices(n_vars, &rhs_poly)?;
        Ok(Constraint::Comparison {
            lhs: poly_to_expr(&lhs_poly),
            rel: parse_rel(&rel)?,
            rhs: poly_to_expr(&rhs_poly),
            penalty_weight,
        })
    }
}

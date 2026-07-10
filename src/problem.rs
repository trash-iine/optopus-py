use optopus::common::Graph;
use optopus::problem::{
    Constraint, ConstraintRel, Expr, FormulaProblem, JobShopScheduling as OptJobShop,
    MaxCut as OptMaxCut, OptDirection, Qubo as OptQubo, Sat as OptSat,
    TspWithCoordinates as OptTsp, VertexCover as OptVc,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

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

    fn __repr__(&self) -> String {
        format!(
            "MaxCut(num_vertices={}, num_edges={})",
            self.inner.graph.num_vertices(),
            self.inner.graph.num_edges()
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
            self.inner.coordinates.len()
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

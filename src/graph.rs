use std::time::{SystemTime, UNIX_EPOCH};

use optopus::prelude::{Graph as OptGraph, seeded_rng};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Largest integer magnitude that survives a round trip through `f32`, matching
/// the bound `Graph::with_random_weights` enforces on its weight range.
const MAX_EXACT_F32_INT: u64 = 1 << 24;

/// Resolves the seed for a generator call. `None` draws a fresh one from the
/// clock, so unseeded calls vary between runs while seeded ones reproduce.
fn resolve_seed(seed: Option<u64>) -> u64 {
    seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    })
}

/// An undirected weighted graph, and the entry point for the random graph
/// generators.
///
/// Vertices are 0-indexed and inferred from the edges. The generators produce
/// unweighted graphs (every weight 1.0); chain
/// :meth:`with_random_weights` to draw integer weights instead.
///
/// Example:
///     >>> g = Graph.erdos_renyi(100, 0.05, seed=42).with_random_weights((1, 10), seed=42)
///     >>> problem = MaxCut.from_graph(g)
#[pyclass(module = "optopus")]
pub struct Graph {
    pub inner: OptGraph,
}

#[pymethods]
impl Graph {
    /// Build a graph from an explicit edge list.
    ///
    /// Args:
    ///     edges (list[tuple[int, int, float]]): Edges as ``(u, v, weight)``.
    ///
    /// Returns:
    ///     Graph: The graph spanned by those edges.
    #[staticmethod]
    fn from_edges(edges: Vec<(usize, usize, f32)>) -> Self {
        Self {
            inner: OptGraph::from_edges(edges),
        }
    }

    /// Erdős-Rényi ``G(n, p)``: every possible edge is present independently
    /// with probability ``p``.
    ///
    /// Args:
    ///     n (int): Number of vertices (``>= 1``).
    ///     p (float): Edge probability in ``[0, 1]``.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     Graph: The generated graph, with all weights set to 1.0.
    ///
    /// Raises:
    ///     ValueError: If ``n`` is 0 or ``p`` is outside ``[0, 1]``.
    #[staticmethod]
    #[pyo3(signature = (n, p, seed=None))]
    fn erdos_renyi(n: usize, p: f64, seed: Option<u64>) -> PyResult<Self> {
        if n == 0 {
            return Err(PyValueError::new_err("'n' must be at least 1"));
        }
        if !(0.0..=1.0).contains(&p) {
            return Err(PyValueError::new_err(format!(
                "'p' must be within [0.0, 1.0], got {p}"
            )));
        }
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptGraph::erdos_renyi(n, p, &mut rng),
        })
    }

    /// Barabási-Albert preferential attachment: starts from a clique of ``m``
    /// vertices and attaches each remaining vertex with ``m`` degree-biased edges.
    ///
    /// The edge count is deterministic: ``m * (m - 1) / 2 + m * (n - m)``.
    ///
    /// Args:
    ///     n (int): Number of vertices.
    ///     m (int): Edges added per new vertex (``1 <= m < n``).
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     Graph: The generated graph, with all weights set to 1.0.
    ///
    /// Raises:
    ///     ValueError: If ``m`` is 0 or ``m >= n``.
    #[staticmethod]
    #[pyo3(signature = (n, m, seed=None))]
    fn barabasi_albert(n: usize, m: usize, seed: Option<u64>) -> PyResult<Self> {
        if m == 0 {
            return Err(PyValueError::new_err("'m' must be at least 1"));
        }
        if m >= n {
            return Err(PyValueError::new_err(format!(
                "'m' must be less than 'n', got m={m}, n={n}"
            )));
        }
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptGraph::barabasi_albert(n, m, &mut rng),
        })
    }

    /// Watts-Strogatz small world: a ring lattice where each vertex is joined to
    /// its ``k`` nearest neighbors, then each edge is rewired with probability ``beta``.
    ///
    /// The edge count is always ``n * k / 2``.
    ///
    /// Args:
    ///     n (int): Number of vertices.
    ///     k (int): Neighbors per vertex; must be a positive even number below ``n``.
    ///     beta (float): Rewiring probability in ``[0, 1]``.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     Graph: The generated graph, with all weights set to 1.0.
    ///
    /// Raises:
    ///     ValueError: If ``k`` is 0, odd, or ``>= n``, or ``beta`` is outside ``[0, 1]``.
    #[staticmethod]
    #[pyo3(signature = (n, k, beta, seed=None))]
    fn watts_strogatz(n: usize, k: usize, beta: f64, seed: Option<u64>) -> PyResult<Self> {
        if k == 0 {
            return Err(PyValueError::new_err("'k' must be at least 1"));
        }
        if !k.is_multiple_of(2) {
            return Err(PyValueError::new_err(format!("'k' must be even, got {k}")));
        }
        if k >= n {
            return Err(PyValueError::new_err(format!(
                "'k' must be less than 'n', got k={k}, n={n}"
            )));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(PyValueError::new_err(format!(
                "'beta' must be within [0.0, 1.0], got {beta}"
            )));
        }
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: OptGraph::watts_strogatz(n, k, beta, &mut rng),
        })
    }

    /// Return a copy of this graph with every edge weight redrawn uniformly from
    /// ``weight_range``.
    ///
    /// Zero is never drawn, so a range spanning zero yields no zero-weight edges.
    ///
    /// Args:
    ///     weight_range (tuple[int, int]): Inclusive ``(min, max)`` bounds.
    ///     seed (int | None): Seed for reproducibility. Defaults to None (nondeterministic).
    ///
    /// Returns:
    ///     Graph: A new graph with the same edges and fresh weights.
    ///
    /// Raises:
    ///     ValueError: If ``min > max``, the range is ``(0, 0)``, or a bound exceeds
    ///         the exactly ``f32``-representable range of ±2**24.
    #[pyo3(signature = (weight_range, seed=None))]
    fn with_random_weights(&self, weight_range: (i64, i64), seed: Option<u64>) -> PyResult<Self> {
        let (min, max) = weight_range;
        if min > max {
            return Err(PyValueError::new_err(format!(
                "'weight_range' min ({min}) must be <= max ({max})"
            )));
        }
        if (min, max) == (0, 0) {
            return Err(PyValueError::new_err(
                "'weight_range' (0, 0) contains no nonzero weight",
            ));
        }
        if min.unsigned_abs() > MAX_EXACT_F32_INT || max.unsigned_abs() > MAX_EXACT_F32_INT {
            return Err(PyValueError::new_err(format!(
                "'weight_range' ({min}, {max}) exceeds the exactly f32-representable \
                 range of +/-{MAX_EXACT_F32_INT}"
            )));
        }
        let mut rng = seeded_rng(resolve_seed(seed));
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_random_weights(weight_range, &mut rng),
        })
    }

    /// Number of vertices that appear in at least one edge.
    fn num_vertices(&self) -> usize {
        self.inner.num_vertices()
    }

    /// Number of undirected edges.
    fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    /// The edge list.
    ///
    /// Returns:
    ///     list[tuple[int, int, float]]: Each undirected edge once, as ``(u, v, weight)``.
    fn edges(&self) -> Vec<(usize, usize, f32)> {
        self.inner.edges().collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Graph(num_vertices={}, num_edges={})",
            self.inner.num_vertices(),
            self.inner.num_edges()
        )
    }
}

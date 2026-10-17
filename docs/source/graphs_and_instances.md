# Graphs and test instances

`MaxCut`, `VertexCover` and `GraphColoring` are defined over an undirected weighted graph. This
page covers `Graph`, which builds one from an edge list, draws one from a random model or lays
out a lattice, and two Max Cut tools: `PlantedMaxCut`, instances whose optimum is known, and
`MaxCutKernel`, which shrinks an instance exactly before the search.

## Graph

Vertices are numbered from 0 and taken from the edges. Every constructor is a static method,
and the random ones take an optional `seed`: the same seed draws the same graph, and `None`
draws a fresh one every call.

| Constructor | Builds |
|---|---|
| `Graph.from_edges(edges)` | the graph of `(u, v, weight)` triples |
| `Graph.erdos_renyi(n, p, seed=None)` | Erdős-Rényi G(n, p): every one of the `n (n − 1) / 2` possible edges present with probability `p` |
| `Graph.barabasi_albert(n, m, seed=None)` | Barabási-Albert preferential attachment, scale-free: a clique of `m` vertices, then every further vertex joined to `m` existing ones by degree; `m (m − 1) / 2 + m (n − m)` edges, `1 <= m < n` |
| `Graph.watts_strogatz(n, k, beta, seed=None)` | Watts-Strogatz small world: a ring where each vertex is joined to its `k` nearest, every edge rewired with probability `beta`; `n k / 2` edges, `k` even and below `n` |
| `Graph.grid_torus_2d(l)` | the periodic square lattice: `l²` vertices, `2 l²` edges, 4-regular, `l >= 3`; vertex `m + l · n` sits at `(m, n)` |
| `Graph.grid_torus_3d(l)` | the periodic cubic lattice: `l³` vertices, `3 l³` edges, 6-regular, `l >= 3`; vertex `m + l · n + l² · k` sits at `(m, n, k)` |

| Method | Returns |
|---|---|
| `with_random_weights(weight_range, seed=None)` | a copy with every weight drawn uniformly from the inclusive `(min, max)` integer range, never 0; bounds within ±2²⁴ |
| `num_vertices()` | the number of vertices with at least one edge |
| `num_edges()` | the number of edges |
| `edges()` | every edge once, as `(u, v, weight)` |

```python
import optopus

g = optopus.Graph.from_edges([(0, 1, 1.0), (1, 2, 2.0), (2, 0, 3.0), (2, 3, 1.0)])
print(g)          # Graph(num_vertices=4, num_edges=4)
print(g.edges())  # [(0, 1, 1.0), (0, 2, 3.0), (1, 2, 2.0), (2, 3, 1.0)]

print(optopus.Graph.barabasi_albert(100, 2, seed=42))     # Graph(num_vertices=100, num_edges=197)
print(optopus.Graph.watts_strogatz(100, 4, 0.1, seed=42))  # Graph(num_vertices=100, num_edges=200)
print(optopus.Graph.grid_torus_2d(8))                      # Graph(num_vertices=64, num_edges=128)
```

The generators and lattices produce unweighted graphs, every weight 1.0. Chain
`with_random_weights` for integer weights, such as `(-1, 1)` for a ±J spin glass on a lattice:

```python
spin_glass = optopus.Graph.grid_torus_2d(8).with_random_weights((-1, 1), seed=3)
print(sorted({w for _, _, w in spin_glass.edges()}))  # [-1.0, 1.0] — zero is never drawn
```

Every graph problem takes a graph through `from_graph`:

```python
g = optopus.Graph.erdos_renyi(200, 0.05, seed=42).with_random_weights((1, 10), seed=42)

mc = optopus.MaxCut.from_graph(g)
vc = optopus.VertexCover.from_graph(g)
gc = optopus.GraphColoring.from_graph(g, num_colors=8)
print(mc)  # MaxCut(num_vertices=200, num_edges=1017)
```

A vertex with no edge is not counted by `num_vertices()`, though a solution still has one entry
per index up to the largest that appears in an edge. A random graph sparse enough to leave
vertices isolated reports fewer than `n`: `Graph.erdos_renyi(100, 0.05, seed=42)` has 98.

## PlantedMaxCut

On a random instance you cannot tell a good result from an optimal one. `PlantedMaxCut` plants
an optimal cut by construction and then disguises it, so the optimum is known exactly and
"how often does this heuristic reach the optimum" becomes a question you can count.

| Constructor | Builds |
|---|---|
| `PlantedMaxCut.wishart(n, alpha, couplers="Gaussian", seed=None)` | the Wishart planted ensemble on the complete graph of `n >= 2` vertices; `alpha` in `(0, 1)`, smaller is harder; `couplers="Discrete"` gives integer weights, which keep the optimum exact |
| `PlantedMaxCut.tile_planting_2d(l, p1, p2, p3, seed=None)` | tile planting on the `l × l` torus, `l` even and `>= 4`; `p1`, `p2`, `p3` are the probabilities of three of the four tile classes, the fourth takes the rest |
| `PlantedMaxCut.tile_planting_3d(l, p_2fp, p_4fp, seed=None)` | tile planting on the `l³` torus; the probabilities of the 2- and 4-frustrated-plaquette classes, the 6-frustrated one takes the rest |

| Method | Returns |
|---|---|
| `problem()` | the instance, an ordinary `MaxCut` |
| `planted()` | the planted assignment, an optimal cut, one `bool` per vertex |
| `optimum()` | its cut weight, the optimum |
| `has_exact_optimum()` | whether every weight is an integer, so that the reported objective can equal `optimum()` exactly |
| `verify()` | re-checks the construction; raises `ValueError` describing the first failure |

```python
# Integer couplers make the optimum survive the f32 objective exactly, so the hit test is
# an equality rather than a judgement call about rounding.
planted = optopus.PlantedMaxCut.wishart(64, 0.75, couplers="Discrete", seed=1)
planted.verify()
print(planted)  # PlantedMaxCut(num_vertices=64, num_edges=2016, optimum=3056960, exact=true)

bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(6, 12), t=1_000, l0=10, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=50_000),
)
report = bls.run(planted.problem(), runs=20, seed=42)

hits = sum(1 for r in report.runs if r.best_objective == planted.optimum())
print(f"reached the optimum in {hits}/{len(report.runs)} runs")
```

With the default Gaussian couplers the weights are real numbers stored as 32-bit floats, and
`has_exact_optimum()` is `False`: compare with `math.isclose` instead. The tile-planting
lattices always have integer weights; their class probabilities tune the hardness.

```python
tiles = optopus.PlantedMaxCut.tile_planting_2d(8, 0.2, 0.3, 0.3, seed=1)
print(tiles.optimum(), tiles.has_exact_optimum())  # 54.0 True
```

## MaxCutKernel

`MaxCutKernel.reduce(problem)` applies exact reduction rules, isolated and pendant vertices,
degree-2 paths and weight domination, until none fires. The kernel is an ordinary `MaxCut`, so
any heuristic runs on it unchanged; only the instance it sees gets smaller.

| Method | Returns |
|---|---|
| `MaxCutKernel.reduce(problem)` | the reduction of a `MaxCut` |
| `kernel()` | the reduced instance, a `MaxCut` |
| `offset()` | the constant to add to a kernel cut to get the original one |
| `is_trivial()` | whether nothing could be removed, as on regular and dense graphs |
| `removed_vertices()` | how many vertices the rules removed |
| `lift(assignment)` | a kernel assignment mapped back to the original vertices, the removed ones set optimally |
| `project(assignment)` | an original assignment restricted to the kernel, for a warm start |

For every kernel assignment `y`, `kernel_cut(y) + offset == original_cut(lift(y))`, so the
lifted solution is exactly as good as the reported value claims.

```python
mc = optopus.MaxCut.from_graph(
    optopus.Graph.barabasi_albert(2_000, 2, seed=7).with_random_weights((1, 10), seed=7)
)

kernel = optopus.MaxCutKernel.reduce(mc)
print(kernel)
# MaxCutKernel(original_vertices=2000, kernel_vertices=771, removed=1229, offset=12841)

stop = optopus.StopCondition(max_iteration=100_000)
if kernel.is_trivial():
    # Regular and dense graphs reduce to themselves; skip the indirection.
    report = optopus.LocalSearch("Flip", stop).run(mc, runs=4, seed=1)
    best, assignment = report.best_objective, report.runs[0].solution
else:
    report = optopus.LocalSearch("Flip", stop).run(kernel.kernel(), runs=4, seed=1)
    best = report.best_objective + kernel.offset()
    assignment = kernel.lift(report.runs[0].solution)

print(f"cut {best} over {len(assignment)} vertices")
```

`project` goes the other way: a solution of the original, restricted to the kernel, starts a
search there through `initial_solution` (see
[Starting from a solution](running.md#starting-from-a-solution)):

```python
start = kernel.project(assignment)
tabu = optopus.TabuSearch(
    neighbor="Flip", tabu_tenure=(5, 10), stop=optopus.StopCondition(max_iteration=200)
)
again = tabu.run(kernel.kernel(), seed=2, initial_solution=start)
```

The rules bite on sparse, irregular graphs, such as the many low-degree vertices of a
Barabási-Albert graph, and do nothing on lattices: `MaxCutKernel.reduce` of a
`grid_torus_2d` instance is trivial.

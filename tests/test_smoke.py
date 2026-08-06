"""Smoke tests exercising the built extension end-to-end.

The instances are tiny so every heuristic reaches the known optimum within the
iteration budget regardless of seed.
"""

import pytest

import optopus

# The triangle's optimum cut puts one vertex alone, cutting the two heaviest edges.
TRIANGLE = [(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)]
TRIANGLE_OPTIMUM = 5.0

UNIT_SQUARE = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


def test_maxcut_simulated_annealing():
    mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])
    sa = optopus.SimulatedAnnealing(
        neighbor="Flip",
        initial_temperature=10.0,
        cooling_rate=0.99,
        stop=optopus.StopCondition(max_iteration=10_000),
    )
    report = sa.run(mc, runs=3, seed=42)
    assert report.best_objective == 5.0
    assert len(report.runs) == 3
    assert len(report.runs[0].solution) == 3


def test_qubo_local_search():
    q = optopus.Qubo.from_entries([(0, 0, -1), (0, 1, 2), (1, 1, -1)])
    ls = optopus.LocalSearch(
        neighbor="Flip",
        stop=optopus.StopCondition(max_iteration=1_000),
    )
    report = ls.run(q, runs=2, seed=7)
    assert report.best_objective == -1.0
    assert len(report.runs[0].solution) == 2


def test_tsp_local_search():
    tsp = optopus.TspWithCoordinates.from_coordinates(
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        name="unit-square",
    )
    ls = optopus.LocalSearch(
        neighbor="TwoOpt",
        stop=optopus.StopCondition(max_iteration=10_000),
    )
    report = ls.run(tsp, runs=2, seed=0)
    assert abs(report.best_objective - 4.0) < 1e-9
    assert sorted(report.runs[0].solution) == [0, 1, 2, 3]


# --- Graph generators -------------------------------------------------------


def test_graph_from_edges():
    g = optopus.Graph.from_edges(TRIANGLE)
    assert g.num_vertices() == 3
    assert g.num_edges() == 3
    assert sorted(g.edges()) == [(0, 1, 1.0), (0, 2, 3.0), (1, 2, 2.0)]


def test_graph_erdos_renyi_is_reproducible():
    a = optopus.Graph.erdos_renyi(60, 0.1, seed=42)
    b = optopus.Graph.erdos_renyi(60, 0.1, seed=42)
    assert a.edges() == b.edges()
    assert a.num_edges() > 0


def test_graph_barabasi_albert_edge_count():
    n, m = 50, 3
    g = optopus.Graph.barabasi_albert(n, m, seed=1)
    # A clique of m vertices, then m edges for each of the remaining n - m.
    assert g.num_edges() == m * (m - 1) // 2 + m * (n - m)


def test_graph_watts_strogatz_edge_count():
    n, k = 40, 6
    g = optopus.Graph.watts_strogatz(n, k, 0.2, seed=1)
    # Rewiring moves edges around but never changes how many there are.
    assert g.num_edges() == n * k // 2


def test_graph_with_random_weights_stays_in_range_and_skips_zero():
    g = optopus.Graph.watts_strogatz(30, 4, 0.2, seed=1).with_random_weights((-5, 5), seed=2)
    weights = [w for _, _, w in g.edges()]
    assert weights
    assert all(-5.0 <= w <= 5.0 and w != 0.0 for w in weights)


def test_problems_from_graph():
    g = optopus.Graph.from_edges(TRIANGLE)
    assert optopus.MaxCut.from_graph(g).__repr__() == optopus.MaxCut.from_edges(TRIANGLE).__repr__()
    assert (
        optopus.VertexCover.from_graph(g).__repr__()
        == optopus.VertexCover.from_edges(TRIANGLE).__repr__()
    )


def test_generated_graph_feeds_maxcut():
    g = optopus.Graph.erdos_renyi(40, 0.15, seed=42).with_random_weights((1, 10), seed=42)
    mc = optopus.MaxCut.from_graph(g)
    report = optopus.LocalSearch("Flip", stop=stop(1_000)).run(mc, seed=42)
    assert report.best_objective > 0.0
    assert len(report.runs[0].solution) == g.num_vertices()


# --- New heuristics ---------------------------------------------------------


def test_variable_neighborhood_search():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    vns = optopus.VariableNeighborhoodSearch(
        search=optopus.LocalSearch("Flip", stop=stop(50)),
        # Each step keeps its own neighborhood and its own budget.
        shakes=[
            optopus.RandomWalk("Flip", stop=stop(3)),
            optopus.RandomWalk("Swap", stop=stop(6)),
        ],
        stop=stop(2_000),
    )
    report = vns.run(mc, runs=3, seed=42)
    assert report.best_objective == TRIANGLE_OPTIMUM


def test_variable_neighborhood_search_with_problem_specific_step():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    vns = optopus.VariableNeighborhoodSearch(
        search=optopus.BreakoutLocalSearch((3, 50), 100, 5, 0.8, 0.5, stop(100)),
        shakes=[optopus.RandomWalk("Flip", stop=stop(3))],
        stop=stop(500),
    )
    assert vns.run(mc, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_walksat_satisfies_all_clauses():
    # (x1 or x2) and (not x1 or x3) and (x2 or not x3) -- satisfiable.
    sat = optopus.Sat.from_clauses(3, [[1, 2], [-1, 3], [2, -3]])
    report = optopus.WalkSat(stop=stop(1_000)).run(sat, runs=3, seed=42)
    assert report.best_objective == 3.0


def test_walksat_adaptive_noise():
    sat = optopus.Sat.from_clauses(3, [[1, 2], [-1, 3], [2, -3]])
    report = optopus.WalkSat(stop=stop(1_000), noise=0.5, adaptive=True).run(sat, seed=42)
    assert report.best_objective == 3.0


def test_population_annealing():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    pa = optopus.PopulationAnnealing(population_size=10, stop=stop(50), sweeps_per_step=5)
    assert pa.run(mc, runs=2, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_breakout_local_search():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bls = optopus.BreakoutLocalSearch((3, 50), 1_000, 20, 0.8, 0.5, stop(1_000))
    assert bls.run(mc, runs=2, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_breakout_local_search_with_plateau_moves():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bls = optopus.BreakoutLocalSearch(
        (3, 50), 1_000, 20, 0.8, 0.5, stop(1_000), plateau_prob=0.5
    )
    assert bls.run(mc, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_rl_breakout_local_search():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    rl = optopus.RlBreakoutLocalSearch((3, 50), 1_000, 20, stop(1_000))
    assert rl.run(mc, runs=2, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_rl_breakout_local_search_accepts_policy_weights():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bins = [1.0, 2.0]
    # 5 perturbation types x len(bins) strengths x 8 context features.
    rl = optopus.RlBreakoutLocalSearch(
        (3, 50), 1_000, 20, stop(500), strength_bins=bins, policy_weights=[0.0] * (5 * 2 * 8)
    )
    assert rl.run(mc, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_lin_kernighan_helsgaun():
    tsp = optopus.TspWithCoordinates.from_coordinates(UNIT_SQUARE, name="unit-square")
    report = optopus.LinKernighanHelsgaun(stop=stop(200)).run(tsp, runs=2, seed=42)
    assert abs(report.best_objective - 4.0) < 1e-9
    assert sorted(report.runs[0].solution) == [0, 1, 2, 3]


# --- Determinism ------------------------------------------------------------


def test_same_seed_reproduces_report():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bls = optopus.BreakoutLocalSearch((3, 50), 1_000, 20, 0.8, 0.5, stop(1_000))
    first = bls.run(mc, runs=3, seed=123)
    second = bls.run(mc, runs=3, seed=123)
    assert [r.best_objective for r in first.runs] == [r.best_objective for r in second.runs]
    assert [r.seed for r in first.runs] == [r.seed for r in second.runs]


# --- Misuse -----------------------------------------------------------------


@pytest.mark.parametrize(
    "make",
    [
        pytest.param(lambda: optopus.WalkSat(stop=stop(10), noise=1.5), id="noise-out-of-range"),
        pytest.param(
            lambda: optopus.PopulationAnnealing(population_size=1, stop=stop(10)),
            id="population-too-small",
        ),
        pytest.param(
            lambda: optopus.PopulationAnnealing(population_size=5, stop=stop(10), delta_beta=0.0),
            id="delta-beta-not-positive",
        ),
        pytest.param(
            lambda: optopus.BreakoutLocalSearch(
                (3, 50), 100, 5, 0.8, 0.5, stop(10), plateau_prob=2.0
            ),
            id="plateau-prob-out-of-range",
        ),
        pytest.param(
            lambda: optopus.RlBreakoutLocalSearch((3, 50), 100, 5, stop(10), strength_bins=[]),
            id="empty-strength-bins",
        ),
        pytest.param(
            lambda: optopus.RlBreakoutLocalSearch((3, 50), 100, 5, stop(10), policy_weights=[0.0]),
            id="policy-weights-wrong-length",
        ),
        pytest.param(
            lambda: optopus.VariableNeighborhoodSearch(
                optopus.LocalSearch("Flip", stop(10)), [], stop(10)
            ),
            id="empty-shakes",
        ),
        pytest.param(lambda: optopus.Graph.erdos_renyi(10, 1.5), id="probability-out-of-range"),
        pytest.param(lambda: optopus.Graph.barabasi_albert(5, 5), id="m-not-below-n"),
        pytest.param(lambda: optopus.Graph.watts_strogatz(10, 3, 0.2), id="k-odd"),
        pytest.param(
            lambda: optopus.Graph.from_edges(TRIANGLE).with_random_weights((0, 0)),
            id="all-zero-weight-range",
        ),
    ],
)
def test_invalid_parameters_raise_value_error(make):
    """Upstream asserts on these, so the binding must reject them before the panic."""
    with pytest.raises(ValueError):
        make()


@pytest.mark.parametrize(
    ("make_heuristic", "make_problem", "expected"),
    [
        pytest.param(
            lambda: optopus.PopulationAnnealing(population_size=5, stop=stop(10)),
            lambda: optopus.Qubo.from_entries([(0, 0, -1)]),
            "PopulationAnnealing is only available for MaxCut",
            id="population-annealing-on-qubo",
        ),
        pytest.param(
            lambda: optopus.WalkSat(stop=stop(10)),
            lambda: optopus.MaxCut.from_edges(TRIANGLE),
            "WalkSat is only available for Sat",
            id="walksat-on-maxcut",
        ),
        pytest.param(
            lambda: optopus.LinKernighanHelsgaun(stop=stop(10)),
            lambda: optopus.Sat.from_clauses(2, [[1, 2]]),
            "LinKernighanHelsgaun is only available for TspWithCoordinates",
            id="lkh-on-sat",
        ),
        pytest.param(
            lambda: optopus.VariableNeighborhoodSearch(
                optopus.BreakoutLocalSearch((3, 50), 100, 5, 0.8, 0.5, stop(10)),
                [optopus.RandomWalk("Flip", stop(3))],
                stop(10),
            ),
            lambda: optopus.Qubo.from_entries([(0, 0, -1)]),
            "BreakoutLocalSearch is only available for MaxCut",
            id="nested-step-on-wrong-problem",
        ),
    ],
)
def test_problem_specific_heuristic_rejects_other_problems(make_heuristic, make_problem, expected):
    with pytest.raises(ValueError, match=expected):
        make_heuristic().run(make_problem())


def test_variable_neighborhood_search_rejects_non_heuristic_step():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    vns = optopus.VariableNeighborhoodSearch(
        optopus.LocalSearch("Flip", stop(10)), ["not a heuristic"], stop(10)
    )
    with pytest.raises(TypeError):
        vns.run(mc)

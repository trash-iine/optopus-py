"""Smoke tests exercising the built extension end-to-end.

The instances are tiny so every heuristic reaches the known optimum within the
iteration budget regardless of seed.
"""

import optopus


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

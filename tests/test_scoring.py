"""Scoring a solution and reading an instance back without running a heuristic."""

import math

import pytest

import optopus
from tests.helpers import TRIANGLE, stop

# A 3-4-5 rectangle: sides 3 and 4, diagonals 5.
RECTANGLE = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0), (0.0, 4.0)]


def test_max_cut_cut_size():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    assert mc.cut_size([1, 0, 0]) == 4.0
    assert mc.cut_size([True, True, False]) == 5.0


def test_qubo_energy():
    qubo = optopus.Qubo.from_entries([(0, 0, -1), (1, 1, -1), (0, 1, 3)])
    assert qubo.energy([1, 0]) == -1
    assert qubo.energy([1, 1]) == 1


def test_sat_counts_and_num_satisfied():
    sat = optopus.Sat.from_clauses(2, [[1, 2], [-1], [-2]])
    assert (sat.num_vars(), sat.num_clauses()) == (2, 3)
    assert sat.num_satisfied([0, 0]) == 2
    assert sat.num_satisfied([1, 1]) == 1


def test_vertex_cover_evaluate_cover():
    vc = optopus.VertexCover.from_edges(TRIANGLE)
    penalty = vc.penalty_weight()
    assert vc.evaluate_cover([1, 1, 0]) == {"objective": 2, "cover_size": 2, "uncovered_edges": 0}
    assert vc.evaluate_cover([1, 0, 0]) == {
        "objective": 1 + penalty,
        "cover_size": 1,
        "uncovered_edges": 1,
    }


def test_tsp_tour_length_and_distances():
    tsp = optopus.Tsp.from_coordinates(RECTANGLE)
    assert tsp.tour_length([0, 1, 2, 3]) == 14.0
    assert tsp.tour_length([0, 2, 1, 3]) == 18.0
    assert tsp.distance(0, 2) == 5.0
    assert tsp.coordinates() == RECTANGLE
    assert tsp.edge_weight_type() == "Continuous"
    matrix = optopus.Tsp.from_distance_matrix([[0.0, 1.0], [1.0, 0.0]])
    assert matrix.coordinates() is None
    assert matrix.edge_weight_type() is None


@pytest.mark.parametrize(
    ("kind", "expected"),
    [
        # Points 1.5 and 2.66 apart: rounded, rounded up, or TSPLIB's pseudo-Euclidean formula.
        ("Continuous", (1.5, math.hypot(1.5, 2.2))),
        ("EUC_2D", (2.0, 3.0)),
        ("CEIL_2D", (2.0, 3.0)),
        ("ATT", (1.0, 1.0)),
    ],
)
def test_tsp_edge_weight_types(kind, expected):
    tsp = optopus.Tsp.from_coordinates([(0.0, 0.0), (1.5, 0.0), (1.5, 2.2)], edge_weight_type=kind)
    assert tsp.edge_weight_type() == kind
    assert (tsp.distance(0, 1), tsp.distance(0, 2)) == pytest.approx(expected)


def test_tsp_nearest_neighbors_answers_like_the_full_matrix():
    full = optopus.Tsp.from_coordinates(RECTANGLE)
    sparse = optopus.Tsp.from_coordinates(RECTANGLE, nearest_neighbors=1)
    pairs = [(i, j) for i in range(4) for j in range(4)]
    assert [sparse.distance(i, j) for i, j in pairs] == [full.distance(i, j) for i, j in pairs]
    report = optopus.LocalSearch("TwoOpt", stop(50)).run(sparse, runs=3, seed=1)
    assert report.best_objective == 14.0


def test_job_shop_makespan_and_schedule():
    shop = optopus.JobShopScheduling.from_jobs([[(0, 3), (1, 2)], [(1, 2), (0, 1)]])
    assert (shop.num_jobs(), shop.num_machines()) == (2, 2)
    assert shop.makespan([0, 1, 0, 1]) == 5
    assert shop.completion_times([0, 1, 0, 1]) == [3, 2, 5, 4]


def test_flow_shop_processing_time():
    shop = optopus.FlowShop.from_processing_times([[3, 2, 4], [2, 5, 1]])
    assert [shop.processing_time(m, j) for m in range(2) for j in range(3)] == [3, 2, 4, 2, 5, 1]


def test_bin_packing_first_fit():
    bins = optopus.BinPacking.from_sizes(10, [6, 4, 5, 5, 3])
    assert bins.first_fit() == [0, 0, 1, 1, 2]
    # First Fit Decreasing places the 6, both 5s, the 4 and the 3.
    assert bins.first_fit([0, 2, 3, 1, 4]) == [0, 0, 1, 1, 2]
    assert bins.evaluate_bins(bins.first_fit())["num_bins"] == 3


def test_vrp_split_giant_tour():
    vrp = optopus.Vrp.from_coordinates(
        [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)], [0, 1, 1, 1], capacity=2, num_vehicles=2
    )
    routes = vrp.split_giant_tour([1, 2, 3])
    assert sorted(c for route in routes for c in route) == [1, 2, 3]
    assert len(routes) == 2
    assert vrp.evaluate_routes(routes)["overload"] == 0


def test_vrp_nearest_neighbors_scores_like_the_full_matrix():
    args = ([(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)], [0, 1, 1, 1])
    full = optopus.Vrp.from_coordinates(*args, capacity=2, num_vehicles=2)
    sparse = optopus.Vrp.from_coordinates(*args, capacity=2, num_vehicles=2, nearest_neighbors=1)
    routes = [[1], [2, 3]]
    assert sparse.evaluate_routes(routes) == full.evaluate_routes(routes)


@pytest.mark.parametrize(
    ("call", "match"),
    [
        (lambda: optopus.MaxCut.from_edges(TRIANGLE).cut_size([1, 0]), "does not fit"),
        (lambda: optopus.Qubo.from_entries([(0, 1, 1)]).energy([1, 2]), "entry 1 must be"),
        (lambda: optopus.Tsp.from_coordinates(RECTANGLE).tour_length([0, 0, 1, 2]), "not fit"),
        (lambda: optopus.Tsp.from_coordinates(RECTANGLE).distance(0, 4), "outside"),
        (
            lambda: optopus.Tsp.from_coordinates(RECTANGLE, edge_weight_type="EUC"),
            "edge_weight_type",
        ),
        (lambda: optopus.Tsp.from_coordinates(RECTANGLE, nearest_neighbors=0), "at least 1"),
        (lambda: optopus.Tsp.from_coordinates([(0.0, math.nan)]), "finite"),
        (
            lambda: optopus.JobShopScheduling.from_jobs([[(0, 1)], [(0, 2)]]).makespan([0]),
            "does not fit",
        ),
        (
            lambda: optopus.FlowShop.from_processing_times([[1, 2]]).processing_time(1, 0),
            "outside",
        ),
        (lambda: optopus.BinPacking.from_sizes(10, [1, 2]).first_fit([0, 0]), "twice"),
        (
            lambda: optopus.Vrp.from_coordinates(
                [(0.0, 0.0), (1.0, 0.0)], [0, 1], capacity=1
            ).split_giant_tour([0]),
            "every customer",
        ),
    ],
)
def test_invalid_input_raises_value_error(call, match):
    with pytest.raises(ValueError, match=match):
        call()

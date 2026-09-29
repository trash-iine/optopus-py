"""Smoke tests exercising the built extension end-to-end.

The instances are tiny so every heuristic reaches the known optimum within the
iteration budget regardless of seed.
"""

import inspect

import pytest

import optopus

# The triangle's optimum cut puts one vertex alone, cutting the two heaviest edges.
TRIANGLE = [(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)]
TRIANGLE_OPTIMUM = 5.0

UNIT_SQUARE = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]

# Two customer clusters far apart on either side of the depot. Capacity leaves enough slack
# for a 3/1 split, so Relocate can travel between partitions rather than being walled in by
# the overload penalty. The optimum serves each cluster with its own vehicle.
VRP_CLUSTERS = [(0.0, 0.0), (10.0, 0.0), (11.0, 0.0), (-10.0, 0.0), (-11.0, 0.0)]
VRP_DEMANDS = [0, 5, 5, 5, 5]
VRP_OPTIMUM = 44.0

# One vehicle over three corners of the unit square: a plain TSP, which is what the
# intra-route TwoOpt neighborhood can actually solve on its own.
VRP_SINGLE_ROUTE = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]
VRP_SINGLE_ROUTE_OPTIMUM = 4.0


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


def clustered_vrp():
    return optopus.Vrp.from_coordinates(VRP_CLUSTERS, VRP_DEMANDS, capacity=15, num_vehicles=2)


def single_route_vrp():
    return optopus.Vrp.from_coordinates(VRP_SINGLE_ROUTE, [0, 1, 1, 1], capacity=10, num_vehicles=1)


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
    tsp = optopus.Tsp.from_coordinates(
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
        search=optopus.BreakoutLocalSearch((6, 100), 100, 5, 0.8, 0.5, stop(100)),
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


def test_population_annealing_is_no_longer_maxcut_only():
    # optopus generalized it off MaxCut, so it runs wherever a neighborhood does.
    qubo = optopus.Qubo.from_entries([(0, 0, -1), (1, 1, -1), (0, 1, 2)])
    pa = optopus.PopulationAnnealing(population_size=6, stop=stop(50), sweeps_per_step=5)
    assert pa.run(qubo, runs=2, seed=42).best_objective == -1.0

    # A pairwise move makes counting the neighborhood O(n^2), so pin the sweep length.
    tsp = optopus.Tsp.from_coordinates(UNIT_SQUARE)
    pa = optopus.PopulationAnnealing(
        population_size=10, stop=stop(50), sweeps_per_step=5, neighbor="TwoOpt", sweep_length=4
    )
    assert abs(pa.run(tsp, seed=42).best_objective - 4.0) < 1e-9


def test_breakout_local_search():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bls = optopus.BreakoutLocalSearch((6, 100), 1_000, 20, 0.8, 0.5, stop(1_000))
    assert bls.run(mc, runs=2, seed=42).best_objective == TRIANGLE_OPTIMUM


def test_lin_kernighan_helsgaun():
    tsp = optopus.Tsp.from_coordinates(UNIT_SQUARE, name="unit-square")
    report = optopus.LinKernighanHelsgaun(stop=stop(200)).run(tsp, runs=2, seed=42)
    assert abs(report.best_objective - 4.0) < 1e-9
    assert sorted(report.runs[0].solution) == [0, 1, 2, 3]


# --- VRP -------------------------------------------------------------------

# The objective accumulates incremental move gains in f64, so it lands within rounding
# distance of the recomputed value rather than exactly on it -- compare with a tolerance,
# as the TSP tests do.


@pytest.mark.parametrize("neighbor", ["Relocate", "Swap"])
def test_vrp_local_search(neighbor):
    vrp = clustered_vrp()
    report = optopus.LocalSearch(neighbor, stop(2_000)).run(vrp, runs=4, seed=42)
    assert abs(report.best_objective - VRP_OPTIMUM) < 1e-9
    assert len(report.runs[0].solution) == vrp.num_vehicles()


def test_vrp_tabu_search():
    vrp = clustered_vrp()
    report = optopus.TabuSearch("Swap", (3, 10), stop(2_000)).run(vrp, runs=4, seed=42)
    assert abs(report.best_objective - VRP_OPTIMUM) < 1e-9


def test_vrp_two_opt_reorders_a_single_route():
    vrp = single_route_vrp()
    report = optopus.LocalSearch("TwoOpt", stop(2_000)).run(vrp, runs=4, seed=42)
    assert abs(report.best_objective - VRP_SINGLE_ROUTE_OPTIMUM) < 1e-9


def test_vrp_adaptive_large_neighborhood_search():
    vrp = clustered_vrp()
    alns = optopus.AdaptiveLargeNeighborhoodSearch(stop(2_000))
    assert abs(alns.run(vrp, runs=4, seed=42).best_objective - VRP_OPTIMUM) < 1e-9


def test_vrp_hybrid_genetic_search():
    vrp = clustered_vrp()
    hgs = optopus.HybridGeneticSearch(stop(2_000), min_population_size=8, generation_size=8)
    assert abs(hgs.run(vrp, runs=4, seed=42).best_objective - VRP_OPTIMUM) < 1e-9


def test_vrp_rejects_unknown_neighbor():
    with pytest.raises(ValueError, match="invalid neighbor 'Flip' for Vrp"):
        optopus.LocalSearch("Flip", stop(10)).run(clustered_vrp())


def test_vrp_evaluate_routes_separates_distance_from_penalty():
    vrp = clustered_vrp()

    feasible = vrp.evaluate_routes([[1, 2], [3, 4]])
    assert feasible["overload"] == 0
    assert feasible["objective"] == feasible["distance"] == VRP_OPTIMUM
    assert feasible["route_loads"] == [10, 10]

    overloaded = vrp.evaluate_routes([[1, 2, 3, 4], []])
    assert overloaded["overload"] == 5
    assert overloaded["objective"] == pytest.approx(
        overloaded["distance"] + vrp.penalty_weight() * overloaded["overload"]
    )


def test_vrp_penalty_weight_exceeds_any_route_set():
    vrp = clustered_vrp()
    # 4 customers + 2 vehicles, times the longest edge (11 to -11 is 22), plus 1.
    assert vrp.penalty_weight() == 6 * 22.0 + 1


def test_vrp_evaluate_routes_rejects_an_invalid_partition():
    with pytest.raises(ValueError):
        clustered_vrp().evaluate_routes([[1, 2], [3]])


@pytest.mark.parametrize("routes", [[[1, 2, 3, 4]], [[1, 2], [3], [4]]], ids=["fewer", "more"])
def test_vrp_evaluate_routes_needs_one_route_per_vehicle(routes):
    with pytest.raises(ValueError, match=r"one route per vehicle \(2\)"):
        clustered_vrp().evaluate_routes(routes)


def test_vrp_picks_a_fleet_size_when_asked():
    vrp = optopus.Vrp.from_coordinates(VRP_CLUSTERS, VRP_DEMANDS, capacity=10)
    # 20 units of demand over capacity 10 needs at least 2 vehicles; the margin adds more.
    assert vrp.num_vehicles() >= 2
    assert vrp.num_customers() == 4
    assert vrp.capacity() == 10


def test_vrp_load_file(tmp_path):
    instance = tmp_path / "demo.vrp"
    instance.write_text(
        "NAME : demo\n"
        "TYPE : CVRP\n"
        "DIMENSION : 5\n"
        "EDGE_WEIGHT_TYPE : EUC_2D\n"
        "CAPACITY : 15\n"
        "NODE_COORD_SECTION\n"
        "1 0 0\n"
        "2 10 0\n"
        "3 11 0\n"
        "4 -10 0\n"
        "5 -11 0\n"
        "DEMAND_SECTION\n"
        "1 0\n"
        "2 5\n"
        "3 5\n"
        "4 5\n"
        "5 5\n"
        "DEPOT_SECTION\n"
        "1\n"
        "-1\n"
        "EOF\n"
    )
    vrp = optopus.Vrp.load_file(str(instance))
    assert vrp.num_customers() == 4
    assert vrp.capacity() == 15
    # EUC_2D rounds to the nearest integer, and these coordinates are already integral.
    # num_vehicles=0 in the file's absence picks 2 that pack plus a spare, so 3 routes.
    assert vrp.num_vehicles() == 3
    assert vrp.evaluate_routes([[1, 2], [3, 4], []])["distance"] == VRP_OPTIMUM


def test_vrp_load_file_reports_a_missing_file():
    with pytest.raises(ValueError):
        optopus.Vrp.load_file("no-such-instance.vrp")


# A truck (capacity 4) and two faster vans (capacity 2) over four customers of demand 2, each
# with a service time of 1. Brute force over every partition and order gives the optima: the
# truck serves the two customers on one side, a van each of the others.
FLEET_COORDS = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0), (-3.0, 0.0), (-3.0, -4.0)]
FLEET_DEMANDS = [0, 2, 2, 2, 2]
FLEET_SERVICE = [0.0, 1.0, 1.0, 1.0, 1.0]
# Route times 14 + 4 + 6 = 24 (makespan 14), cost 10 + 2 * 1 + 0.5 * (6 + 10) = 20.
FLEET_OPTIMUM = {"TotalTime": 44.0, "Makespan": 34.0}


def fleet_vrp(objective_mode="TotalTime", truck_min_count=0, van_max_route_time=None):
    truck = optopus.VehicleType(
        "truck", capacity=4, max_count=1, fixed_cost=10.0, min_count=truck_min_count
    )
    van = optopus.VehicleType(
        "van",
        capacity=2,
        max_count=2,
        speed=2.0,
        fixed_cost=1.0,
        variable_cost_per_distance=0.5,
        max_route_time=van_max_route_time,
    )
    return optopus.Vrp.with_fleet(
        FLEET_COORDS,
        FLEET_DEMANDS,
        [truck, van],
        service_times=FLEET_SERVICE,
        objective_mode=objective_mode,
    )


def test_vrp_with_fleet_lays_routes_out_by_vehicle_type():
    vrp = fleet_vrp()
    assert vrp.num_vehicles() == 3
    assert vrp.slot_types() == [0, 1, 1]
    assert [vt.name for vt in vrp.vehicle_types()] == ["truck", "van"]
    assert vrp.vehicle_types()[1].speed == 2.0
    assert vrp.vehicle_types()[1].max_route_time is None
    assert vrp.service_times() == FLEET_SERVICE
    assert vrp.objective_mode() == "TotalTime"
    assert vrp.cost_weight() == 1.0
    # 7 edges of at most 10 over speed 1, plus 4 of service; costs 10 + 2 + 0.5 * 70.
    assert vrp.penalty_weight() == 74.0 + 47.0 + 1
    with pytest.raises(ValueError, match="2 vehicle types"):
        vrp.capacity()


def test_vrp_with_fleet_evaluate_routes_reports_every_term():
    result = fleet_vrp().evaluate_routes([[1, 2], [3], [4]])
    assert result["route_distances"] == [12.0, 6.0, 10.0]
    assert result["route_times"] == [14.0, 4.0, 6.0]
    assert result["used_count"] == [1, 2]
    assert (result["total_time"], result["makespan"]) == (24.0, 14.0)
    assert result["total_cost"] == 20.0
    assert result["objective"] == FLEET_OPTIMUM["TotalTime"]


def test_vrp_with_fleet_penalizes_every_soft_constraint():
    vrp = fleet_vrp(truck_min_count=1, van_max_route_time=5.0)
    # The truck idles (1 short), and each van carries 4 over capacity 2 for 8 time units.
    result = vrp.evaluate_routes([[], [1, 2], [3, 4]])
    assert result["overload"] == 4
    assert result["time_excess"] == 3.0 + 3.0
    assert result["min_count_shortfall"] == 1
    violation = 4 + 6.0 + 1
    expected = result["total_time"] + result["total_cost"] + vrp.penalty_weight() * violation
    assert result["objective"] == pytest.approx(expected)


@pytest.mark.parametrize("mode", ["TotalTime", "Makespan"])
@pytest.mark.parametrize(
    "heuristic",
    [
        optopus.HybridGeneticSearch(stop=stop(200)),
        optopus.AdaptiveLargeNeighborhoodSearch(stop=stop(500)),
    ],
    ids=["hgs", "alns"],
)
def test_vrp_with_fleet_reaches_the_optimum(heuristic, mode):
    report = heuristic.run(fleet_vrp(mode), runs=4, seed=42)
    assert report.best_objective == pytest.approx(FLEET_OPTIMUM[mode])


@pytest.mark.parametrize("neighbor", ["Relocate", "Swap", "TwoOpt"])
def test_vrp_with_fleet_runs_the_generic_moves(neighbor):
    vrp = fleet_vrp()
    report = optopus.LocalSearch(neighbor, stop(2_000)).run(vrp, runs=2, seed=42)
    solution = report.runs[0].solution
    assert len(solution) == vrp.num_vehicles()
    assert vrp.evaluate_routes(solution)["objective"] == pytest.approx(report.best_objective)


def test_vrp_with_objective_mode_returns_a_copy():
    vrp = fleet_vrp()
    makespan = vrp.with_objective_mode("Makespan")
    assert (vrp.objective_mode(), makespan.objective_mode()) == ("TotalTime", "Makespan")
    routes = [[1, 2], [3], [4]]
    assert makespan.evaluate_routes(routes)["objective"] == FLEET_OPTIMUM["Makespan"]
    assert vrp.evaluate_routes(routes)["objective"] == FLEET_OPTIMUM["TotalTime"]


def test_vrp_load_file_reads_a_toml_fleet(tmp_path):
    instance = tmp_path / "fleet.toml"
    customers = "".join(
        f"[[customers]]\nid = {i}\nx = {x}\ny = {y}\ndemand = 2\nservice_time = 1.0\n"
        for i, (x, y) in enumerate(FLEET_COORDS[1:], start=1)
    )
    instance.write_text(
        'objective_mode = "Makespan"\n'
        "[depot]\nx = 0.0\ny = 0.0\n"
        '[[vehicle_types]]\nname = "truck"\ncapacity = 4\nspeed = 1.0\n'
        "fixed_cost = 10.0\nmax_count = 1\n"
        '[[vehicle_types]]\nname = "van"\ncapacity = 2\nspeed = 2.0\nfixed_cost = 1.0\n'
        "variable_cost_per_distance = 0.5\nmax_count = 2\n" + customers
    )
    vrp = optopus.Vrp.load_file(str(instance))
    # The name defaults to the file stem.
    assert repr(vrp) == 'Vrp(name="fleet", num_customers=4, vehicle_types=2, num_vehicles=3)'
    assert vrp.slot_types() == [0, 1, 1]
    assert vrp.objective_mode() == "Makespan"
    assert vrp.evaluate_routes([[1, 2], [3], [4]])["objective"] == FLEET_OPTIMUM["Makespan"]


def test_vrp_load_file_reports_an_invalid_toml_fleet(tmp_path):
    instance = tmp_path / "fleet.toml"
    instance.write_text('objective_mode = "TotalTime"\n[depot]\nx = 0.0\ny = 0.0\n')
    with pytest.raises(ValueError):
        optopus.Vrp.load_file(str(instance))


def test_vehicle_type_reports_its_arguments():
    vt = optopus.VehicleType("van", capacity=2, max_count=3, min_count=1, max_route_time=8.0)
    assert (vt.name, vt.capacity, vt.max_count, vt.min_count) == ("van", 2, 3, 1)
    assert (vt.speed, vt.fixed_cost, vt.variable_cost_per_distance) == (1.0, 0.0, 0.0)
    assert vt.max_route_time == 8.0
    assert optopus.VehicleType("x", 1, 1, max_route_time=float("inf")).max_route_time is None


@pytest.mark.parametrize(
    ("make", "expected"),
    [
        pytest.param(lambda: optopus.VehicleType("v", 0, 1), "'capacity'", id="capacity"),
        pytest.param(lambda: optopus.VehicleType("v", 1, 0), "'max_count'", id="max-count"),
        pytest.param(lambda: optopus.VehicleType("v", 1, 1, speed=0.0), "'speed'", id="speed-zero"),
        pytest.param(
            lambda: optopus.VehicleType("v", 1, 1, min_count=2), "'min_count'", id="min-count"
        ),
        pytest.param(
            lambda: optopus.VehicleType("v", 1, 1, fixed_cost=-1.0),
            "'fixed_cost'",
            id="negative-cost",
        ),
        pytest.param(
            lambda: optopus.VehicleType("v", 1, 1, variable_cost_per_distance=float("nan")),
            "'variable_cost_per_distance'",
            id="nan-cost",
        ),
        pytest.param(
            lambda: optopus.VehicleType("v", 1, 1, max_route_time=0.0),
            "'max_route_time'",
            id="route-time-zero",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet([], [], [optopus.VehicleType("v", 1, 1)]),
            "must not be empty",
            id="fleet-no-depot",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(
                [(0.0, float("inf"))], [0], [optopus.VehicleType("v", 1, 1)]
            ),
            "must be finite",
            id="fleet-infinite-coordinate",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(FLEET_COORDS, FLEET_DEMANDS, []),
            "at least one VehicleType",
            id="fleet-empty",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(
                FLEET_COORDS, FLEET_DEMANDS, [optopus.VehicleType("v", 1, 1)], service_times=[0.0]
            ),
            "'service_times' must have the same length",
            id="fleet-service-length",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(
                FLEET_COORDS,
                FLEET_DEMANDS,
                [optopus.VehicleType("v", 1, 1)],
                service_times=[0.0, -1.0, 0.0, 0.0, 0.0],
            ),
            "service time 1 is -1",
            id="fleet-negative-service",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(
                FLEET_COORDS, FLEET_DEMANDS, [optopus.VehicleType("v", 1, 1)], objective_mode="Sum"
            ),
            "invalid objective mode 'Sum'",
            id="fleet-objective-mode",
        ),
        pytest.param(
            lambda: optopus.Vrp.with_fleet(
                FLEET_COORDS, FLEET_DEMANDS, [optopus.VehicleType("v", 1, 1)], cost_weight=-1.0
            ),
            "'cost_weight'",
            id="fleet-cost-weight",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_coordinates([(0.0, 0.0), (float("nan"), 0.0)], [0, 1], 5),
            "coordinate 1",
            id="cvrp-nan-coordinate",
        ),
    ],
)
def test_vrp_fleet_rejects_invalid_input(make, expected):
    """Upstream panics on each of these, so the binding must reject them first."""
    with pytest.raises(ValueError, match=expected):
        make()


# --- MaxCut kernelization ---------------------------------------------------

# A path has pendant and degree-2 vertices at every step, so the rules reduce it away
# entirely; a torus is 4-regular, so none of them fire.
PATH = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 5, 1.0)]


def cut_weight(edges, assignment):
    return sum(w for u, v, w in edges if assignment[u] != assignment[v])


def test_maxcut_kernel_preserves_the_objective():
    kernel = optopus.MaxCutKernel.reduce(optopus.MaxCut.from_edges(PATH))
    assert not kernel.is_trivial()
    assert kernel.removed_vertices() == 6

    report = optopus.LocalSearch("Flip", stop(500)).run(kernel.kernel(), runs=4, seed=7)
    lifted = kernel.lift(report.runs[0].solution)
    assert len(lifted) == 6
    assert cut_weight(PATH, lifted) == report.runs[0].best_objective + kernel.offset()


def test_maxcut_kernel_project_inverts_lift():
    kernel = optopus.MaxCutKernel.reduce(optopus.MaxCut.from_edges(PATH))
    assignment = [False] * 6
    assert kernel.project(kernel.lift(kernel.project(assignment))) == kernel.project(assignment)


def test_maxcut_kernel_is_trivial_on_a_regular_graph():
    torus = optopus.MaxCut.from_graph(optopus.Graph.grid_torus_2d(4))
    kernel = optopus.MaxCutKernel.reduce(torus)
    assert kernel.is_trivial()
    assert kernel.removed_vertices() == 0
    assert kernel.offset() == 0.0


def test_maxcut_kernel_rejects_a_short_assignment():
    kernel = optopus.MaxCutKernel.reduce(optopus.MaxCut.from_edges(TRIANGLE))
    with pytest.raises(ValueError, match="original vertex count"):
        kernel.project([True])


# --- Planted MaxCut instances -----------------------------------------------


def test_planted_maxcut_tile_planting_2d_is_exact():
    planted = optopus.PlantedMaxCut.tile_planting_2d(6, 0.5, 0.0, 0.5, seed=3)
    planted.verify()
    assert planted.has_exact_optimum()
    assert len(planted.planted()) == 36
    assert planted.problem().__repr__() == "MaxCut(num_vertices=36, num_edges=72)"

    bls = optopus.BreakoutLocalSearch((6, 40), 1_000, 10, 0.8, 0.5, stop(20_000))
    assert bls.run(planted.problem(), runs=3, seed=5).best_objective == planted.optimum()


def test_planted_maxcut_tile_planting_3d_is_exact():
    planted = optopus.PlantedMaxCut.tile_planting_3d(4, 0.5, 0.3, seed=3)
    planted.verify()
    assert len(planted.planted()) == 64


def test_planted_maxcut_wishart_couplers_control_exactness():
    discrete = optopus.PlantedMaxCut.wishart(24, 0.75, couplers="Discrete", seed=1)
    discrete.verify()
    assert discrete.has_exact_optimum()

    gaussian = optopus.PlantedMaxCut.wishart(24, 0.75, seed=1)
    gaussian.verify()
    assert not gaussian.has_exact_optimum()


def test_planted_maxcut_rejects_unknown_couplers():
    with pytest.raises(ValueError, match="invalid couplers 'Uniform'"):
        optopus.PlantedMaxCut.wishart(8, 0.5, couplers="Uniform")


# --- Torus lattices ---------------------------------------------------------


@pytest.mark.parametrize("side", [3, 4, 5])
def test_graph_grid_torus_2d(side):
    g = optopus.Graph.grid_torus_2d(side)
    assert (g.num_vertices(), g.num_edges()) == (side * side, 2 * side * side)


@pytest.mark.parametrize("side", [3, 4])
def test_graph_grid_torus_3d(side):
    g = optopus.Graph.grid_torus_3d(side)
    assert (g.num_vertices(), g.num_edges()) == (side**3, 3 * side**3)


def test_grid_torus_accepts_random_weights():
    g = optopus.Graph.grid_torus_2d(4).with_random_weights((1, 10), seed=1)
    assert all(1.0 <= w <= 10.0 for _, _, w in g.edges())


# --- TSP distances ----------------------------------------------------------

# A 4-city ring: each city is 1 from its neighbors and 2 from the opposite one, so the
# optimal tour costs 4 whichever way it is written.
RING_MATRIX = [
    [0.0, 1.0, 2.0, 1.0],
    [1.0, 0.0, 1.0, 2.0],
    [2.0, 1.0, 0.0, 1.0],
    [1.0, 2.0, 1.0, 0.0],
]


def test_tsp_replaces_tsp_with_coordinates():
    # optopus renamed the problem when it gained non-Euclidean distances, and the binding
    # follows without keeping an alias.
    assert not hasattr(optopus, "TspWithCoordinates")


def test_tsp_from_distance_matrix():
    tsp = optopus.Tsp.from_distance_matrix(RING_MATRIX, name="ring")
    assert tsp.num_cities() == 4
    report = optopus.LocalSearch("TwoOpt", stop(500)).run(tsp, runs=4, seed=1)
    assert abs(report.best_objective - 4.0) < 1e-9


def test_tsp_from_distance_matrix_rejects_a_ragged_matrix():
    with pytest.raises(ValueError):
        optopus.Tsp.from_distance_matrix([[0.0, 1.0], [1.0]])


def test_tsp_adaptive_large_neighborhood_search():
    # ALNS is no longer VRP-only: optopus generalized ruin-and-recreate onto tours.
    tsp = optopus.Tsp.from_coordinates(UNIT_SQUARE, name="unit-square")
    report = optopus.AdaptiveLargeNeighborhoodSearch(stop(500)).run(tsp, runs=3, seed=1)
    assert abs(report.best_objective - 4.0) < 1e-9


# --- Graph Coloring ---------------------------------------------------------

# An odd cycle is the smallest graph needing three colors, and its optimum is exactly 3.
CYCLE5 = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0)]
CYCLE5_OPTIMUM = 3.0


@pytest.mark.parametrize("neighbor", ["Recolor", "Swap"])
def test_graph_coloring_tabu_search(neighbor):
    gc = optopus.GraphColoring.from_edges(CYCLE5)
    report = optopus.TabuSearch(neighbor, (3, 10), stop(3_000)).run(gc, runs=4, seed=1)
    assert report.best_objective == CYCLE5_OPTIMUM
    assert len(report.runs[0].solution) == 5


def test_graph_coloring_derives_and_overrides_the_palette():
    # A 5-cycle is 2-regular, so max_degree + 1 is 3.
    assert optopus.GraphColoring.from_edges(CYCLE5).num_colors() == 3
    assert optopus.GraphColoring.from_edges(CYCLE5, num_colors=5).num_colors() == 5
    assert optopus.GraphColoring.from_graph(optopus.Graph.from_edges(CYCLE5)).num_colors() == 3


def test_graph_coloring_evaluate_colors_counts_conflicts():
    gc = optopus.GraphColoring.from_edges(CYCLE5)

    proper = gc.evaluate_colors([0, 1, 0, 1, 2])
    assert proper["conflicts"] == 0
    assert proper["colors_used"] == 3
    assert proper["objective"] == CYCLE5_OPTIMUM

    # One color everywhere conflicts on all five edges, at penalty_weight each.
    clashing = gc.evaluate_colors([0, 0, 0, 0, 0])
    assert clashing["conflicts"] == 5
    assert clashing["objective"] == 1 + gc.penalty_weight() * 5


def test_graph_coloring_evaluate_colors_validates_its_input():
    gc = optopus.GraphColoring.from_edges(CYCLE5)
    with pytest.raises(ValueError, match="one entry per vertex"):
        gc.evaluate_colors([0, 1])
    with pytest.raises(ValueError, match="outside the palette"):
        gc.evaluate_colors([0, 1, 0, 1, 9])


def test_graph_coloring_rejects_unknown_neighbor():
    with pytest.raises(ValueError, match="invalid neighbor 'Flip' for GraphColoring"):
        optopus.LocalSearch("Flip", stop(10)).run(optopus.GraphColoring.from_edges(CYCLE5))


# --- Formula over integer variables -----------------------------------------

# optopus moved Formula onto its integer layer, so variables carry ranges and a solution is
# a list[int]. Binary variables stay the default, and "Flip" stays accepted for "Change".

# Minimize x0 + x1 - 2*x0*x1, whose minimum is 0 (at [0, 0] and at [1, 1]).
XOR_OBJECTIVE = [([0], 1.0), ([1], 1.0), ([0, 1], -2.0)]


@pytest.mark.parametrize("neighbor", ["Change", "Flip"])
def test_formula_binary_variables(neighbor):
    formula = optopus.Formula(n_vars=2, objective=XOR_OBJECTIVE, direction="Minimize")
    assert formula.n_vars() == 2
    report = optopus.LocalSearch(neighbor, stop(200)).run(formula, runs=4, seed=1)
    # best_objective is direction-corrected, so a minimized cost of 0 reports as 0.
    assert report.best_objective == 0.0
    assert report.runs[0].solution in ([0, 0], [1, 1])


def test_formula_integer_bounds():
    # Minimize x0 + x1 over x0 in [2, 5] and x1 in [3, 7]: the minimum cost is 5.
    formula = optopus.Formula(
        n_vars=2,
        objective=[([0], 1.0), ([1], 1.0)],
        direction="Minimize",
        bounds=[(2, 5), (3, 7)],
    )
    report = optopus.LocalSearch("Change", stop(500)).run(formula, runs=4, seed=1)
    assert report.best_objective == -5.0
    assert report.runs[0].solution == [2, 3]
    assert formula.eval_objective([2, 3]) == 5.0
    assert formula.eval_penalty([2, 3]) == 0.0


def test_formula_honors_a_constraint_penalty():
    # Maximize x0 + x1 subject to x0 + x1 <= 1, penalized heavily enough to bind.
    formula = optopus.Formula(
        n_vars=2,
        objective=[([0], 1.0), ([1], 1.0)],
        direction="Maximize",
        constraints=[([([0], 1.0), ([1], 1.0)], "Le", [([], 1.0)], 10.0)],
    )
    report = optopus.LocalSearch("Flip", stop(200)).run(formula, runs=4, seed=1)
    assert report.best_objective == 1.0
    assert sum(report.runs[0].solution) == 1
    assert formula.eval_penalty([1, 1]) == 10.0


def test_formula_reverse_neighborhood():
    formula = optopus.Formula(
        n_vars=4, objective=[([0], 1.0), ([3], 1.0)], direction="Minimize", bounds=[(0, 3)] * 4
    )
    report = optopus.LocalSearch("Reverse", stop(300)).run(formula, runs=4, seed=1)
    assert len(report.runs[0].solution) == 4


def test_formula_rejects_unknown_neighbor():
    formula = optopus.Formula(n_vars=2, objective=XOR_OBJECTIVE)
    with pytest.raises(ValueError, match="invalid neighbor 'TwoOpt' for Formula"):
        optopus.LocalSearch("TwoOpt", stop(10)).run(formula)


# --- Determinism ------------------------------------------------------------


def test_same_seed_reproduces_report():
    mc = optopus.MaxCut.from_edges(TRIANGLE)
    bls = optopus.BreakoutLocalSearch((6, 100), 1_000, 20, 0.8, 0.5, stop(1_000))
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
            lambda: optopus.PopulationAnnealing(population_size=5, stop=stop(10), sweep_length=0),
            id="sweep-length-zero",
        ),
        pytest.param(
            lambda: optopus.Formula(n_vars=0, objective=[]),
            id="formula-no-variables",
        ),
        pytest.param(
            lambda: optopus.Formula(n_vars=2, objective=[([0], 1.0)], bounds=[(0, 5)]),
            id="formula-bounds-length-mismatch",
        ),
        pytest.param(
            lambda: optopus.Formula(n_vars=2, objective=[([0], 1.0)], bounds=[(5, 0), (0, 5)]),
            id="formula-bound-inverted",
        ),
        pytest.param(
            lambda: optopus.Formula(n_vars=2, objective=[([7], 1.0)]),
            id="formula-variable-out-of-range",
        ),
        pytest.param(
            lambda: optopus.Formula(
                n_vars=2,
                objective=[([0], 1.0)],
                constraints=[([([5], 1.0)], "Le", [([], 1.0)], 1.0)],
            ),
            id="formula-constraint-lhs-out-of-range",
        ),
        pytest.param(
            lambda: optopus.Formula(
                n_vars=2,
                objective=[([0], 1.0)],
                constraints=[([([0], 1.0)], "Ge", [([2], 1.0)], 1.0)],
            ),
            id="formula-constraint-rhs-out-of-range",
        ),
        pytest.param(
            lambda: optopus.Formula(
                n_vars=2,
                objective=[([0], 1.0)],
                constraints=[([([0, 3], 1.0)], "Clamp", (0.0, 1.0), 1.0)],
            ),
            id="formula-clamp-out-of-range",
        ),
        pytest.param(
            lambda: optopus.GraphColoring.from_edges(TRIANGLE, num_colors=0),
            id="graph-coloring-no-colors",
        ),
        pytest.param(lambda: optopus.Sat.from_clauses(2, [[1, 3]]), id="sat-literal-too-large"),
        pytest.param(lambda: optopus.Sat.from_clauses(2, [[-3]]), id="sat-negated-too-large"),
        pytest.param(lambda: optopus.Sat.from_clauses(2, [[0]]), id="sat-literal-zero"),
        pytest.param(lambda: optopus.StopCondition(max_duration_secs=-1.0), id="duration-negative"),
        pytest.param(
            lambda: optopus.StopCondition(max_duration_secs=float("inf")), id="duration-infinite"
        ),
        pytest.param(
            lambda: optopus.Tsp.from_coordinates([]),
            id="tsp-no-cities",
        ),
        pytest.param(
            lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(10), removal_fraction=0.0),
            id="removal-fraction-not-positive",
        ),
        pytest.param(
            lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(10), cooling_rate=1.5),
            id="cooling-rate-out-of-range",
        ),
        pytest.param(
            lambda: optopus.HybridGeneticSearch(stop(10), min_population_size=3),
            id="population-below-floor",
        ),
        pytest.param(
            lambda: optopus.HybridGeneticSearch(stop(10), target_feasible=1.0),
            id="target-feasible-not-exclusive",
        ),
        pytest.param(lambda: optopus.Graph.grid_torus_2d(2), id="torus-side-too-small"),
        pytest.param(
            lambda: optopus.PlantedMaxCut.tile_planting_2d(5, 0.5, 0.0, 0.5),
            id="tile-side-odd",
        ),
        pytest.param(
            lambda: optopus.PlantedMaxCut.tile_planting_2d(6, 0.6, 0.6, 0.0),
            id="tile-probs-sum-past-one",
        ),
        pytest.param(
            lambda: optopus.PlantedMaxCut.wishart(8, 1.0),
            id="wishart-alpha-not-exclusive",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_coordinates([(0.0, 0.0)], [0, 1], capacity=5),
            id="vrp-demands-length-mismatch",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_coordinates([(0.0, 0.0), (1.0, 0.0)], [0, 1], capacity=0),
            id="vrp-capacity-not-positive",
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
    ("make", "expected"),
    [
        pytest.param(
            lambda: optopus.Formula(
                n_vars=1, objective=[([0], 1.0)], constraints=[([([0], 1.0)], "Le", [], -1.0)]
            ),
            "penalty_weight must be finite and non-negative",
            id="formula-negative-weight",
        ),
        pytest.param(
            lambda: optopus.Formula(
                n_vars=1,
                objective=[([0], 1.0)],
                constraints=[([([0], 1.0)], "Clamp", (2.0, 1.0), 1.0)],
            ),
            r"lo \(2\) above hi \(1\)",
            id="formula-clamp-inverted",
        ),
        pytest.param(
            lambda: optopus.Formula(
                n_vars=1, objective=[([0], 1.0)], constraints=[([([0], 1.0)], "!=", [], 1.0)]
            ),
            "'Clamp'",
            id="formula-unknown-relation-lists-clamp",
        ),
        pytest.param(
            lambda: optopus.Tsp.from_distance_matrix([[0.0, 1.0], [2.0, 0.0]]),
            "must be symmetric",
            id="tsp-asymmetric-matrix",
        ),
        pytest.param(
            lambda: optopus.Tsp.from_distance_matrix([[0.0, -1.0], [-1.0, 0.0]]),
            "non-negative",
            id="tsp-negative-distance",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_distance_matrix([[0.0, 1.0], [3.0, 0.0]], [0, 1], 5),
            "must be symmetric",
            id="vrp-asymmetric-matrix",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_distance_matrix(
                [[0.0, float("nan")], [float("nan"), 0.0]], [0, 1], 5
            ),
            "finite",
            id="vrp-nan-distance",
        ),
        pytest.param(
            lambda: optopus.Vrp.from_coordinates([(0.0, 0.0), (1.0, 0.0)], [0, -1], 5),
            "demand 1 is negative",
            id="vrp-negative-demand",
        ),
    ],
)
def test_inputs_the_search_would_misprice_raise_value_error(make, expected):
    """The search would run on these, but price moves or penalties wrongly."""
    with pytest.raises(ValueError, match=expected):
        make()


def test_a_matrix_symmetric_up_to_rounding_is_accepted():
    third = 1.0 / 3.0
    tsp = optopus.Tsp.from_distance_matrix([[0.0, third], [third * (1 + 1e-12), 0.0]])
    assert tsp.num_cities() == 2


@pytest.mark.parametrize("rel", ["<", "<=", "==", "=", ">=", ">"])
def test_formula_accepts_relation_symbols(rel):
    constraint = ([([0], 1.0)], rel, [], 1.0)
    optopus.Formula(n_vars=1, objective=[([0], 1.0)], constraints=[constraint])


def test_stop_condition_repr_reads_as_python():
    assert repr(optopus.StopCondition(max_iteration=5, max_duration_secs=1.0)) == (
        "StopCondition(max_iteration=5, max_duration_secs=1.0, max_failed_update=None)"
    )


@pytest.mark.parametrize(
    ("make_heuristic", "make_problem", "expected"),
    [
        pytest.param(
            lambda: optopus.WalkSat(stop=stop(10)),
            lambda: optopus.MaxCut.from_edges(TRIANGLE),
            "WalkSat is only available for Sat",
            id="walksat-on-maxcut",
        ),
        pytest.param(
            lambda: optopus.LinKernighanHelsgaun(stop=stop(10)),
            lambda: optopus.Sat.from_clauses(2, [[1, 2]]),
            "LinKernighanHelsgaun is only available for Tsp",
            id="lkh-on-sat",
        ),
        pytest.param(
            lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(10)),
            lambda: optopus.MaxCut.from_edges(TRIANGLE),
            "AdaptiveLargeNeighborhoodSearch is only available for Vrp, Tsp and Python",
            id="alns-on-maxcut",
        ),
        pytest.param(
            lambda: optopus.HybridGeneticSearch(stop(10)),
            lambda: optopus.MaxCut.from_edges(TRIANGLE),
            "HybridGeneticSearch is only available for Vrp",
            id="hgs-on-maxcut",
        ),
        pytest.param(
            lambda: optopus.VariableNeighborhoodSearch(
                optopus.BreakoutLocalSearch((6, 100), 100, 5, 0.8, 0.5, stop(10)),
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


def _public_callables():
    for name in dir(optopus):
        cls = getattr(optopus, name)
        if isinstance(cls, type):
            yield name, cls
            for member in dir(cls):
                if not member.startswith("_"):
                    yield f"{name}.{member}", getattr(cls, member)


PUBLIC_CALLABLES = list(_public_callables())


@pytest.mark.parametrize(
    ("name", "obj"), PUBLIC_CALLABLES, ids=[name for name, _ in PUBLIC_CALLABLES]
)
def test_signatures_show_real_defaults(name, obj):
    # PyO3 renders a default it cannot print as a literal, such as `String::new()`, as `...`.
    try:
        signature = str(inspect.signature(obj))
    except (TypeError, ValueError):
        pytest.skip("not introspectable")
    assert "Ellipsis" not in signature, f"{name}{signature}"


def test_iterations_count_every_step_of_a_plain_heuristic():
    report = optopus.RandomWalk("Flip", stop(50)).run(
        optopus.MaxCut.from_edges(TRIANGLE), runs=2, seed=0
    )
    for run in report.runs:
        assert run.iterations == 50 == run.n_accepted + run.n_rejected
    assert report.avg_iterations == 50


def test_iterations_include_the_hgs_initial_population():
    # The initial population is one unit of work: a limit of 1 still builds all 4 * 4 of it.
    report = optopus.HybridGeneticSearch(stop(1), min_population_size=4).run(
        clustered_vrp(), seed=0
    )
    assert report.runs[0].iterations == 16


def test_iterations_of_a_composed_heuristic_are_its_steps():
    # The outer limit is checked when the first step returns, after its own 10 iterations.
    iterated = optopus.Iterated(
        optopus.RandomWalk("Flip", stop(10)), optopus.RandomWalk("Flip", stop(3)), stop(1)
    )
    report = iterated.run(optopus.MaxCut.from_edges(TRIANGLE), seed=0)
    assert report.runs[0].iterations == 10

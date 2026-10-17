"""The tuning parameters of ALNS, iterated greedy, HGS and the eigenvalue bound."""

import pytest

import optopus
from tests.helpers import TRIANGLE, TRIANGLE_OPTIMUM, UNIT_SQUARE, stop

SQUARE_TOUR = 4.0
BIN_SIZES = [6, 4, 5, 5, 3, 7, 2, 8]  # three full bins of 10


def tsp():
    return optopus.Tsp.from_coordinates(UNIT_SQUARE)


def vrp():
    # One vehicle over three corners of the unit square, a tour of length 4.
    return optopus.Vrp.from_coordinates(UNIT_SQUARE, [0, 1, 1, 1], capacity=10, num_vehicles=1)


def bins():
    return optopus.BinPacking.from_sizes(10, BIN_SIZES)


@pytest.mark.parametrize(
    "alns",
    [
        optopus.AdaptiveLargeNeighborhoodSearch(stop(200), removal_fraction=0.5),
        optopus.AdaptiveLargeNeighborhoodSearch(
            stop(200), removal_fraction=0.5, local_repair=False
        ),
        optopus.AdaptiveLargeNeighborhoodSearch(
            stop(200),
            removal_fraction=0.5,
            scores=(1.0, 1.0, 0.0),
            segment_length=5,
            reaction=0.5,
            max_removal=2,
        ),
    ],
)
@pytest.mark.parametrize(("make", "optimum"), [(tsp, SQUARE_TOUR), (vrp, 4.0), (bins, 3.0)])
def test_alns_settings_reach_the_optimum(alns, make, optimum):
    assert alns.run(make(), runs=2, seed=1).best_objective == pytest.approx(optimum, abs=0.1)


@pytest.mark.parametrize("local_repair", [True, False])
@pytest.mark.parametrize(
    ("make", "temperature", "optimum"),
    [(tsp, 0.5, SQUARE_TOUR), (vrp, 0.5, 4.0), (bins, 0.1, 3.0)],
)
def test_iterated_greedy_runs_beyond_the_flow_shop(make, temperature, optimum, local_repair):
    greedy = optopus.IteratedGreedy(
        stop(200), removal_count=2, temperature=temperature, local_repair=local_repair
    )
    assert greedy.run(make(), runs=2, seed=1).best_objective == pytest.approx(optimum, abs=0.1)


def test_iterated_greedy_takes_a_fixed_temperature_on_the_flow_shop():
    shop = optopus.FlowShop.from_processing_times([[3, 2, 4, 1], [2, 5, 1, 3], [4, 1, 2, 2]])
    a = optopus.IteratedGreedy(stop(50), temperature=1.0).run(shop, seed=1)
    b = optopus.IteratedGreedy(stop(50)).run(shop, seed=1)
    assert a.best_objective == b.best_objective


def test_iterated_greedy_needs_a_temperature_off_the_flow_shop():
    with pytest.raises(ValueError, match="IteratedGreedy on Tsp needs a 'temperature'"):
        optopus.IteratedGreedy(stop(10)).run(tsp())


def test_hgs_settings_run():
    hgs = optopus.HybridGeneticSearch(
        stop(100),
        min_population_size=4,
        generation_size=4,
        n_elite=2,
        n_closest=2,
        penalty_adaptation=(10, 1.5, 0.5, 0.1),
        repair_probability=1.0,
        initial_population=(2, 2),
        descent_passes=4,
    )
    assert hgs.run(vrp(), seed=1).best_objective == pytest.approx(4.0)


def test_eigenvalue_relaxation_proves_the_optimum():
    relaxation = optopus.EigenvalueRelaxation(max_iterations=20, lanczos_steps=10)
    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(50)), relaxation=relaxation)
    run = bnb.run(optopus.MaxCut.from_edges(TRIANGLE)).runs[0]
    assert run.best_objective == TRIANGLE_OPTIMUM
    assert run.proven_optimal


def test_eigenvalue_relaxation_defaults_are_optopus_own():
    # The class signature spells the defaults out; this keeps it in step with optopus.
    assert repr(optopus.EigenvalueRelaxation()) == (
        "EigenvalueRelaxation(max_iterations=100, lanczos_steps=30, certify_limit=3000)"
    )


def test_eigenvalue_relaxation_is_for_max_cut_only():
    bnb = optopus.BranchAndBound(
        optopus.LocalSearch("Flip", stop(10)), relaxation=optopus.EigenvalueRelaxation()
    )
    with pytest.raises(ValueError, match="'Eigenvalue' is only available for MaxCut"):
        bnb.run(optopus.Qubo.from_entries([(0, 1, 1)]))


def test_stop_condition_reads_back():
    cond = optopus.StopCondition(max_iteration=5, max_duration_secs=1.5)
    assert (cond.max_iteration, cond.max_duration_secs, cond.max_failed_update) == (5, 1.5, None)


@pytest.mark.parametrize(
    "build",
    [
        lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(1), scores=(1.0, -1.0, 0.0)),
        lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(1), scores=(float("nan"), 1, 1)),
        lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(1), segment_length=0),
        lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(1), reaction=1.5),
        lambda: optopus.AdaptiveLargeNeighborhoodSearch(stop(1), max_removal=0),
        lambda: optopus.IteratedGreedy(stop(1), temperature=0.0),
        lambda: optopus.IteratedGreedy(stop(1), temperature=float("inf")),
        lambda: optopus.HybridGeneticSearch(stop(1), n_elite=0),
        lambda: optopus.HybridGeneticSearch(stop(1), n_closest=0),
        lambda: optopus.HybridGeneticSearch(stop(1), penalty_adaptation=(0, 1.2, 0.85, 0.05)),
        lambda: optopus.HybridGeneticSearch(stop(1), penalty_adaptation=(10, 1.0, 0.85, 0.05)),
        lambda: optopus.HybridGeneticSearch(stop(1), penalty_adaptation=(10, 1.2, 1.0, 0.05)),
        lambda: optopus.HybridGeneticSearch(stop(1), penalty_adaptation=(10, 1.2, 0.85, 0.5)),
        lambda: optopus.HybridGeneticSearch(stop(1), repair_probability=1.5),
        lambda: optopus.HybridGeneticSearch(stop(1), initial_population=(0, 4)),
        lambda: optopus.HybridGeneticSearch(stop(1), initial_population=(4, 0)),
        lambda: optopus.HybridGeneticSearch(stop(1), descent_passes=0),
        lambda: optopus.EigenvalueRelaxation(max_iterations=0),
        lambda: optopus.EigenvalueRelaxation(lanczos_steps=0),
    ],
)
def test_out_of_range_settings_raise_value_error(build):
    with pytest.raises(ValueError):
        build()

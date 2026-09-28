"""Composed heuristics: Sequential, Iterated, Restart, GeneticAlgorithm,
ReinforcementLearningSearch and BreakoutLocalSearch.from_parts, on the built-in problems."""

import pytest

import optopus

TRIANGLE = [(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)]
TRIANGLE_OPTIMUM = 5.0

# Minimize -x0 - x1 + 2 x0 x1: the optimum sets exactly one bit.
QUBO = [(0, 0, -1), (0, 1, 2), (1, 1, -1)]

UNIT_SQUARE = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


def triangle():
    return optopus.MaxCut.from_edges(TRIANGLE)


COMPOSED = {
    "Sequential": lambda: optopus.Sequential(
        [optopus.RandomWalk("Flip", stop(3)), optopus.LocalSearch("Flip", stop(20))], stop(10)
    ),
    "Iterated": lambda: optopus.Iterated(
        optopus.LocalSearch("Flip", stop(20)), optopus.RandomWalk("Flip", stop(2)), stop(10)
    ),
    "Restart": lambda: optopus.Restart(
        optopus.LocalSearch("Flip", stop(20)), restart=stop(5), stop=stop(50)
    ),
    "GeneticAlgorithm": lambda: optopus.GeneticAlgorithm(
        population_size=6, mutation=optopus.LocalSearch("Flip", stop(20)), stop=stop(20)
    ),
    "ReinforcementLearningSearch": lambda: optopus.ReinforcementLearningSearch("Flip", stop(200)),
    "BreakoutLocalSearch.from_parts": lambda: optopus.BreakoutLocalSearch.from_parts(
        descent=optopus.LocalSearch("Flip", stop(100)),
        random=optopus.RandomWalk("Flip", stop(1)),
        directed=[(optopus.TabuSearch("Flip", (1, 2), stop(1)), 1.0)],
        tabu_tenure=(1, 2),
        t=10,
        l0=1,
        p0=0.5,
        stop=stop(50),
    ),
}


@pytest.mark.parametrize("make", COMPOSED.values(), ids=COMPOSED.keys())
def test_composed_heuristics_solve_the_triangle(make):
    assert make().run(triangle(), runs=2, seed=42).best_objective == TRIANGLE_OPTIMUM


@pytest.mark.parametrize("make", COMPOSED.values(), ids=COMPOSED.keys())
def test_composed_heuristics_run_on_qubo(make):
    assert make().run(optopus.Qubo.from_entries(QUBO), seed=1).best_objective == -1.0


def test_steps_nest():
    ils = optopus.Iterated(
        optopus.Sequential(
            [optopus.LocalSearch("Flip", stop(20)), optopus.TabuSearch("Flip", (1, 2), stop(5))],
            stop(1),
        ),
        optopus.RandomWalk("Flip", stop(2)),
        stop(10),
    )
    assert ils.run(triangle(), seed=3).best_objective == TRIANGLE_OPTIMUM


@pytest.mark.parametrize(
    ("problem", "crossover", "optimum"),
    [
        pytest.param(lambda: optopus.Qubo.from_entries(QUBO), "Uniform", -1.0, id="qubo-uniform"),
        pytest.param(
            lambda: optopus.Tsp.from_coordinates(UNIT_SQUARE),
            "Order",
            4.0,
            id="tsp-order",
        ),
    ],
)
def test_ga_named_crossovers(problem, crossover, optimum):
    neighbor = "TwoOpt" if crossover == "Order" else "Flip"
    ga = optopus.GeneticAlgorithm(
        population_size=6,
        mutation=optopus.LocalSearch(neighbor, stop(20)),
        stop=stop(20),
        crossover=crossover,
    )
    assert ga.run(problem(), seed=5).best_objective == pytest.approx(optimum)


def test_ga_sub_problem_crossover_takes_any_sub_heuristic():
    ga = optopus.GeneticAlgorithm(
        population_size=6,
        mutation=optopus.RandomWalk("Flip", stop(1)),
        stop=stop(30),
        crossover="SubProblem",
        sub_heuristic=optopus.LocalSearch("Flip", stop(50)),
        parent_selection="BiasedFitness",
        n_elite=2,
        n_closest=2,
    )
    assert ga.run(triangle(), seed=2).best_objective == TRIANGLE_OPTIMUM


def test_ga_distant_top_k_selection():
    ga = optopus.GeneticAlgorithm(
        population_size=6,
        mutation=optopus.LocalSearch("Flip", stop(20)),
        stop=stop(20),
        parent_selection="DistantTopK",
        parent_top_k=3,
    )
    assert ga.run(triangle(), seed=4).best_objective == TRIANGLE_OPTIMUM


@pytest.mark.parametrize(
    ("make", "problem", "message"),
    [
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6, optopus.LocalSearch("Flip", stop(5)), stop(5), crossover="Order"
            ),
            triangle,
            "invalid crossover 'Order' for MaxCut \\(use 'Uniform' or 'SubProblem'\\)",
            id="unknown-crossover",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6, optopus.LocalSearch("Flip", stop(5)), stop(5), crossover="SubProblem"
            ),
            triangle,
            "needs a 'sub_heuristic'",
            id="sub-problem-without-sub-heuristic",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6,
                optopus.LocalSearch("Flip", stop(5)),
                stop(5),
                sub_heuristic=optopus.LocalSearch("Flip", stop(5)),
            ),
            triangle,
            "only applies to crossover 'SubProblem'",
            id="sub-heuristic-without-sub-problem",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6, optopus.LocalSearch("Relocate", stop(5)), stop(5), crossover="SubProblem"
            ),
            lambda: optopus.Vrp.from_coordinates(
                [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)], [0, 1, 1], capacity=5, num_vehicles=1
            ),
            "invalid crossover 'SubProblem' for Vrp \\(use 'Order'\\)",
            id="no-sub-problem-on-vrp",
        ),
        pytest.param(
            lambda: optopus.Iterated(
                optopus.LocalSearch("Flip", stop(5)), optopus.WalkSat(stop(5)), stop(5)
            ),
            triangle,
            "WalkSat is only available for Sat",
            id="nested-specific-step",
        ),
    ],
)
def test_composed_misuse_raises_at_run(make, problem, message):
    with pytest.raises(ValueError, match=message):
        make().run(problem())


@pytest.mark.parametrize(
    "make",
    [
        pytest.param(
            lambda: optopus.Sequential([], stop(5)),
            id="empty-sequential",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(1, optopus.LocalSearch("Flip", stop(5)), stop(5)),
            id="population-too-small",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6, optopus.LocalSearch("Flip", stop(5)), stop(5), parent_selection="DistantTopK"
            ),
            id="distant-top-k-without-k",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                4, optopus.LocalSearch("Flip", stop(5)), stop(5), parent_selection="BiasedFitness"
            ),
            id="n-elite-not-below-population",
        ),
        pytest.param(
            lambda: optopus.GeneticAlgorithm(
                6, optopus.LocalSearch("Flip", stop(5)), stop(5), parent_selection="Roulette"
            ),
            id="unknown-parent-selection",
        ),
        pytest.param(
            lambda: optopus.ReinforcementLearningSearch("Flip", stop(5), policy_weights=[0.0]),
            id="policy-weights-wrong-length",
        ),
        pytest.param(
            lambda: optopus.ReinforcementLearningSearch("Flip", stop(5), reward_shaping="Clipped"),
            id="unknown-reward-shaping",
        ),
        pytest.param(
            lambda: optopus.ReinforcementLearningSearch("Flip", stop(5), learning_rate=-1.0),
            id="negative-learning-rate",
        ),
        pytest.param(
            lambda: optopus.BreakoutLocalSearch.from_parts(
                optopus.LocalSearch("Flip", stop(5)),
                optopus.RandomWalk("Flip", stop(1)),
                [],
                (1, 2),
                10,
                1,
                0.5,
                stop(5),
            ),
            id="bls-without-directed",
        ),
        pytest.param(
            lambda: optopus.BreakoutLocalSearch((5, 2), 10, 1, 0.5, 0.5, stop(5)),
            id="bls-empty-tenure-range",
        ),
    ],
)
def test_invalid_composed_parameters_raise_value_error(make):
    with pytest.raises(ValueError):
        make()

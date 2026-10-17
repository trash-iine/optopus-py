"""Formula over permutation variables."""

import itertools

import pytest

import optopus
from tests.helpers import stop

# Items linked in a chain 0 - 2 - 1 - 3: the best row puts each next to its partners.
LINKS = {(0, 2): 1.0, (2, 1): 1.0, (1, 3): 1.0}


def cost(pos):
    return sum(w * (pos[i] - pos[j]) ** 2 for (i, j), w in LINKS.items())


OPTIMUM = min(cost(p) for p in itertools.permutations(range(4)))


def arrangement():
    objective = []
    for (i, j), w in LINKS.items():
        objective += [([i, i], w), ([j, j], w), ([i, j], -2.0 * w)]
    return optopus.Formula(n_vars=4, objective=objective, direction="Minimize", permutation=True)


def test_a_permutation_formula_reports_itself():
    row = arrangement()
    assert row.is_permutation()
    assert repr(row) == 'Formula(n_vars=4, direction="Minimize", permutation=true)'
    assert not optopus.Formula(n_vars=2, objective=[]).is_permutation()


@pytest.mark.parametrize(
    "heuristic",
    [
        optopus.LocalSearch("Swap", stop(100)),
        optopus.SimulatedAnnealing("Reverse", 1.0, 0.99, stop(500)),
        optopus.GeneticAlgorithm(6, optopus.LocalSearch("Swap", stop(20)), stop(20)),
    ],
    ids=["LocalSearch", "SimulatedAnnealing", "GeneticAlgorithm"],
)
def test_searches_keep_a_permutation_and_reach_the_optimum(heuristic):
    report = heuristic.run(arrangement(), runs=3, seed=1)
    for run in report.runs:
        assert sorted(run.solution) == [0, 1, 2, 3]
    best = max(report.runs, key=lambda r: r.best_objective)
    assert arrangement().eval_objective(best.solution) == OPTIMUM


def test_a_permutation_starts_from_a_given_order():
    run = optopus.LocalSearch("Swap", stop(0)).run(arrangement(), initial_solution=[3, 1, 2, 0])
    assert run.runs[0].solution == [3, 1, 2, 0]


@pytest.mark.parametrize(
    ("call", "match"),
    [
        (
            lambda: optopus.Formula(n_vars=2, objective=[], bounds=[(0, 1)] * 2, permutation=True),
            "cannot both be given",
        ),
        (lambda: optopus.LocalSearch("Change", stop(5)).run(arrangement()), "'Change'"),
        (lambda: optopus.LocalSearch("Flip", stop(5)).run(arrangement()), "'Flip'"),
        (
            lambda: optopus.BranchAndBound(optopus.LocalSearch("Swap", stop(5))).run(arrangement()),
            "permutation does not allow",
        ),
        (
            lambda: optopus.GeneticAlgorithm(
                4, optopus.LocalSearch("Swap", stop(5)), stop(3), crossover="SubProblem"
            ).run(arrangement()),
            "'SubProblem' crossover does not apply",
        ),
        (
            lambda: optopus.LocalSearch("Swap", stop(5)).run(
                arrangement(), initial_solution=[0, 0, 1, 2]
            ),
            "appears twice",
        ),
    ],
)
def test_what_a_permutation_rules_out_raises_value_error(call, match):
    with pytest.raises(ValueError, match=match):
        call()

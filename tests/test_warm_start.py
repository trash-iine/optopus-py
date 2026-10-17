"""Runs that start from a given solution through ``run(..., initial_solution=...)``."""

import pytest

import optopus
from tests.helpers import TRIANGLE, UNIT_SQUARE, stop

JOBS = [
    [(0, 3), (1, 2), (2, 2)],
    [(0, 2), (2, 1), (1, 4)],
    [(1, 4), (2, 3), (0, 1)],
]

# Each built-in problem with a neighborhood it has, and a solution of it in the form
# `RunResult.solution` takes.
PROBLEMS = {
    "MaxCut": (lambda: optopus.MaxCut.from_edges(TRIANGLE), "Flip", [True, False, False]),
    "Qubo": (
        lambda: optopus.Qubo.from_entries([(0, 0, -1), (0, 1, 2), (1, 1, -1)]),
        "Flip",
        [True, True],
    ),
    "Sat": (lambda: optopus.Sat.from_clauses(2, [[1, 2], [-1], [-2]]), "Flip", [True, True]),
    "VertexCover": (lambda: optopus.VertexCover.from_edges(TRIANGLE), "Flip", [1, 1, 1]),
    "Tsp": (lambda: optopus.Tsp.from_coordinates(UNIT_SQUARE), "TwoOpt", [0, 2, 1, 3]),
    "JobShopScheduling": (
        lambda: optopus.JobShopScheduling.from_jobs(JOBS),
        "Swap",
        [2, 2, 2, 1, 1, 1, 0, 0, 0],
    ),
    "Vrp": (
        lambda: optopus.Vrp.from_coordinates(UNIT_SQUARE, [0, 1, 1, 1], capacity=2, num_vehicles=2),
        "Relocate",
        [[3, 1], [2]],
    ),
    "GraphColoring": (
        lambda: optopus.GraphColoring.from_edges(TRIANGLE),
        "Recolor",
        [0, 0, 1],
    ),
    "FlowShop": (
        lambda: optopus.FlowShop.from_processing_times([[3, 2, 4], [2, 5, 1]]),
        "Swap",
        [2, 0, 1],
    ),
    "Qap": (
        lambda: optopus.Qap.from_matrices(
            [[0, 3, 1], [3, 0, 2], [1, 2, 0]], [[0, 5, 2], [5, 0, 4], [2, 4, 0]]
        ),
        "Swap",
        [2, 1, 0],
    ),
    "BinPacking": (
        lambda: optopus.BinPacking.from_sizes(10, [6, 4, 5, 5]),
        "Relocate",
        [0, 1, 2, 3],
    ),
    "Formula": (
        lambda: optopus.Formula(n_vars=3, objective=[([0, 1], 2.0), ([2], -1.0)]),
        "Change",
        [1, 0, 1],
    ),
}


@pytest.mark.parametrize("name", PROBLEMS)
def test_a_run_starts_from_the_given_solution(name):
    make, neighbor, given = PROBLEMS[name]
    problem = make()
    # A run that stops before its first step reports the solution it started from.
    report = optopus.LocalSearch(neighbor, stop(0)).run(problem, initial_solution=given)
    (run,) = report.runs
    assert run.iterations == 0
    # 0/1 assignments come back as bools, which compare equal to the ints given.
    assert run.solution == given
    assert run.best_objective == run.initial_objective


@pytest.mark.parametrize("name", PROBLEMS)
def test_the_best_solution_of_one_run_continues_in_the_next(name):
    make, neighbor, _ = PROBLEMS[name]
    problem = make()
    first = optopus.RandomWalk(neighbor, stop(20)).run(problem, seed=1).runs[0]
    report = optopus.LocalSearch(neighbor, stop(50)).run(
        problem, runs=3, seed=2, initial_solution=first.solution
    )
    for run in report.runs:
        assert run.initial_objective == pytest.approx(first.best_objective)
        assert run.improvement >= 0


def test_seeded_runs_from_a_given_solution_repeat():
    problem = optopus.Tsp.from_coordinates(UNIT_SQUARE)
    sa = optopus.SimulatedAnnealing("TwoOpt", 1.0, 0.9, stop(30))
    a = sa.run(problem, runs=2, seed=5, initial_solution=[0, 2, 1, 3])
    b = sa.run(problem, runs=2, seed=5, initial_solution=[0, 2, 1, 3])
    assert [r.solution for r in a.runs] == [r.solution for r in b.runs]


@pytest.mark.parametrize(
    ("name", "given", "error", "match"),
    [
        ("MaxCut", [True, False], ValueError, "does not fit"),
        ("MaxCut", [0, 2, 1], ValueError, "entry 1 must be 0, 1"),
        ("MaxCut", 5, TypeError, "sequence of 0/1"),
        ("Tsp", [0, 0, 1, 2], ValueError, "does not fit"),
        ("JobShopScheduling", [0, 1, 2], ValueError, "does not fit"),
        ("Vrp", [[1, 2, 3]], ValueError, "does not fit"),
        ("GraphColoring", [0, 1], ValueError, "does not fit"),
        ("FlowShop", [0, 0, 1], ValueError, "does not fit"),
        ("Qap", [0, 0, 1], ValueError, "does not fit"),
        ("BinPacking", [0, 0, 0, 0], ValueError, "does not fit"),
        ("Formula", [1, 0], ValueError, "does not fit"),
        ("Formula", [1, 0, 2], ValueError, "does not fit"),
    ],
)
def test_a_solution_that_does_not_fit_is_rejected(name, given, error, match):
    make, neighbor, _ = PROBLEMS[name]
    with pytest.raises(error, match=match):
        optopus.LocalSearch(neighbor, stop(1)).run(make(), initial_solution=given)


class Bits:
    """Maximize the number of set bits, drawing its random moves from the run's rng."""

    minimize = False

    class Flip:
        def neighbors(self, problem, x):
            return range(len(x))

        def apply(self, problem, x, i):
            return (*x[:i], not x[i], *x[i + 1 :])

        def random_neighbor(self, problem, x, rng):
            return rng.randrange(len(x))

    def __init__(self, n):
        self.n = n
        self.neighborhoods = {"Flip": Bits.Flip()}
        self.drawn = 0

    def new_solution(self, rng):
        self.drawn += 1
        return tuple(rng.random() < 0.5 for _ in range(self.n))

    def objective(self, x):
        return float(sum(x))


def test_a_python_problem_starts_from_the_given_object():
    problem = Bits(6)
    given = (False,) * 6
    report = optopus.LocalSearch("Flip", stop(0)).run(problem, runs=2, initial_solution=given)
    assert all(run.solution is given for run in report.runs)
    assert all(run.initial_objective == 0.0 for run in report.runs)
    assert problem.drawn == 0


def test_a_python_problem_from_a_given_solution_repeats_with_a_seed():
    # The moves come from `random_neighbor`, which draws from the rng handed to the callbacks,
    # so this repeats only if that rng is seeded from the run although `new_solution` is not
    # called.
    sa = optopus.SimulatedAnnealing("Flip", 1.0, 0.95, stop(40))
    given = (False,) * 8
    a = sa.run(Bits(8), runs=2, seed=3, initial_solution=given)
    b = sa.run(Bits(8), runs=2, seed=3, initial_solution=given)
    assert [r.solution for r in a.runs] == [r.solution for r in b.runs]
    assert all(r.initial_objective == 0.0 for r in a.runs)

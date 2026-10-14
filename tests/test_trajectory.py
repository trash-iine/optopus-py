"""RunResult.trajectory: the improvements of the best solution over a run."""

import math

import pytest

import optopus
from tests.helpers import stop

N = 12


class OneMax:
    """Maximize the number of set bits, with a flip neighborhood."""

    minimize = False

    class Flip:
        def neighbors(self, problem, x):
            return range(len(x))

        def apply(self, problem, x, i):
            return (*x[:i], not x[i], *x[i + 1 :])

    def __init__(self, n):
        self.n = n
        self.neighborhoods = {"Flip": OneMax.Flip()}

    def new_solution(self, rng):
        return tuple(rng.random() < 0.5 for _ in range(self.n))

    def objective(self, x):
        return float(sum(x))


def circle(n):
    return [(math.cos(2 * math.pi * i / n), math.sin(2 * math.pi * i / n)) for i in range(n)]


def max_cut():
    return optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))


# Each case: the problem, the heuristic, and whether the report's scale is higher-is-better.
# A Formula reports higher-is-better whichever way it optimizes.
CASES = {
    "MaxCut": (max_cut, lambda: optopus.LocalSearch("Flip", stop(10_000)), True),
    "Qubo": (
        lambda: optopus.Qubo.from_entries([(i, i, -1) for i in range(N)]),
        lambda: optopus.LocalSearch("Flip", stop(10_000)),
        False,
    ),
    "Tsp": (
        lambda: optopus.Tsp.from_coordinates(circle(10)),
        lambda: optopus.LocalSearch("TwoOpt", stop(10_000)),
        False,
    ),
    "Formula": (
        lambda: optopus.Formula(
            n_vars=N, objective=[([i], 1.0) for i in range(N)], direction="Minimize"
        ),
        lambda: optopus.LocalSearch("Flip", stop(10_000)),
        True,
    ),
    "Python": (lambda: OneMax(N), lambda: optopus.LocalSearch("Flip", stop(10_000)), True),
    "Iterated": (
        max_cut,
        lambda: optopus.Iterated(
            optopus.LocalSearch("Flip", stop(100)), optopus.RandomWalk("Flip", stop(5)), stop(50)
        ),
        True,
    ),
    "Restart": (
        max_cut,
        lambda: optopus.Restart(
            optopus.LocalSearch("Flip", stop(100)), restart=stop(20), stop=stop(200)
        ),
        True,
    ),
}


@pytest.mark.parametrize("case", CASES.values(), ids=CASES.keys())
def test_trajectory_records_every_improvement_of_the_best(case):
    make_problem, make_heuristic, higher_is_better = case
    report = make_heuristic().run(make_problem(), runs=3, seed=5)
    for run in report.runs:
        points = run.trajectory
        assert (points == []) == (run.best_objective == run.initial_objective)
        if not points:
            continue
        objectives = [run.initial_objective] + [objective for _, _, objective in points]
        for before, after in zip(objectives, objectives[1:]):
            assert after > before if higher_is_better else after < before
        for (t0, i0, _), (t1, i1, _) in zip(points, points[1:]):
            assert t0 <= t1
            assert i0 <= i1
        elapsed, iteration, objective = points[-1]
        assert objective == run.best_objective
        assert iteration == run.best_iteration
        assert 0.0 <= elapsed <= run.total_time_secs


def test_trajectory_is_empty_when_the_run_never_improves():
    run = optopus.LocalSearch("Flip", stop(0)).run(max_cut(), seed=1).runs[0]
    assert run.trajectory == []


def test_a_seeded_trajectory_reproduces_apart_from_the_clock():
    def steps():
        run = optopus.SimulatedAnnealing("Flip", 5.0, 0.999, stop(5_000)).run(max_cut(), seed=3)
        return [(iteration, objective) for _, iteration, objective in run.runs[0].trajectory]

    first = steps()
    assert len(first) > 1
    assert steps() == first

"""on_improvement: the callback a run reports its improvements to while it runs."""

import pytest

import optopus
from tests.helpers import stop

TIMED = optopus.StopCondition(max_duration_secs=0.3)


def max_cut():
    return optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=1))


class OneMax:
    """Maximize the number of set bits, with a flip neighborhood."""

    minimize = False

    class Flip:
        def neighbors(self, problem, x):
            return range(len(x))

        def apply(self, problem, x, i):
            return (*x[:i], not x[i], *x[i + 1 :])

        def random_neighbor(self, problem, x, rng):
            return rng.randrange(len(x))

        def delta(self, problem, x, i):
            return -1.0 if x[i] else 1.0

    def __init__(self, n):
        self.n = n
        self.neighborhoods = {"Flip": OneMax.Flip()}

    def new_solution(self, rng):
        return tuple(rng.random() < 0.5 for _ in range(self.n))

    def objective(self, x):
        return float(sum(x))


CASES = {
    "built-in": (max_cut, lambda: optopus.SimulatedAnnealing("Flip", 5.0, 0.99999, TIMED), True),
    "Python": (lambda: OneMax(300), lambda: optopus.RandomWalk("Flip", TIMED), True),
    "composed": (
        max_cut,
        lambda: optopus.Iterated(
            optopus.LocalSearch("Flip", stop(50)), optopus.RandomWalk("Flip", stop(20)), TIMED
        ),
        True,
    ),
    "Formula": (
        lambda: optopus.Formula(
            n_vars=40, objective=[([i], 1.0) for i in range(40)], direction="Minimize"
        ),
        lambda: optopus.SimulatedAnnealing("Flip", 2.0, 0.9999, TIMED),
        True,
    ),
}


@pytest.mark.parametrize("case", CASES.values(), ids=CASES.keys())
def test_improvements_reach_the_callback_while_the_run_goes_on(case):
    make_problem, make_heuristic, higher_is_better = case
    calls = []
    report = make_heuristic().run(
        make_problem(), runs=2, seed=3, on_improvement=lambda *args: calls.append(args)
    )
    for run in report.runs:
        mine = [c for c in calls if c[0] == report.runs.index(run)]
        assert mine, "every run reports at least its first improvement"
        objectives = [objective for _, _, objective in mine]
        for before, after in zip(objectives, objectives[1:]):
            assert after > before if higher_is_better else after < before
        elapsed = [t for _, t, _ in mine]
        assert elapsed == sorted(elapsed)
        assert elapsed[-1] <= run.total_time_secs
        # What was held back at the 0.1 s interval is sent, not dropped: the run ran 0.3 s.
        assert objectives[-1] == run.best_objective


def test_an_exception_in_the_callback_is_raised_and_no_later_run_starts():
    seen = []

    def fail(run, elapsed, objective):
        seen.append(run)
        raise RuntimeError("stop here")

    with pytest.raises(RuntimeError, match="stop here"):
        optopus.LocalSearch("Flip", stop(100)).run(max_cut(), runs=3, seed=1, on_improvement=fail)
    assert seen == [0]


def test_on_improvement_must_be_callable():
    with pytest.raises(TypeError, match="callable"):
        optopus.LocalSearch("Flip", stop(10)).run(max_cut(), on_improvement=3)


def test_a_run_without_a_callback_is_unchanged():
    def steps(**kwargs):
        run = optopus.LocalSearch("Flip", stop(500)).run(max_cut(), seed=4, **kwargs).runs[0]
        return run.best_objective, [p[1:] for p in run.trajectory]

    assert steps() == steps(on_improvement=lambda *args: None)

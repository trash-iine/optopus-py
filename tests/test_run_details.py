"""What ParallelTempering and ReinforcementLearningSearch report about themselves."""

import math

import pytest

import optopus
from tests.helpers import stop


def max_cut():
    return optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(20, 0.3, seed=1))


def test_parallel_tempering_reports_its_ladder_and_exchanges():
    pt = optopus.ParallelTempering(4, stop(200), beta_min=0.1, beta_max=5.0)
    report = pt.run(max_cut(), runs=2, seed=1)
    for run in report.runs:
        assert len(run.betas) == 4
        assert run.betas[0] == pytest.approx(0.1)
        assert run.betas[-1] == pytest.approx(5.0)
        # A geometric ladder: every neighboring pair has the same ratio.
        ratios = [b / a for a, b in zip(run.betas, run.betas[1:])]
        assert all(r == pytest.approx(ratios[0]) for r in ratios)
        assert 0.0 <= run.exchange_acceptance <= 1.0
        assert run.policy_weights is None


def test_reinforcement_learning_reports_the_weights_it_learned():
    rl = optopus.ReinforcementLearningSearch("Flip", stop(300))
    first = rl.run(max_cut(), seed=1).runs[0]
    weights = first.policy_weights
    assert len(weights) == 21
    assert all(math.isfinite(w) for w in weights)
    assert first.betas is None
    # The learned weights start the next search.
    again = optopus.ReinforcementLearningSearch("Flip", stop(300), policy_weights=weights)
    assert again.run(max_cut(), seed=1).runs[0].policy_weights != weights


@pytest.mark.parametrize(
    "heuristic",
    [
        optopus.LocalSearch("Flip", stop(10)),
        # Nested, the ladder belongs to one step of the run, not to the run.
        optopus.Iterated(
            optopus.ParallelTempering(3, stop(10)), optopus.RandomWalk("Flip", stop(2)), stop(2)
        ),
    ],
)
def test_other_runs_report_none(heuristic):
    run = heuristic.run(max_cut(), seed=1).runs[0]
    assert (run.betas, run.exchange_acceptance, run.policy_weights) == (None, None, None)


class Flip:
    def neighbors(self, problem, x):
        return range(len(x))

    def apply(self, problem, x, i):
        return (*x[:i], not x[i], *x[i + 1 :])

    def random_neighbor(self, problem, x, rng):
        return rng.randrange(len(x))


class OneMax:
    minimize = False
    neighborhoods = {"Flip": Flip()}  # noqa: RUF012

    def new_solution(self, rng):
        return tuple(rng.random() < 0.5 for _ in range(8))

    def objective(self, x):
        return float(sum(x))


@pytest.mark.parametrize(
    ("heuristic", "field"),
    [
        (optopus.ParallelTempering(3, stop(20)), "betas"),
        (optopus.ReinforcementLearningSearch("Flip", stop(20)), "policy_weights"),
    ],
)
def test_a_python_problem_reports_them_too(heuristic, field):
    # Every heuristic on a Python problem runs inside a guard that drives the loop itself.
    run = heuristic.run(OneMax(), seed=1).runs[0]
    assert getattr(run, field) is not None

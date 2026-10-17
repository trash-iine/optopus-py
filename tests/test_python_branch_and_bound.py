"""BranchAndBound on a problem written in Python."""

import itertools

import pytest

import optopus
from tests.helpers import stop

VALUES = [6, 5, 8, 9, 6, 7]
WEIGHTS = [2, 3, 6, 7, 5, 9]
CAPACITY = 15
PENALTY = 10.0


def score(x):
    over = max(0, sum(w * v for w, v in zip(WEIGHTS, x)) - CAPACITY)
    return sum(v * xi for v, xi in zip(VALUES, x)) - PENALTY * over


OPTIMUM = max(score(x) for x in itertools.product([0, 1], repeat=len(VALUES)))


def upper_bound(ranges):
    """Every item at its upper end, capacity ignored: no packing in the ranges does better."""
    return float(sum(v * hi for v, (_, hi) in zip(VALUES, ranges)))


class Flip:
    """Flips an item the node has not fixed."""

    def neighbors(self, problem, x):
        return [i for i, (lo, hi) in enumerate(problem.bounds) if lo < hi]

    def apply(self, problem, x, i):
        return (*x[:i], 1 - x[i], *x[i + 1 :])

    def random_neighbor(self, problem, x, rng):
        free = self.neighbors(problem, x)
        return rng.choice(free) if free else None


class Knapsack:
    minimize = False

    def __init__(self, bounds=None):
        self.bounds = bounds or [(0, 1)] * len(VALUES)
        self.neighborhoods = {"Flip": Flip()}

    def new_solution(self, rng):
        return tuple(rng.randint(lo, hi) for lo, hi in self.bounds)

    def objective(self, x):
        return score(x)

    def variable_bounds(self):
        return self.bounds

    def variable_values(self, x):
        return list(x)

    def solution_from_values(self, values):
        return tuple(values)

    def restricted(self, bounds):
        return type(self)(bounds)


@pytest.mark.parametrize(
    "inner",
    [
        optopus.LocalSearch("Flip", stop(20)),
        # Draws its moves from the rng a node inherits, since a node never calls new_solution.
        optopus.SimulatedAnnealing("Flip", 1.0, 0.9, stop(20)),
    ],
    ids=["LocalSearch", "SimulatedAnnealing"],
)
def test_it_proves_the_optimum(inner):
    bnb = optopus.BranchAndBound(inner, relaxation=upper_bound)
    run = bnb.run(Knapsack(), seed=1).runs[0]
    assert run.best_objective == OPTIMUM
    assert score(run.solution) == OPTIMUM
    assert run.proven_optimal
    assert run.dual_bound == OPTIMUM


def test_seeded_runs_repeat():
    bnb = optopus.BranchAndBound(
        optopus.SimulatedAnnealing("Flip", 1.0, 0.9, stop(10)), stop(30), relaxation=upper_bound
    )
    a = bnb.run(Knapsack(), runs=2, seed=4)
    b = bnb.run(Knapsack(), runs=2, seed=4)
    assert [r.solution for r in a.runs] == [r.solution for r in b.runs]


def test_a_problem_without_the_methods_is_told_which_are_missing():
    # Knapsack's members but `restricted`.
    plain = type("Plain", (), {k: v for k, v in vars(Knapsack).items() if k != "restricted"})
    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(5)), relaxation=upper_bound)
    with pytest.raises(ValueError, match="needs these methods on the problem: 'restricted'"):
        bnb.run(plain())


@pytest.mark.parametrize("relaxation", [None, "Binary", "Interval"])
def test_it_needs_a_relaxation_written_in_python(relaxation):
    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(5)), relaxation=relaxation)
    with pytest.raises(ValueError, match="needs a relaxation written in Python"):
        bnb.run(Knapsack())


def test_invalid_bounds_raise_value_error():
    class Inverted(Knapsack):
        def variable_bounds(self):
            return [(1, 0)] * len(VALUES)

    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(5)), relaxation=upper_bound)
    with pytest.raises(ValueError, match="lower bound 1 above upper bound 0"):
        bnb.run(Inverted())


def test_a_node_needs_the_same_neighborhoods():
    class Renamed(Knapsack):
        def restricted(self, bounds):
            node = Knapsack(bounds)
            node.neighborhoods = {"Toggle": Flip()}
            return node

    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(5)), relaxation=upper_bound)
    with pytest.raises(ValueError, match="neighborhoods"):
        bnb.run(Renamed())


@pytest.mark.parametrize("where", ["restricted", "variable_values", "relaxation"])
def test_an_exception_in_a_branching_callback_is_raised_from_run(where):
    class Boom(Exception):
        pass

    def boom(*args):
        raise Boom(where)

    problem = Knapsack()
    relaxation = upper_bound
    if where == "relaxation":
        relaxation = boom
    else:
        setattr(problem, where, boom)
    bnb = optopus.BranchAndBound(optopus.LocalSearch("Flip", stop(5)), relaxation=relaxation)
    with pytest.raises(Boom, match=where):
        bnb.run(problem, seed=1)

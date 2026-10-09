"""BranchAndBound: proven optima on tiny instances, checked against enumeration."""

import itertools
import math
import random

import pytest

import optopus


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


def flip_search():
    return optopus.LocalSearch("Flip", stop=stop(100))


def change_search():
    return optopus.LocalSearch("Change", stop=stop(100))


def assignments(n):
    return itertools.product([False, True], repeat=n)


# A 12-vertex random graph, large enough that the default bound needs dozens of nodes.
_rng = random.Random(3)
MAXCUT_EDGES = [
    (i, j, float(_rng.randint(1, 5)))
    for i in range(12)
    for j in range(i + 1, 12)
    if _rng.random() < 0.4
]
MAXCUT_OPTIMUM = max(sum(w for i, j, w in MAXCUT_EDGES if x[i] != x[j]) for x in assignments(12))

QUBO_ENTRIES = [(0, 0, -1), (0, 1, 2), (1, 1, -1), (2, 2, -3), (1, 2, 1), (0, 2, -2), (3, 3, 1)]
QUBO_OPTIMUM = min(sum(c * x[i] * x[j] for i, j, c in QUBO_ENTRIES) for x in assignments(4))

# Not satisfiable: x2 must be false, which leaves (not x1 or x3) and (x2 or not x3) at odds
# with x1.
SAT_CLAUSES = [[1, 2], [-1, 3], [2, -3], [-2], [1]]
SAT_OPTIMUM = max(
    sum(any(x[abs(lit) - 1] == (lit > 0) for lit in c) for c in SAT_CLAUSES) for x in assignments(3)
)

# A 5-cycle with a chord: covered by three vertices, not two.
VC_EDGES = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0), (0, 2, 1.0)]
VC_OPTIMUM = 3.0

KNAPSACK_VALUES = [6, 5, 4, 2]
KNAPSACK_WEIGHTS = [4, 3, 2, 1]
KNAPSACK_BUDGET = 5
KNAPSACK_OPTIMUM = 9.0


def knapsack():
    return optopus.Formula(
        n_vars=4,
        objective=[([i], float(v)) for i, v in enumerate(KNAPSACK_VALUES)],
        constraints=[
            (
                [([i], float(w)) for i, w in enumerate(KNAPSACK_WEIGHTS)],
                "Le",
                [([], float(KNAPSACK_BUDGET))],
                7.0,
            )
        ],
    )


def bounded_min():
    # min (x0 - 2)^2 + 3 x1 over x0 in [0, 4], x1 in [1, 3]: 3 at x0 = 2, x1 = 1. Reported
    # negated, as a minimized Formula is.
    return optopus.Formula(
        n_vars=2,
        objective=[([0, 0], 1.0), ([0], -4.0), ([], 4.0), ([1], 3.0)],
        direction="Minimize",
        bounds=[(0, 4), (1, 3)],
    )


PROBLEMS = {
    "MaxCut": (lambda: optopus.MaxCut.from_edges(MAXCUT_EDGES), flip_search, MAXCUT_OPTIMUM),
    "Qubo": (lambda: optopus.Qubo.from_entries(QUBO_ENTRIES), flip_search, QUBO_OPTIMUM),
    "Sat": (lambda: optopus.Sat.from_clauses(3, SAT_CLAUSES), flip_search, SAT_OPTIMUM),
    "VertexCover": (lambda: optopus.VertexCover.from_edges(VC_EDGES), flip_search, VC_OPTIMUM),
    "Formula": (knapsack, change_search, KNAPSACK_OPTIMUM),
    "Formula-min": (bounded_min, change_search, -3.0),
}


@pytest.mark.parametrize("name", PROBLEMS)
def test_proves_the_optimum_with_the_default_relaxation(name):
    make, search, optimum = PROBLEMS[name]
    report = optopus.BranchAndBound(search()).run(make(), runs=2, seed=1)
    for run in report.runs:
        assert run.best_objective == optimum
        assert run.proven_optimal is True
        assert run.dual_bound == optimum


@pytest.mark.parametrize("relaxation", ["Binary", "Eigenvalue"])
def test_maxcut_relaxations_prove_the_same_optimum(relaxation):
    mc = optopus.MaxCut.from_edges(MAXCUT_EDGES)
    run = optopus.BranchAndBound(flip_search(), relaxation=relaxation).run(mc, seed=1).runs[0]
    assert run.best_objective == MAXCUT_OPTIMUM
    assert run.proven_optimal is True


def test_eigenvalue_searches_fewer_nodes_than_binary():
    mc = optopus.MaxCut.from_edges(MAXCUT_EDGES)

    def iterations(relaxation):
        bnb = optopus.BranchAndBound(flip_search(), relaxation=relaxation)
        return bnb.run(mc, seed=1).runs[0].iterations

    assert iterations("Eigenvalue") < iterations("Binary")


@pytest.mark.parametrize(
    "heuristic",
    [
        optopus.WalkSat(stop=stop(50)),
        optopus.TabuSearch("Flip", (1, 2), stop=stop(20)),
    ],
    ids=["WalkSat", "TabuSearch"],
)
def test_a_node_is_searched_by_any_heuristic_of_the_problem(heuristic):
    sat = optopus.Sat.from_clauses(3, SAT_CLAUSES)
    run = optopus.BranchAndBound(heuristic).run(sat, seed=1).runs[0]
    assert run.best_objective == SAT_OPTIMUM
    assert run.proven_optimal is True


def test_maxcut_node_searched_by_breakout_local_search():
    mc = optopus.MaxCut.from_edges(MAXCUT_EDGES)
    bls = optopus.BreakoutLocalSearch((2, 4), 100, 2, 0.8, 0.5, stop(50))
    run = optopus.BranchAndBound(bls, relaxation="Eigenvalue").run(mc, seed=1).runs[0]
    assert run.best_objective == MAXCUT_OPTIMUM
    assert run.proven_optimal is True


def test_a_stopped_search_reports_the_bound_still_open():
    mc = optopus.MaxCut.from_edges(MAXCUT_EDGES)
    run = optopus.BranchAndBound(flip_search(), stop(5)).run(mc, seed=1).runs[0]
    assert run.proven_optimal is False
    assert run.dual_bound >= MAXCUT_OPTIMUM >= run.best_objective


def fractional_fill(ranges):
    """Dantzig's bound: the items held at 1, then the free ones by value per weight."""
    taken = [i for i, (lower, _) in enumerate(ranges) if lower == 1]
    free = [i for i, (lower, upper) in enumerate(ranges) if lower < upper]
    bound = float(sum(KNAPSACK_VALUES[i] for i in taken))
    room = max(KNAPSACK_BUDGET - sum(KNAPSACK_WEIGHTS[i] for i in taken), 0)
    for i in sorted(free, key=lambda i: KNAPSACK_VALUES[i] / KNAPSACK_WEIGHTS[i], reverse=True):
        part = min(1.0, room / KNAPSACK_WEIGHTS[i])
        bound += part * KNAPSACK_VALUES[i]
        room -= part * KNAPSACK_WEIGHTS[i]
    return bound


def test_a_relaxation_written_in_python():
    seen = []

    def relaxation(ranges):
        seen.append(ranges)
        return fractional_fill(ranges)

    run = optopus.BranchAndBound(change_search(), relaxation=relaxation).run(knapsack(), seed=1)
    run = run.runs[0]
    assert run.best_objective == KNAPSACK_OPTIMUM
    assert run.solution == [0, 1, 1, 0]
    assert run.proven_optimal is True
    assert seen
    assert all(len(r) == 4 and all(lo <= hi for lo, hi in r) for r in seen)


def test_a_python_relaxation_states_a_minimized_bound_unnegated():
    # The objective is at least 3 * x1's lower end, since (x0 - 2)^2 >= 0.
    def relaxation(ranges):
        (lo0, hi0), (lo1, _) = ranges
        nearest = min(max(2, lo0), hi0)
        return float((nearest - 2) ** 2 + 3 * lo1)

    bnb = optopus.BranchAndBound(change_search(), relaxation=relaxation)
    run = bnb.run(bounded_min(), seed=1).runs[0]
    assert run.best_objective == -3.0
    assert run.solution == [2, 1]
    assert run.proven_optimal is True
    assert run.dual_bound == -3.0


def test_a_python_relaxation_on_a_binary_problem_sees_fixed_variables():
    def relaxation(ranges):
        assert all(r in [(0, 1), (0, 0), (1, 1)] for r in ranges)
        return math.fsum(w for _, _, w in MAXCUT_EDGES)

    mc = optopus.MaxCut.from_edges(MAXCUT_EDGES[:10])
    run = optopus.BranchAndBound(flip_search(), stop(500), relaxation).run(mc, seed=1).runs[0]
    assert run.best_objective <= run.dual_bound


@pytest.mark.parametrize(
    "heuristic",
    [
        flip_search(),
        optopus.Sequential([optopus.BranchAndBound(flip_search())], stop(1)),
    ],
    ids=["LocalSearch", "nested"],
)
def test_other_heuristics_report_no_proof(heuristic):
    run = heuristic.run(optopus.MaxCut.from_edges(MAXCUT_EDGES), seed=1).runs[0]
    assert run.proven_optimal is None
    assert run.dual_bound is None


class OneFlip:
    def neighbors(self, problem, x):
        return list(range(len(x)))

    def apply(self, problem, x, i):
        return (*x[:i], not x[i], *x[i + 1 :])


class Ones:
    minimize = False

    def __init__(self):
        self.neighborhoods = {"Flip": OneFlip()}

    def new_solution(self, rng):
        return (False, False)

    def objective(self, x):
        return float(sum(x))


def raises(exc):
    def relaxation(ranges):
        raise exc

    return relaxation


def maxcut():
    return optopus.MaxCut.from_edges(MAXCUT_EDGES)


def triangle_tsp():
    return optopus.Tsp.from_coordinates([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)])


@pytest.mark.parametrize(
    ("problem", "search", "relaxation", "error", "match"),
    [
        (
            lambda: optopus.Qubo.from_entries(QUBO_ENTRIES),
            flip_search,
            "Eigenvalue",
            ValueError,
            "only available for MaxCut, not Qubo",
        ),
        (maxcut, flip_search, "Interval", ValueError, "only available for Formula"),
        (knapsack, change_search, "Binary", ValueError, "MaxCut, Qubo, Sat and VertexCover"),
        (
            triangle_tsp,
            lambda: optopus.LocalSearch("TwoOpt", stop(10)),
            None,
            ValueError,
            "BranchAndBound is only available for MaxCut",
        ),
        (Ones, flip_search, None, ValueError, "BranchAndBound is only available for MaxCut"),
        (knapsack, change_search, raises(ZeroDivisionError("boom")), ZeroDivisionError, "boom"),
        (knapsack, change_search, lambda ranges: math.nan, ValueError, "NaN"),
        (knapsack, change_search, lambda ranges: "high", TypeError, None),
    ],
    ids=[
        "eigenvalue-on-qubo",
        "interval-on-maxcut",
        "binary-on-formula",
        "tsp",
        "python-problem",
        "relaxation-raises",
        "relaxation-nan",
        "relaxation-not-a-number",
    ],
)
def test_run_errors(problem, search, relaxation, error, match):
    bnb = optopus.BranchAndBound(search(), relaxation=relaxation)
    with pytest.raises(error, match=match):
        bnb.run(problem(), seed=1)


def test_an_error_does_not_leak_into_the_next_run():
    bnb = optopus.BranchAndBound(change_search(), relaxation=raises(RuntimeError("once")))
    with pytest.raises(RuntimeError):
        bnb.run(knapsack(), seed=1)
    run = optopus.BranchAndBound(change_search()).run(knapsack(), seed=1).runs[0]
    assert run.proven_optimal is True


@pytest.mark.parametrize(
    ("relaxation", "error"),
    [("Nope", ValueError), (3, TypeError)],
)
def test_constructor_rejects_a_bad_relaxation(relaxation, error):
    with pytest.raises(error):
        optopus.BranchAndBound(flip_search(), relaxation=relaxation)

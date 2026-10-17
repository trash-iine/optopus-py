"""GeneticAlgorithm's "SubProblem" crossover on a problem written in Python."""

import itertools

import pytest

import optopus
from tests.helpers import stop

# An 8-vertex graph: two weighted 4-cycles joined by two chords.
EDGES = [
    (0, 1, 3.0), (1, 2, 1.0), (2, 3, 3.0), (3, 0, 1.0),
    (4, 5, 2.0), (5, 6, 2.0), (6, 7, 2.0), (7, 4, 2.0),
    (0, 4, 1.5), (2, 6, 0.5),
]  # fmt: skip


def cut_weight(x):
    return sum(w for i, j, w in EDGES if x[i] != x[j])


OPTIMUM = max(cut_weight(x) for x in itertools.product([False, True], repeat=8))


class Flip:
    def neighbors(self, problem, x):
        return range(len(x))

    def apply(self, problem, x, i):
        return (*x[:i], not x[i], *x[i + 1 :])


class Cut:
    """Max Cut, where a vertex may also be tied to fixed sides outside the graph.

    ``anchors[i]`` lists ``(side, weight)``: the weight counts when vertex ``i`` is not on that
    side. A sub-problem keeps the vertices two parents disagree on, and every edge to a vertex
    they agree on becomes an anchor.
    """

    minimize = False

    def __init__(self, n, edges, anchors=None):
        self.n = n
        self.edges = edges
        self.anchors = anchors or [[] for _ in range(n)]
        self.neighborhoods = {"Flip": Flip()}

    def new_solution(self, rng):
        return tuple(rng.random() < 0.5 for _ in range(self.n))

    def objective(self, x):
        inner = sum(w for i, j, w in self.edges if x[i] != x[j])
        return inner + sum(w for i in range(self.n) for side, w in self.anchors[i] if x[i] != side)

    def crossover(self, a, b, rng):
        return tuple(rng.choice(pair) for pair in zip(a, b))

    def extract_sub_problem(self, a, b):
        free = [i for i in range(self.n) if a[i] != b[i]]
        index = {v: k for k, v in enumerate(free)}
        edges, anchors = [], [list(self.anchors[v]) for v in free]
        for i, j, w in self.edges:
            if i in index and j in index:
                edges.append((index[i], index[j], w))
            elif i in index:
                anchors[index[i]].append((a[j], w))
            elif j in index:
                anchors[index[j]].append((a[i], w))
        return Cut(len(free), edges, anchors)

    def lift_solution(self, a, b, sub):
        x = list(a)
        for k, v in enumerate(i for i in range(self.n) if a[i] != b[i]):
            x[v] = sub[k]
        return tuple(x)


def sub_problem_ga(sub_heuristic=None, crossover="SubProblem"):
    return optopus.GeneticAlgorithm(
        6,
        optopus.RandomWalk("Flip", stop(2)),
        stop(30),
        crossover=crossover,
        sub_heuristic=sub_heuristic or optopus.LocalSearch("Flip", stop(50)),
    )


def test_the_sub_problem_crossover_reaches_the_optimum():
    report = sub_problem_ga().run(Cut(8, EDGES), runs=3, seed=1)
    assert report.best_objective == OPTIMUM
    best = max(report.runs, key=lambda r: r.best_objective)
    assert cut_weight(best.solution) == OPTIMUM


def test_a_tabu_search_runs_on_the_sub_problems():
    class KeyedFlip(Flip):
        def tabu_keys(self, problem, i):
            return i

    class KeyedCut(Cut):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.neighborhoods = {"Flip": KeyedFlip()}

        def extract_sub_problem(self, a, b):
            sub = super().extract_sub_problem(a, b)
            return KeyedCut(sub.n, sub.edges, sub.anchors)

    ga = sub_problem_ga(sub_heuristic=optopus.TabuSearch("Flip", (1, 2), stop(30)))
    assert ga.run(KeyedCut(8, EDGES), runs=2, seed=1).best_objective == OPTIMUM


def test_an_exception_in_the_sub_problem_is_raised_from_run():
    class Boom(Exception):
        pass

    class Faulty(Cut):
        def extract_sub_problem(self, a, b):
            sub = super().extract_sub_problem(a, b)
            sub.new_solution = lambda rng: (_ for _ in ()).throw(Boom("inside the sub-problem"))
            return sub

    with pytest.raises(Boom, match="inside the sub-problem"):
        sub_problem_ga().run(Faulty(8, EDGES), seed=1)


def test_an_exception_in_extract_sub_problem_is_raised_from_run():
    class Boom(Exception):
        pass

    class Faulty(Cut):
        def extract_sub_problem(self, a, b):
            raise Boom("cannot cut")

    with pytest.raises(Boom, match="cannot cut"):
        sub_problem_ga().run(Faulty(8, EDGES), seed=1)


def test_a_sub_problem_needs_the_same_neighborhoods():
    class Renamed(Cut):
        def extract_sub_problem(self, a, b):
            sub = super().extract_sub_problem(a, b)
            sub.neighborhoods = {"Toggle": Flip()}
            return sub

    with pytest.raises(ValueError, match=r'neighborhoods \["Flip"\] in that order'):
        sub_problem_ga().run(Renamed(8, EDGES), seed=1)


@pytest.mark.parametrize(
    ("ga", "problem", "match"),
    [
        (sub_problem_ga(crossover="Uniform"), Cut, "invalid crossover 'Uniform'"),
        (
            optopus.GeneticAlgorithm(
                4,
                optopus.RandomWalk("Flip", stop(2)),
                stop(3),
                sub_heuristic=optopus.LocalSearch("Flip", stop(5)),
            ),
            Cut,
            "only applies to crossover 'SubProblem'",
        ),
    ],
)
def test_misconfigured_sub_problem_crossover_raises_value_error(ga, problem, match):
    with pytest.raises(ValueError, match=match):
        ga.run(problem(8, EDGES), seed=1)


def test_a_problem_without_the_methods_is_told_which_are_missing():
    class Plain:
        minimize = False
        neighborhoods = {"Flip": Flip()}  # noqa: RUF012

        def new_solution(self, rng):
            return (False,) * 4

        def objective(self, x):
            return 0.0

        def crossover(self, a, b, rng):
            return a

    with pytest.raises(ValueError, match="'extract_sub_problem', 'lift_solution'"):
        sub_problem_ga().run(Plain(), seed=1)

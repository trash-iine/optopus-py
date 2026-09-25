"""Problems defined in Python, run through the generic heuristics."""

import itertools
import math

import pytest

import optopus

# Six points on a hexagon, jittered so the optimal tour is unique up to direction.
POINTS = [(0.0, 0.0), (2.0, 0.1), (3.1, 1.9), (2.0, 3.8), (0.1, 4.0), (-1.0, 2.0)]


def tour_length(points, tour):
    return sum(
        math.dist(points[tour[i]], points[tour[(i + 1) % len(tour)]])
        for i in range(len(tour))
    )


TSP_OPTIMUM = min(
    tour_length(POINTS, (0,) + rest) for rest in itertools.permutations(range(1, 6))
)


class TwoOpt:
    """Reverse the segment tour[i..=j]. Solutions are tuples, so apply copies."""

    def neighbors(self, problem, tour):
        n = len(tour)
        return [(i, j) for i in range(1, n - 1) for j in range(i + 1, n)]

    def apply(self, problem, tour, move):
        i, j = move
        return tour[:i] + tour[i : j + 1][::-1] + tour[j + 1 :]

    def delta(self, problem, tour, move):
        i, j = move
        p, n = problem.points, len(tour)
        a, b = tour[i - 1], tour[i]
        c, d = tour[j], tour[(j + 1) % n]
        return (
            math.dist(p[a], p[c])
            + math.dist(p[b], p[d])
            - math.dist(p[a], p[b])
            - math.dist(p[c], p[d])
        )

    def tabu_keys(self, problem, move):
        return move


class Swap:
    """Swap two positions. No delta, so the Rust side prices it by applying."""

    def neighbors(self, problem, tour):
        n = len(tour)
        return [(i, j) for i in range(1, n) for j in range(i + 1, n)]

    def apply(self, problem, tour, move):
        i, j = move
        t = list(tour)
        t[i], t[j] = t[j], t[i]
        return tuple(t)

    def random_neighbor(self, problem, tour, rng):
        i, j = rng.sample(range(1, len(tour)), 2)
        return (min(i, j), max(i, j))


class Tsp:
    minimize = True

    def __init__(self, points):
        self.points = points
        self.neighborhoods = {"TwoOpt": TwoOpt(), "Swap": Swap()}

    def new_solution(self, rng):
        rest = list(range(1, len(self.points)))
        rng.shuffle(rest)
        return (0, *rest)

    def objective(self, tour):
        return tour_length(self.points, tour)


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


GENERIC = [
    optopus.LocalSearch(neighbor="TwoOpt", stop=stop(200)),
    optopus.SimulatedAnnealing(
        neighbor="TwoOpt", initial_temperature=1.0, cooling_rate=0.99, stop=stop(3000)
    ),
    optopus.TabuSearch(neighbor="TwoOpt", tabu_tenure=(1, 3), stop=stop(300)),
    optopus.LateAcceptanceHillClimbing(neighbor="TwoOpt", history_length=10, stop=stop(3000)),
    optopus.BeamSearch(neighbor="TwoOpt", beam_width=4, stop=stop(100)),
    optopus.PopulationAnnealing(
        population_size=8, stop=stop(30), sweeps_per_step=2, neighbor="TwoOpt", sweep_length=10
    ),
]


@pytest.mark.parametrize("heuristic", GENERIC, ids=lambda h: type(h).__name__)
def test_generic_heuristics_reach_the_tsp_optimum(heuristic):
    report = heuristic.run(Tsp(POINTS), runs=3, seed=1)
    assert report.best_objective == pytest.approx(TSP_OPTIMUM)
    best = min(report.runs, key=lambda r: r.best_objective)
    assert sorted(best.solution) == list(range(6))
    assert tour_length(POINTS, best.solution) == pytest.approx(best.best_objective)


def test_vns_mixes_neighborhoods_of_one_problem():
    vns = optopus.VariableNeighborhoodSearch(
        search=optopus.LocalSearch(neighbor="TwoOpt", stop=stop(100)),
        shakes=[
            optopus.RandomWalk(neighbor="Swap", stop=stop(1)),
            optopus.RandomWalk(neighbor="Swap", stop=stop(3)),
        ],
        stop=stop(50),
    )
    report = vns.run(Tsp(POINTS), runs=2, seed=3)
    assert report.best_objective == pytest.approx(TSP_OPTIMUM)


def test_a_seeded_run_reproduces():
    sa = optopus.SimulatedAnnealing(
        neighbor="Swap", initial_temperature=2.0, cooling_rate=0.999, stop=stop(500)
    )
    a = sa.run(Tsp(POINTS), runs=2, seed=7)
    b = sa.run(Tsp(POINTS), runs=2, seed=7)
    assert [r.solution for r in a.runs] == [r.solution for r in b.runs]
    assert [r.best_iteration for r in a.runs] == [r.best_iteration for r in b.runs]


def test_delta_matches_pricing_by_apply():
    """Dropping delta changes the cost of a step, not the search."""

    class SlowTsp(Tsp):
        def __init__(self, points):
            super().__init__(points)
            self.neighborhoods = {"TwoOpt": _without(TwoOpt, "delta")()}

    ls = optopus.LocalSearch(neighbor="TwoOpt", stop=stop(100))
    fast = ls.run(Tsp(POINTS), runs=3, seed=5)
    slow = ls.run(SlowTsp(POINTS), runs=3, seed=5)
    assert [r.solution for r in fast.runs] == [r.solution for r in slow.runs]


def _without(cls, name):
    """A copy of `cls` without the method `name`."""
    body = {k: v for k, v in vars(cls).items() if k != name}
    return type(cls.__name__, cls.__bases__, body)


class OneMax:
    """Maximize the number of set bits, with a flip neighborhood."""

    minimize = False

    class Flip:
        def neighbors(self, problem, x):
            return range(len(x))

        def apply(self, problem, x, i):
            return x[:i] + (not x[i],) + x[i + 1 :]

        def delta(self, problem, x, i):
            return -1.0 if x[i] else 1.0

        def tabu_keys(self, problem, i):
            return i

    def __init__(self, n):
        self.n = n
        self.neighborhoods = {"Flip": OneMax.Flip()}

    def new_solution(self, rng):
        return tuple(rng.random() < 0.5 for _ in range(self.n))

    def objective(self, x):
        return float(sum(x))


def test_maximization_direction():
    report = optopus.TabuSearch(neighbor="Flip", tabu_tenure=(2, 4), stop=stop(100)).run(
        OneMax(20), runs=2, seed=0
    )
    assert report.best_objective == 20.0
    assert all(report.runs[0].solution)


def test_unknown_neighbor_is_rejected():
    with pytest.raises(ValueError, match="invalid neighbor 'Flip'.*TwoOpt, Swap"):
        optopus.LocalSearch(neighbor="Flip", stop=stop(10)).run(Tsp(POINTS))


def test_tabu_search_needs_tabu_keys():
    with pytest.raises(ValueError, match="tabu_keys.*'Swap'"):
        optopus.TabuSearch(neighbor="Swap", tabu_tenure=(1, 2), stop=stop(10)).run(
            Tsp(POINTS)
        )


def test_problem_specific_heuristics_are_rejected():
    with pytest.raises(ValueError, match="WalkSat is only available for Sat"):
        optopus.WalkSat(stop=stop(10), noise=0.5, adaptive=False).run(Tsp(POINTS))


def test_missing_protocol_parts_are_reported():
    class NoObjective:
        minimize = True
        neighborhoods = {"TwoOpt": TwoOpt()}

        def new_solution(self, rng):
            return (0, 1, 2)

    with pytest.raises(TypeError, match="'objective'"):
        optopus.LocalSearch(neighbor="TwoOpt", stop=stop(10)).run(NoObjective())


def test_an_exception_in_a_callback_propagates_and_stops_the_run():
    class Boom(Exception):
        pass

    calls = []

    class Faulty(Tsp):
        def objective(self, tour):
            calls.append(tour)
            if len(calls) > 3:
                raise Boom("objective failed")
            return super().objective(tour)

    with pytest.raises(Boom, match="objective failed"):
        optopus.LocalSearch(neighbor="TwoOpt", stop=stop(10_000)).run(Faulty(POINTS), runs=5)
    # The first failure ends every run, rather than each of them failing on its own.
    assert len(calls) == 4


# --- Heuristics that need more than moves ------------------------------------


class RuinTsp(Tsp):
    """The TSP with the ruin methods ALNS needs. The tour is the one container, a list the
    methods edit in place, and a city goes between two neighbors on the cycle."""

    def d(self, a, b):
        return math.dist(self.points[a], self.points[b])

    def to_partial(self, tour):
        return list(tour)

    def finish(self, partial):
        return tuple(partial)

    def elements(self, partial):
        return partial

    def remove_all(self, partial, elements):
        gone = set(elements)
        partial[:] = [c for c in partial if c not in gone]

    def removal_gain(self, partial, city):
        i, n = partial.index(city), len(partial)
        prev, nxt = partial[i - 1], partial[(i + 1) % n]
        return self.d(prev, city) + self.d(city, nxt) - self.d(prev, nxt)

    def relatedness(self, a, b):
        return self.d(a, b)

    def num_buckets(self, partial):
        return 1

    def num_places(self, partial, bucket):
        return max(len(partial), 1)

    def insertion_cost(self, partial, bucket, place, city):
        if not partial:
            return 0.0
        prev, nxt = partial[place - 1], partial[place % len(partial)]
        return self.d(prev, city) + self.d(city, nxt) - self.d(prev, nxt)

    def insert(self, partial, bucket, place, city):
        partial.insert(place, city)

    def crossover(self, a, b, rng):
        # Order crossover: a slice of `a`, the rest in `b`'s order.
        i, j = sorted(rng.sample(range(len(a)), 2))
        kept = a[i:j]
        rest = [c for c in b if c not in kept]
        return tuple(rest[:i] + list(kept) + rest[i:])


def test_alns_runs_on_a_python_problem_with_the_ruin_methods():
    alns = optopus.AdaptiveLargeNeighborhoodSearch(stop(300))
    report = alns.run(RuinTsp(POINTS), runs=2, seed=1)
    assert report.best_objective == pytest.approx(TSP_OPTIMUM)
    assert sorted(report.runs[0].solution) == list(range(6))


def test_alns_uses_partial_objective_and_repair_around():
    calls = {"partial_objective": 0, "repair_around": 0}

    class Repaired(RuinTsp):
        def partial_objective(self, partial):
            calls["partial_objective"] += 1
            return tour_length(self.points, partial)

        def repair_around(self, partial, anchors, rng):
            calls["repair_around"] += 1
            assert all(c in partial for c in anchors)

    report = optopus.AdaptiveLargeNeighborhoodSearch(stop(100)).run(Repaired(POINTS), seed=2)
    assert report.best_objective == pytest.approx(TSP_OPTIMUM)
    assert calls["partial_objective"] > 0
    assert calls["repair_around"] > 0


def test_alns_names_the_missing_ruin_methods():
    with pytest.raises(ValueError, match="'to_partial', 'finish'.*'insert'"):
        optopus.AdaptiveLargeNeighborhoodSearch(stop(10)).run(Tsp(POINTS))


def test_ga_uses_the_problems_crossover():
    ga = optopus.GeneticAlgorithm(
        population_size=6, mutation=optopus.LocalSearch("TwoOpt", stop(50)), stop=stop(20)
    )
    report = ga.run(RuinTsp(POINTS), runs=2, seed=3)
    assert report.best_objective == pytest.approx(TSP_OPTIMUM)


def test_ga_needs_a_crossover_method():
    ga = optopus.GeneticAlgorithm(6, optopus.LocalSearch("TwoOpt", stop(5)), stop(5))
    with pytest.raises(ValueError, match="needs a 'crossover' method"):
        ga.run(Tsp(POINTS))


def test_ga_rejects_a_named_crossover_on_a_python_problem():
    ga = optopus.GeneticAlgorithm(
        6, optopus.LocalSearch("TwoOpt", stop(5)), stop(5), crossover="Order"
    )
    with pytest.raises(ValueError, match="one crossover"):
        ga.run(RuinTsp(POINTS))


@pytest.mark.parametrize("with_distance", [False, True], ids=["eq", "distance"])
def test_bls_from_parts_runs_on_a_python_problem(with_distance):
    distances = []

    class Measured(OneMax):
        if with_distance:

            def distance(self, a, b):
                distances.append(1)
                return sum(x != y for x, y in zip(a, b))

    bls = optopus.BreakoutLocalSearch.from_parts(
        descent=optopus.LocalSearch("Flip", stop(100)),
        random=optopus.RandomWalk("Flip", stop(1)),
        directed=[(optopus.TabuSearch("Flip", (2, 4), stop(1)), 1.0)],
        tabu_tenure=(2, 4),
        t=20,
        l0=2,
        p0=0.5,
        stop=stop(30),
    )
    assert bls.run(Measured(16), seed=4).best_objective == 16.0
    assert bool(distances) == with_distance


def test_composed_heuristics_run_on_a_python_problem():
    composed = [
        optopus.Iterated(
            optopus.LocalSearch("TwoOpt", stop(100)), optopus.RandomWalk("Swap", stop(2)), stop(10)
        ),
        optopus.Restart(optopus.LocalSearch("TwoOpt", stop(100)), stop(20), stop(60)),
        optopus.Sequential(
            [optopus.RandomWalk("Swap", stop(2)), optopus.LocalSearch("TwoOpt", stop(100))],
            stop(10),
        ),
        optopus.ReinforcementLearningSearch("TwoOpt", stop(500)),
    ]
    for h in composed:
        report = h.run(Tsp(POINTS), runs=2, seed=6)
        assert report.best_objective == pytest.approx(TSP_OPTIMUM), type(h).__name__

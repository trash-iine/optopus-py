"""FlowShop, Qap and BinPacking on tiny instances with optima known by enumeration."""

import itertools

import pytest

import optopus


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)


# Three machines, four jobs: processing_times[machine][job].
FLOW_TIMES = [[3, 2, 4, 1], [2, 5, 1, 3], [4, 1, 2, 2]]


def makespan(times, sequence):
    done = [0] * len(times)
    for job in sequence:
        for m, row in enumerate(times):
            done[m] = max(done[m], done[m - 1] if m else 0) + row[job]
    return done[-1]


FLOW_OPTIMUM = min(makespan(FLOW_TIMES, p) for p in itertools.permutations(range(4)))

QAP_FLOW = [[0, 3, 0, 2], [3, 0, 0, 1], [0, 0, 0, 4], [2, 1, 4, 0]]
QAP_DISTANCE = [[0, 22, 53, 53], [22, 0, 40, 62], [53, 40, 0, 55], [53, 62, 55, 0]]


def qap_cost(p):
    return sum(QAP_FLOW[i][j] * QAP_DISTANCE[p[i]][p[j]] for i in range(4) for j in range(4))


QAP_OPTIMUM = min(qap_cost(p) for p in itertools.permutations(range(4)))

# Pairs to 10 exactly, so the optimum fills four bins and its objective is 4 - 1 = 3.
BIN_SIZES = [6, 4, 5, 5, 3, 7, 2, 8]
BIN_CAPACITY = 10
BIN_OPTIMUM = 3.0


def flow_shop():
    return optopus.FlowShop.from_processing_times(FLOW_TIMES, name="tiny")


def qap():
    return optopus.Qap.from_matrices(QAP_FLOW, QAP_DISTANCE)


def bin_packing():
    return optopus.BinPacking.from_sizes(BIN_CAPACITY, BIN_SIZES)


def check_flow_shop(problem, run):
    assert sorted(run.solution) == list(range(4))
    assert run.best_objective == problem.makespan(run.solution)


def check_qap(problem, run):
    assert sorted(run.solution) == list(range(4))
    assert run.best_objective == problem.cost(run.solution)


def check_bin_packing(problem, run):
    assert problem.evaluate_bins(run.solution)["objective"] == pytest.approx(run.best_objective)
    loads = [0] * (max(run.solution) + 1)
    for item, b in enumerate(run.solution):
        loads[b] += BIN_SIZES[item]
    assert max(loads) <= BIN_CAPACITY


CASES = {
    "FlowShop-LocalSearch": (flow_shop, optopus.LocalSearch("Relocate", stop(100))),
    "FlowShop-TabuSearch": (flow_shop, optopus.TabuSearch("Swap", (1, 2), stop(100))),
    "FlowShop-IteratedGreedy": (flow_shop, optopus.IteratedGreedy(stop(50), removal_count=2)),
    "FlowShop-ALNS": (
        flow_shop,
        optopus.AdaptiveLargeNeighborhoodSearch(stop(50), removal_fraction=0.5),
    ),
    "FlowShop-GRASP": (
        flow_shop,
        optopus.Iterated(
            optopus.LocalSearch("Relocate", stop(50)),
            optopus.GreedyRandomizedConstruction(alpha=0.3),
            stop(200),
        ),
    ),
    "FlowShop-GeneticAlgorithm": (
        flow_shop,
        optopus.GeneticAlgorithm(6, optopus.LocalSearch("Swap", stop(10)), stop(50)),
    ),
    "Qap-TabuSearch": (qap, optopus.TabuSearch("Swap", (1, 2), stop(100))),
    "Qap-SimulatedAnnealing": (qap, optopus.SimulatedAnnealing("Swap", 100.0, 0.99, stop(500))),
    "Qap-GeneticAlgorithm": (
        qap,
        optopus.GeneticAlgorithm(6, optopus.LocalSearch("Swap", stop(10)), stop(50)),
    ),
    "BinPacking-ALNS": (
        bin_packing,
        optopus.AdaptiveLargeNeighborhoodSearch(stop(200), removal_fraction=0.4),
    ),
    "BinPacking-GRASP": (bin_packing, optopus.GreedyRandomizedConstruction()),
    "BinPacking-GeneticAlgorithm": (
        bin_packing,
        optopus.GeneticAlgorithm(6, optopus.LocalSearch("Relocate", stop(10)), stop(50)),
    ),
}

OPTIMA = {"FlowShop": FLOW_OPTIMUM, "Qap": QAP_OPTIMUM, "BinPacking": BIN_OPTIMUM}
CHECKS = {"FlowShop": check_flow_shop, "Qap": check_qap, "BinPacking": check_bin_packing}


@pytest.mark.parametrize("case", CASES)
def test_reaches_the_optimum(case):
    make, heuristic = CASES[case]
    problem = make()
    kind = case.split("-")[0]
    report = heuristic.run(problem, runs=2, seed=1)
    assert report.best_objective == pytest.approx(OPTIMA[kind])
    for run in report.runs:
        CHECKS[kind](problem, run)


@pytest.mark.parametrize(
    ("make", "neighbor"),
    [
        (flow_shop, "Relocate"),
        (flow_shop, "Swap"),
        (qap, "Swap"),
        (bin_packing, "Relocate"),
        (bin_packing, "Swap"),
    ],
)
def test_every_neighborhood_runs_under_a_generic_heuristic(make, neighbor):
    problem = make()
    report = optopus.SimulatedAnnealing(neighbor, 1.0, 0.99, stop(200)).run(problem, seed=3)
    CHECKS[type(problem).__name__](problem, report.runs[0])


def test_flow_shop_reads_back_and_scores():
    fs = flow_shop()
    assert (fs.num_jobs(), fs.num_machines()) == (4, 3)
    assert repr(fs) == 'FlowShop(name="tiny", n_jobs=4, n_machines=3)'
    assert fs.makespan([0, 1, 2, 3]) == makespan(FLOW_TIMES, [0, 1, 2, 3])
    neh = fs.neh()
    assert sorted(neh) == list(range(4))
    assert fs.makespan(neh) == FLOW_OPTIMUM


def test_flow_shop_load_file(tmp_path):
    path = tmp_path / "tiny.txt"
    rows = "\n".join(" ".join(map(str, row)) for row in FLOW_TIMES)
    path.write_text(f"4 3\n{rows}\n")
    fs = optopus.FlowShop.load_file(str(path))
    assert repr(fs) == 'FlowShop(name="tiny", n_jobs=4, n_machines=3)'
    assert fs.makespan([0, 1, 2, 3]) == makespan(FLOW_TIMES, [0, 1, 2, 3])


def test_qap_reads_back_and_scores():
    q = qap()
    assert q.size() == 4
    assert q.cost([0, 1, 2, 3]) == qap_cost([0, 1, 2, 3])


def test_qap_load_file(tmp_path):
    path = tmp_path / "tiny.dat"

    def matrix(m):
        return "\n".join(" ".join(map(str, row)) for row in m)

    path.write_text(f"4\n\n{matrix(QAP_FLOW)}\n\n{matrix(QAP_DISTANCE)}\n")
    q = optopus.Qap.load_file(str(path))
    assert repr(q) == 'Qap(name="tiny", n=4)'
    assert q.cost([3, 2, 1, 0]) == qap_cost([3, 2, 1, 0])


def test_bin_packing_reads_back_and_scores():
    bp = bin_packing()
    assert (bp.capacity(), bp.sizes(), bp.num_items()) == (10, BIN_SIZES, 8)
    assert bp.lower_bound() == 4
    scored = bp.evaluate_bins([5, 5, 2, 2, 9, 9, 7, 7])
    assert scored["bins"] == [0, 0, 1, 1, 2, 2, 3, 3]
    assert scored["loads"] == [10, 10, 10, 10]
    assert scored["num_bins"] == 4
    assert scored["objective"] == pytest.approx(3.0)


def test_bin_packing_load_file(tmp_path):
    path = tmp_path / "tiny.txt"
    path.write_text(f"8 10\n{' '.join(map(str, BIN_SIZES))}\n")
    bp = optopus.BinPacking.load_file(str(path))
    assert repr(bp) == 'BinPacking(name="tiny", n_items=8, capacity=10)'


@pytest.mark.parametrize(
    ("build", "match"),
    [
        (lambda: optopus.FlowShop.from_processing_times([]), "at least one machine"),
        (lambda: optopus.FlowShop.from_processing_times([[]]), "at least one machine"),
        (lambda: optopus.FlowShop.from_processing_times([[1, 2], [3]]), "machine 1 has 1"),
        (lambda: optopus.FlowShop.from_processing_times([[2**31, 2**31]]), "past the"),
        (lambda: flow_shop().makespan([0, 1, 1, 2]), "twice"),
        (lambda: flow_shop().makespan([0, 1, 2]), "4 entries"),
        (lambda: flow_shop().makespan([0, 1, 2, 4]), "outside"),
        (lambda: optopus.Qap.from_matrices([], []), "at least one facility"),
        (lambda: optopus.Qap.from_matrices([[1, 2]], [[1]]), "square matrices of the same size"),
        (
            lambda: optopus.Qap.from_matrices([[0]], [[0, 1], [1, 0]]),
            "square matrices of the same size",
        ),
        (lambda: optopus.Qap.from_matrices([[10**10]], [[10**10]]), "overflow"),
        (lambda: qap().cost([0, 0, 1, 2]), "twice"),
        (lambda: optopus.BinPacking.from_sizes(0, [1]), "capacity"),
        (lambda: optopus.BinPacking.from_sizes(10, []), "empty"),
        (lambda: optopus.BinPacking.from_sizes(10, [11]), "fit in a bin"),
        (lambda: optopus.BinPacking.from_sizes(10, [0]), "fit in a bin"),
        (lambda: bin_packing().evaluate_bins([0] * 8), "over the capacity"),
        (lambda: bin_packing().evaluate_bins([0, 1]), "8 items"),
        (lambda: optopus.FlowShop.load_file("no-such-instance.txt"), "no-such-instance"),
        (lambda: optopus.Qap.load_file("no-such-instance.dat"), "no-such-instance"),
        (lambda: optopus.BinPacking.load_file("no-such-instance.txt"), "no-such-instance"),
    ],
)
def test_invalid_input_raises_value_error(build, match):
    with pytest.raises(ValueError, match=match):
        build()


def test_bin_packing_load_file_rejects_an_item_too_large(tmp_path):
    path = tmp_path / "bad.txt"
    path.write_text("2 10\n4 11\n")
    with pytest.raises(ValueError, match="fit in a bin"):
        optopus.BinPacking.load_file(str(path))


@pytest.mark.parametrize(
    ("heuristic", "problem", "match"),
    [
        (optopus.IteratedGreedy(stop(5)), qap, "only available for FlowShop"),
        (optopus.GreedyRandomizedConstruction(), qap, "only available for Tsp, Vrp, FlowShop"),
        (optopus.AdaptiveLargeNeighborhoodSearch(stop(5)), qap, "only available for Vrp, Tsp"),
        (optopus.LocalSearch("Flip", stop(5)), flow_shop, "'Relocate' or 'Swap'"),
        (optopus.LocalSearch("Relocate", stop(5)), qap, "'Swap'"),
        (
            optopus.GeneticAlgorithm(4, optopus.LocalSearch("Swap", stop(5)), stop(5), "Uniform"),
            bin_packing,
            "'Group'",
        ),
    ],
)
def test_run_errors(heuristic, problem, match):
    with pytest.raises(ValueError, match=match):
        heuristic.run(problem(), seed=1)


@pytest.mark.parametrize(
    "build",
    [
        lambda: optopus.IteratedGreedy(stop(5), removal_count=0),
        lambda: optopus.IteratedGreedy(stop(5), temperature_factor=0.0),
        lambda: optopus.IteratedGreedy(stop(5), temperature_factor=float("inf")),
        lambda: optopus.GreedyRandomizedConstruction(alpha=-0.1),
        lambda: optopus.GreedyRandomizedConstruction(alpha=1.5),
    ],
)
def test_constructor_rejects_bad_parameters(build):
    with pytest.raises(ValueError):
        build()


def test_one_construction_per_call():
    report = optopus.GreedyRandomizedConstruction(alpha=0.0).run(flow_shop(), runs=3, seed=1)
    assert all(run.iterations == 1 for run in report.runs)

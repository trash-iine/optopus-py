"""Instance files read by ``load_file``, and the graph a file holds."""

import pytest

import optopus
from tests.helpers import TRIANGLE, TRIANGLE_OPTIMUM, stop

# The triangle of `TRIANGLE` in the `N M / i j w` edge-list format, 1-indexed.
TRIANGLE_FILE = "3 3\n1 2 1\n2 3 2\n1 3 3\n"


def write(tmp_path, text, name="instance.txt"):
    path = tmp_path / name
    path.write_text(text)
    return str(path)


def test_graph_round_trips_through_a_file(tmp_path):
    # Vertex 2 has no edge; the header still counts it, so it survives the round trip.
    g = optopus.Graph.from_edges([(0, 1, 1.5), (1, 3, 0.1)])
    path = str(tmp_path / "g.txt")
    g.write_to_file(path)
    assert (tmp_path / "g.txt").read_text().splitlines()[0] == "4 2"
    loaded = optopus.Graph.load_file(path)
    assert loaded.edges() == g.edges()


def test_graph_reads_unweighted_edges_as_weight_one(tmp_path):
    g = optopus.Graph.load_file(write(tmp_path, "3 2\n1 2\n2 3 4\n"))
    assert g.edges() == [(0, 1, 1.0), (1, 2, 4.0)]


def test_graph_queries():
    g = optopus.Graph.from_edges([*TRIANGLE, (2, 3, 5.0)])
    assert [g.degree(v) for v in range(5)] == [2, 2, 3, 1, 0]
    assert g.neighbors(2) == [(0, 3.0), (1, 2.0), (3, 5.0)]
    assert g.neighbors(9) == []
    assert g.has_edge(1, 2)
    assert g.has_edge(2, 1)
    assert not g.has_edge(0, 3)
    assert g.weight(0, 2) == 3.0
    assert g.weight(0, 3) == 0.0
    sub = g.induced_subgraph([0, 2, 3])
    assert sub.edges() == [(0, 2, 3.0), (2, 3, 5.0)]
    assert g.induced_subgraph({0, 1}).edges() == [(0, 1, 1.0)]


def test_graph_write_reports_a_missing_directory(tmp_path):
    path = tmp_path / "no-such-dir" / "g.txt"
    with pytest.raises(FileNotFoundError) as info:
        optopus.Graph.from_edges(TRIANGLE).write_to_file(str(path))
    assert info.value.filename == str(path)


def test_max_cut_load_file(tmp_path):
    mc = optopus.MaxCut.load_file(write(tmp_path, TRIANGLE_FILE))
    assert mc.graph().edges() == [(0, 1, 1.0), (0, 2, 3.0), (1, 2, 2.0)]
    report = optopus.LocalSearch("Flip", stop(100)).run(mc, runs=3, seed=1)
    assert report.best_objective == TRIANGLE_OPTIMUM


def test_vertex_cover_load_file(tmp_path):
    vc = optopus.VertexCover.load_file(write(tmp_path, TRIANGLE_FILE))
    assert vc.graph().num_edges() == 3
    # A triangle needs two of its vertices.
    report = optopus.TabuSearch("Flip", (1, 2), stop(200)).run(vc, seed=1)
    assert sum(report.runs[0].solution) == 2


@pytest.mark.parametrize(("num_colors", "expected"), [(None, 3), (5, 5)])
def test_graph_coloring_load_file(tmp_path, num_colors, expected):
    gc = optopus.GraphColoring.load_file(write(tmp_path, TRIANGLE_FILE), num_colors=num_colors)
    assert gc.num_colors() == expected
    assert gc.graph().num_edges() == 3


def test_qubo_load_file(tmp_path):
    # minimize -x0 - x1 + 3 x0 x1: one variable set, energy -1.
    qubo = optopus.Qubo.load_file(write(tmp_path, "2 3\n1 1 -1\n2 2 -1\n1 2 3\n"))
    report = optopus.LocalSearch("Flip", stop(100)).run(qubo, runs=3, seed=1)
    assert report.best_objective == -1
    assert sum(report.runs[0].solution) == 1


def test_sat_load_file(tmp_path):
    cnf = "c a comment\np cnf 3 3\n1 -2 0\n2 3 0\n-1 -3 0\n"
    sat = optopus.Sat.load_file(write(tmp_path, cnf, "f.cnf"))
    assert repr(sat) == "Sat(n_vars=3, n_clauses=3)"
    assert optopus.WalkSat(stop(200)).run(sat, seed=1).best_objective == 3


def test_job_shop_load_file(tmp_path):
    # Two jobs on two machines in opposite orders: the best schedule overlaps them, makespan 5.
    js = optopus.JobShopScheduling.load_file(write(tmp_path, "2 2\n0 3 1 2\n1 2 0 1\n", "js"))
    assert repr(js) == 'JobShopScheduling(name="js", n_jobs=2, n_machines=2)'
    report = optopus.LocalSearch("Swap", stop(100)).run(js, runs=3, seed=1)
    assert report.best_objective == 5


@pytest.mark.parametrize(
    ("load", "text"),
    [
        (optopus.Graph.load_file, "3 1\n0 2 1\n"),
        (optopus.MaxCut.load_file, "3 1\na b c\n"),
        (optopus.VertexCover.load_file, ""),
        (optopus.GraphColoring.load_file, "3 1\n1 9 1\n"),
        (optopus.Sat.load_file, "p cnf 2 1\n1 5 0\n"),
        (optopus.Sat.load_file, "1 2 0\n"),
        (optopus.JobShopScheduling.load_file, "2 2\n0 3 5 2\n1 4 0 1\n"),
        (optopus.JobShopScheduling.load_file, "2 2\n0 3\n1 4 0 1\n"),
    ],
)
def test_a_file_that_does_not_parse_raises_value_error(tmp_path, load, text):
    with pytest.raises(ValueError):
        load(write(tmp_path, text))


@pytest.mark.parametrize(
    "load",
    [
        optopus.Graph.load_file,
        optopus.MaxCut.load_file,
        optopus.Qubo.load_file,
        optopus.Sat.load_file,
        optopus.VertexCover.load_file,
        optopus.GraphColoring.load_file,
        optopus.JobShopScheduling.load_file,
    ],
)
def test_a_missing_file_raises_file_not_found(tmp_path, load):
    path = str(tmp_path / "missing.txt")
    with pytest.raises(FileNotFoundError) as info:
        load(path)
    assert info.value.filename == path

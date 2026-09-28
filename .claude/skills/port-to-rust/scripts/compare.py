"""Compare a Python optopus-py run with its Rust port.

Both sides write a JSON report with the same layout: the Rust port through the
``harness.rs`` template (``cargo run --release -- --json rs.json``), the Python
original through :func:`dump_report`. Then::

    python compare.py py.json rs.json --mode exact   # same core, same seeds: must match
    python compare.py py.json rs.json --mode stat    # different random streams: compare quality

``exact`` fits a built-in problem stopped by iterations or failed updates only.
``stat`` fits a problem written in Python (its ``random.Random`` draws differ from
Rust's ``SmallRng``) and any run with a time limit. The exit status is 0 on a pass.

:func:`check_objective` re-scores the Rust solutions with the Python objective,
which catches an objective ported wrongly even where ``stat`` cannot.
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Any, Callable

# Per-run fields that are deterministic given the core, the heuristic and the seed.
EXACT_FIELDS = (
    "seed",
    "best_objective",
    "initial_objective",
    "best_iteration",
    "n_accepted",
    "n_rejected",
    "n_best_updates",
    "solution",
)


def _jsonable(value: Any) -> Any:
    """A solution as JSON: tuples become lists, anything unknown its repr."""
    if isinstance(value, (bool, int, float, str)) or value is None:
        return value
    if isinstance(value, (list, tuple)):
        return [_jsonable(v) for v in value]
    if isinstance(value, dict):
        return {str(k): _jsonable(v) for k, v in value.items()}
    return repr(value)


def dump_report(report: Any, path: str | Path, *, minimize: bool) -> None:
    """Write an optopus-py ``RunReport`` in the layout the Rust harness writes.

    Args:
        report: The ``RunReport`` returned by ``heuristic.run``.
        path: Where to write the JSON.
        minimize: The problem's direction, as in ``references/api-mapping.md``.
    """
    runs = [
        {
            "seed": r.seed,
            "best_objective": r.best_objective,
            "initial_objective": r.initial_objective,
            "improvement": r.improvement,
            "best_iteration": r.best_iteration,
            "n_accepted": r.n_accepted,
            "n_rejected": r.n_rejected,
            "n_best_updates": r.n_best_updates,
            "time_to_best_secs": r.time_to_best_secs,
            "total_time_secs": r.total_time_secs,
            "solution": _jsonable(r.solution),
        }
        for r in report.runs
    ]
    data = {
        "source": "python",
        "minimize": minimize,
        "best_objective": report.best_objective,
        "avg_objective": report.avg_objective,
        "worst_objective": report.worst_objective,
        "std_objective": report.std_objective,
        "avg_total_time_secs": report.avg_total_time_secs,
        "runs": runs,
    }
    Path(path).write_text(json.dumps(data, indent=2))


def check_objective(
    rust_json: str | Path,
    objective: Callable[[Any], float],
    decode: Callable[[Any], Any] = lambda x: x,
    rel_tol: float = 1e-9,
) -> bool:
    """Re-score each Rust best solution with the Python objective.

    Args:
        rust_json: The report the Rust port wrote.
        objective: The Python problem's ``objective``.
        decode: Turns a JSON solution back into what ``objective`` takes, e.g. ``tuple``.
        rel_tol: Relative tolerance of the comparison.

    Returns:
        bool: True when every run's objective matches.
    """
    data = json.loads(Path(rust_json).read_text())
    ok = True
    for i, run in enumerate(data["runs"]):
        expected = objective(decode(run["solution"]))
        got = run["best_objective"]
        match = math.isclose(expected, got, rel_tol=rel_tol, abs_tol=1e-9)
        ok &= match
        print(f"run {i}: rust={got} python-objective={expected} {'ok' if match else 'MISMATCH'}")
    return ok


def _same(a: Any, b: Any, rel_tol: float) -> bool:
    if isinstance(a, float) or isinstance(b, float):
        return math.isclose(a, b, rel_tol=rel_tol, abs_tol=1e-9)
    return a == b


def compare_exact(py: dict, rs: dict, rel_tol: float) -> bool:
    """Every run must agree on every deterministic field."""
    if len(py["runs"]) != len(rs["runs"]):
        print(f"run count differs: python={len(py['runs'])} rust={len(rs['runs'])}")
        return False
    ok = True
    for i, (p, r) in enumerate(zip(py["runs"], rs["runs"])):
        diffs = [f for f in EXACT_FIELDS if not _same(p[f], r[f], rel_tol)]
        ok &= not diffs
        if diffs:
            detail = ", ".join(f"{f}: python={p[f]!r} rust={r[f]!r}" for f in diffs)
            print(f"run {i}: MISMATCH {detail}")
        else:
            print(f"run {i}: ok (best_objective={r['best_objective']})")
    return ok


def compare_stat(py: dict, rs: dict, tolerance: float) -> bool:
    """The Rust mean must not be worse than the Python mean by more than the spread allows."""
    minimize = py["minimize"]
    p_obj = [r["best_objective"] for r in py["runs"]]
    r_obj = [r["best_objective"] for r in rs["runs"]]
    p_avg, r_avg = statistics.fmean(p_obj), statistics.fmean(r_obj)
    spread = max(statistics.pstdev(p_obj), statistics.pstdev(r_obj))
    slack = 2 * spread + tolerance * max(abs(p_avg), 1.0)
    worse_by = (r_avg - p_avg) if minimize else (p_avg - r_avg)
    ok = worse_by <= slack

    rows = [
        ("best", py["best_objective"], rs["best_objective"]),
        ("avg", p_avg, r_avg),
        ("worst", py["worst_objective"], rs["worst_objective"]),
        ("std", statistics.pstdev(p_obj), statistics.pstdev(r_obj)),
        ("time/run [s]", py["avg_total_time_secs"], rs["avg_total_time_secs"]),
    ]
    print(f"{'':14}{'python':>16}{'rust':>16}")
    for name, p, r in rows:
        print(f"{name:14}{p:>16.6g}{r:>16.6g}")
    direction = "minimize" if minimize else "maximize"
    verdict = "ok" if ok else "WORSE"
    print(f"{direction}: rust is worse by {worse_by:.6g} (allowed {slack:.6g}) -> {verdict}")
    return ok


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("python_json")
    parser.add_argument("rust_json")
    parser.add_argument("--mode", choices=("exact", "stat"), required=True)
    parser.add_argument("--rel-tol", type=float, default=1e-9, help="exact: float tolerance")
    parser.add_argument(
        "--tolerance", type=float, default=0.01, help="stat: relative slack on the mean"
    )
    args = parser.parse_args(argv)

    py = json.loads(Path(args.python_json).read_text())
    rs = json.loads(Path(args.rust_json).read_text())
    if py["minimize"] != rs["minimize"]:
        print(f"direction differs: python minimize={py['minimize']} rust={rs['minimize']}")
        return 1
    ok = (
        compare_exact(py, rs, args.rel_tol)
        if args.mode == "exact"
        else compare_stat(py, rs, args.tolerance)
    )
    p_time, r_time = py["avg_total_time_secs"], rs["avg_total_time_secs"]
    if p_time > 0 and r_time > 0:
        print(
            f"time/run: python {p_time:.4g}s, rust {r_time:.4g}s ({p_time / r_time:.2f}x speedup)"
        )
    print("PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

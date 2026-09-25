# Problems written in Python

The generic heuristics also run on a problem you define in Python. `LocalSearch`,
`SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `TabuSearch`,
`LateAcceptanceHillClimbing`, `RandomWalk`, `BeamSearch`, `PopulationAnnealing` and
`VariableNeighborhoodSearch` all accept one. The search loop stays in Rust and calls back into your objects for everything that
depends on the problem.

## The protocol

There is no base class to inherit. Any object with these members works.

| Problem member | Meaning |
|---|---|
| `minimize` | `True` to minimize `objective`, `False` to maximize it |
| `new_solution(rng)` | a random starting solution |
| `objective(solution)` | the solution's value, a float |
| `neighborhoods` | a mapping from a name to a neighborhood object |

A heuristic picks a neighborhood by name through its `neighbor` argument. A problem can declare up
to eight.

| Neighborhood member | Meaning |
|---|---|
| `neighbors(problem, solution)` | an iterable of moves |
| `apply(problem, solution, move)` | the solution after the move, as a new object |
| `delta(problem, solution, move)` | optional, `objective` after the move minus before |
| `random_neighbor(problem, solution, rng)` | optional, one move, or `None` when there is none |
| `tabu_keys(problem, move)` | optional, what the move makes tabu, needed by `TabuSearch` |

Solutions and moves can be any Python objects. `apply` must not change the solution it is given,
because the search keeps references to earlier solutions. Tuples and other immutable values make
that automatic.

`rng` is a `random.Random` seeded from the run's seed. Draw from it, not from the global `random`
module, and a run with a `seed` reproduces exactly.

`tabu_keys` returns an int, a tuple of ints, or a list of those. A move is tabu while any of its
keys is.

## Example

A traveling salesperson problem with a 2-opt neighborhood.

```python
import math
import optopus


class TwoOpt:
    def neighbors(self, problem, tour):
        n = len(tour)
        return [(i, j) for i in range(1, n - 1) for j in range(i + 1, n)]

    def random_neighbor(self, problem, tour, rng):
        return tuple(sorted(rng.sample(range(1, len(tour)), 2)))

    def apply(self, problem, tour, move):
        i, j = move
        return tour[:i] + tour[i : j + 1][::-1] + tour[j + 1 :]

    def delta(self, problem, tour, move):
        i, j = move
        d = problem.dist
        a, b, c, e = tour[i - 1], tour[i], tour[j], tour[(j + 1) % len(tour)]
        return d(a, c) + d(b, e) - d(a, b) - d(c, e)

    def tabu_keys(self, problem, move):
        return move


class Tsp:
    minimize = True

    def __init__(self, points):
        self.points = points
        self.neighborhoods = {"TwoOpt": TwoOpt()}

    def dist(self, a, b):
        return math.dist(self.points[a], self.points[b])

    def new_solution(self, rng):
        rest = list(range(1, len(self.points)))
        rng.shuffle(rest)
        return (0, *rest)

    def objective(self, tour):
        return sum(self.dist(tour[i - 1], tour[i]) for i in range(len(tour)))


points = [(math.cos(k), math.sin(3 * k)) for k in range(30)]
sa = optopus.SimulatedAnnealing(
    neighbor="TwoOpt",
    initial_temperature=0.1,
    cooling_rate=0.9999,
    stop=optopus.StopCondition(max_iteration=50_000),
)
report = sa.run(Tsp(points), runs=5, seed=42)
print(report.best_objective, report.runs[0].solution)
```

## Speed

Every step calls into Python, so a step costs about as much as the Python code it runs. The
binding itself adds well under a microsecond.

Two optional methods decide most of that cost.

- Without `delta`, each move is priced by calling `apply` and then `objective`. Local search and
  tabu search price the whole neighborhood every step.
- Without `random_neighbor`, simulated annealing, late acceptance, random walk and population
  annealing build the full list from `neighbors` every step just to pick one move from it.

## Errors

The problem is checked against the protocol when `run` starts. A missing member raises
`TypeError`, and an unknown `neighbor` or a `TabuSearch` over a neighborhood without `tabu_keys`
raises `ValueError`.

An exception raised inside one of your methods stops the search at the next step, and `run`
raises that same exception. The remaining runs are skipped. Ctrl-C interrupts a run the same way.

Problem-specific heuristics such as `WalkSat` or `BreakoutLocalSearch` only work on their own
problem type and raise `ValueError` on a Python problem.

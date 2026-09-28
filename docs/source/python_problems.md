# Problems written in Python

The generic heuristics also run on a problem you define in Python. `LocalSearch`,
`SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `TabuSearch`,
`LateAcceptanceHillClimbing`, `RandomWalk`, `BeamSearch`, `PopulationAnnealing` and
`ReinforcementLearningSearch` accept one, and so do the composed heuristics
(`VariableNeighborhoodSearch`, `Sequential`, `Iterated`, `Restart`, `GeneticAlgorithm`,
`BreakoutLocalSearch.from_parts`). With the ruin methods below, so does
`AdaptiveLargeNeighborhoodSearch`. The search loop stays in Rust and calls back into your objects for everything that
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

`tabu_keys` returns one key or a list of keys, and a move is tabu while any of its keys is. A
key is a non-negative int, or a tuple of two or three non-negative ints; anything else raises
`TypeError`. An int may be as large as you like, `item * 10**15 + bin` included, though a
tuple, `(item, bin)`, says the same thing more plainly. An int and a tuple never collide, even
when the numbers match. An empty list makes the move never tabu.
Keys of different neighborhoods share one memory, so a flip and a swap keyed on the same
variable index forbid each other.

`random_neighbor` returning `None` means the solution has no move in that neighborhood.
`SimulatedAnnealing`, `BangBangSimulatedAnnealing`, `LateAcceptanceHillClimbing` and
`PopulationAnnealing` end the run there, and `RandomWalk` counts the step as rejected and tries
again. Return `None` only when there truly is no move; a neighborhood that is empty only
sometimes, such as a swap between bins when every item shares one, is better merged with one
that always has a move.

`RunResult.solution` is the solution object your methods produced, returned as it is.

## What some heuristics need besides

| Heuristic | Problem member |
|---|---|
| `GeneticAlgorithm` | `crossover(a, b, rng)`, the child of two solutions |
| `GeneticAlgorithm`, `BreakoutLocalSearch.from_parts` | optional `distance(a, b)`, a non-negative int. Without it, `a == b` counts as 0 and anything else as 1 |
| `AdaptiveLargeNeighborhoodSearch` | the ruin methods |

A Python problem has exactly one crossover, so `GeneticAlgorithm` leaves `crossover` and
`sub_heuristic` unset.

### Ruin methods

Ruin and recreate takes elements out of a solution and puts them back one by one. The elements are
ints, and they sit in containers (a route, a bin, the one tour) at numbered places. The working copy
it edits, the partial, is any Python object your methods agree on, typically lists.

| Problem member | Meaning |
|---|---|
| `to_partial(solution)` | a fresh mutable copy of the solution, which the others edit in place |
| `finish(partial)` | the solution the partial describes |
| `elements(partial)` | the placed elements |
| `remove_all(partial, elements)` | take these out |
| `removal_gain(partial, element)` | what taking it out saves |
| `relatedness(a, b)` | how alike two elements are, smaller is more alike |
| `num_buckets(partial)` | how many containers an element may go into |
| `num_places(partial, bucket)` | how many positions that container offers |
| `insertion_cost(partial, bucket, place, element)` | what putting it there costs |
| `insert(partial, bucket, place, element)` | put it there |
| `partial_objective(partial)` | optional, the objective without calling `finish` |
| `repair_around(partial, anchors, rng)` | optional, a local search around the elements just put back |

`to_partial` must copy. A partial that shares its lists with the solution would change the
solution behind the search's back. Fold capacity violations into `insertion_cost` with a penalty,
so that some place is always available.

The costs are read as costs whatever `minimize` says, so a maximizing problem states them in
terms of what it loses:

- `removal_gain` is how much better the partial gets by taking the element out, positive when
  removing it helps. Worst removal takes the elements with the largest gain first.
- `insertion_cost` is how much worse the partial gets by putting the element there. Lower is
  better, and repair puts each element where it is lowest.
- `relatedness` returns a number, ints included; Shaw removal takes elements that are alike
  together.
- `num_buckets` counts the containers an element may go into now. A problem that opens
  containers on demand, like bins, includes one empty container here, so repair can always open
  a new one.
- `num_places` is `1` for a container whose order does not matter (a bin, a color class),
  `len + 1` for a sequence (a route), and `len` for a cyclic one (a tour).
- `partial_objective` is in the problem's own direction, like `objective`. The search calls
  `finish` only for a candidate it accepts and reads `partial_objective` for the others, so it
  saves time only once most candidates are rejected.
- `repair_around` receives `anchors` as a list of the elements just put back, and edits the
  partial in place.

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

With both, a step still calls `apply` and `objective` once for every move it accepts, since the
new current solution needs its own value. An expensive `objective` shows up in proportion to
the acceptance rate.

## Constraints and flat objectives

The generic heuristics accept any solution `new_solution` and `apply` return, so a hard
constraint is either kept by construction, with moves that cannot break it, or penalized in
`objective`. A penalty needs a weight larger than the most a move can gain by breaking the
constraint, as [Choosing a penalty weight](formula.md#choosing-a-penalty-weight) explains
for `Formula`; the built-in `VertexCover` and `GraphColoring` use `num_vertices + 1` for the
same reason.

An objective that counts something, such as bins used, is flat: most moves leave the count
unchanged, and the search cannot tell a move toward emptying a bin from any other. Add a
secondary term, smaller than one unit of the count, that rewards progress, such as the sum of
squared bin loads. A neighborhood that cannot change the count, such as a swap of two items
between bins, has to be paired with one that can.

## Errors

The problem is checked against the protocol when `run` starts. A missing member raises
`TypeError`, and an unknown `neighbor` or a `TabuSearch` over a neighborhood without `tabu_keys`
raises `ValueError`.

An exception raised inside one of your methods stops the search at the next step, and `run`
raises that same exception. The remaining runs are skipped. Ctrl-C interrupts a run the same way.

Problem-specific heuristics such as `WalkSat` or the MaxCut form of `BreakoutLocalSearch` only
work on their own problem type and raise `ValueError` on a Python problem. So does a heuristic
whose member is missing, for example `GeneticAlgorithm` without `crossover` or
`AdaptiveLargeNeighborhoodSearch` without the ruin methods, which names what is missing.

# Exact search

Every other heuristic returns the best solution it found, and none can say whether a better one
exists. `BranchAndBound` can, on `MaxCut`, `Qubo`, `Sat`, `VertexCover`, `Formula` and
[problems written in Python](#on-a-problem-written-in-python) with the branching methods. It splits
the problem into nodes, each with some variables' ranges narrowed, searches each node with
another heuristic, and drops a node once a bound, the *relaxation*, shows that nothing in it
beats the best solution so far. When no node is left, the best solution is optimal.

## BranchAndBound

Every iteration takes the open node whose bound is best. When that bound cannot beat the best
solution so far, the node is dropped. Otherwise `heuristic` searches it, starting from the best
solution, and the node splits one variable's range in two at that solution's value.

| Argument | Default | Meaning |
|---|---|---|
| `heuristic` | | the heuristic searching each node; its `stop` is a budget per node |
| `stop` | `None` | `StopCondition` for the whole search; `None` runs until no node is left |
| `relaxation` | `None` | the bound nodes are pruned by: a name from the table below, a function, or `None` for the problem's default |

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
bnb = optopus.BranchAndBound(
    heuristic=optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=100)),
    stop=optopus.StopCondition(max_duration_secs=10.0),
    relaxation="Eigenvalue",
)
run = bnb.run(mc, seed=42).runs[0]
print(run.best_objective, run.proven_optimal)  # 93.0 True
```

Two fields of the `RunResult` say what the search proved; on every other heuristic they are
`None`:

- `proven_optimal` is `True` when the search ran out of nodes before its `stop`.
- `dual_bound` is the best objective a solution not yet ruled out could reach. It equals
  `best_objective` once the optimum is proven, and the gap between the two bounds how far a
  search stopped by `stop` can be from the optimum. It is infinite when the search stopped
  before the whole problem had been split once.

`heuristic` searches each node, so a short `LocalSearch` or `TabuSearch` is the usual choice. It
decides how soon good solutions turn up, and the relaxation how many nodes are left to search.
A binary problem folds the variables a node fixes into a smaller instance of itself, so its own
heuristics, `WalkSat` and `BreakoutLocalSearch` included, search a node as they search the whole
problem. A `Formula` is searched over the narrowed ranges. `stop` defaults to no limit, which on
a large instance means a run that never ends; give it a time limit there.

## Relaxations

The relaxation decides how many nodes are searched. Each problem has a default:

| `relaxation` | Problems | Bound |
|---|---|---|
| `"Binary"` (default) | `MaxCut`, `Qubo`, `Sat`, `VertexCover` | the fixed variables folded away and a simple bound of what is left, such as the sum of the positive weights on `MaxCut` |
| `"Eigenvalue"` | `MaxCut` | the eigenvalue bound of Delorme and Poljak, much tighter; its cost per node grows with the cube of the free vertices |
| `"Interval"` (default) | `Formula` | every term bounded by interval arithmetic over the ranges, valid on every formula and weak on most |

On the 30-vertex graph above, `"Binary"` proves the same optimum with about 2,500 times as many
iterations as `"Eigenvalue"`. The built-in bounds suit instances of a few dozen variables. A
name the problem does not support raises `ValueError` when `run` is called.

`relaxation=optopus.EigenvalueRelaxation(max_iterations=100, lanczos_steps=30,
certify_limit=3000)` is `"Eigenvalue"` with its limits set: the subgradient steps and Lanczos
steps spent on each bound, and the largest node, in vertices, it certifies before falling back to
the sum of the positive weights. More steps tighten the bound and cost more per node.

## A bound of your own

A bound that knows the problem is written as a function. It takes the node's ranges, one
inclusive `(lower, upper)` pair per variable, and returns a number no assignment within them
beats: at least the best objective on a maximized problem, at most it on a minimized one. On a
binary problem a range is `(0, 1)` while the variable is free and `(0, 0)` or `(1, 1)` once it
is fixed. The objective is the one the search ranks by, penalty included and, on a `Formula`,
not negated.

On a knapsack, the items held at 1 plus the free ones filling the room left, best value per
weight first and the last one in part, is such a bound:

```python
values = [6, 5, 4, 2]
weights = [4, 3, 2, 1]
budget = 5

knapsack = optopus.Formula(
    n_vars=len(values),
    objective=[([i], float(v)) for i, v in enumerate(values)],
    direction="Maximize",
    constraints=[
        ([([i], float(w)) for i, w in enumerate(weights)], "Le", [([], float(budget))], 7.0),
    ],
)


def fractional_fill(ranges):
    taken = [i for i, (lower, _) in enumerate(ranges) if lower == 1]
    free = [i for i, (lower, upper) in enumerate(ranges) if lower < upper]
    bound = float(sum(values[i] for i in taken))
    room = max(budget - sum(weights[i] for i in taken), 0)
    for i in sorted(free, key=lambda i: values[i] / weights[i], reverse=True):
        part = min(1.0, room / weights[i])
        bound += part * values[i]
        room -= part * weights[i]
    return bound


bnb = optopus.BranchAndBound(
    heuristic=optopus.LocalSearch("Change", stop=optopus.StopCondition(max_iteration=100)),
    relaxation=fractional_fill,
)
run = bnb.run(knapsack, seed=42).runs[0]
print(run.best_objective, run.solution, run.proven_optimal)  # 9.0 [0, 1, 1, 0] True
```

[Modeling with Formula](formula.md#knapsack) explains the model and its penalty weight of 7.

A function that is not a bound prunes the optimum without an error, and the run then reports
`proven_optimal=True` for a solution that is not optimal. Check a new bound against enumeration
on instances small enough to enumerate. A function that raises stops the run, and `run` raises
the exception; one that returns NaN raises `ValueError`.

## On a problem written in Python

A [problem written in Python](python_problems.md) branches once its solutions assign an integer
to each variable and it can narrow those ranges, through four methods:

| Problem member | Returns |
|---|---|
| `variable_bounds()` | the inclusive `(lower, upper)` range of each variable |
| `variable_values(solution)` | the value of each variable in `solution` |
| `solution_from_values(values)` | the solution assigning `values` |
| `restricted(bounds)` | the same problem with each variable's range narrowed to `bounds`, declaring the same neighborhoods in the same order |

A node is the problem `restricted` returns, so the neighborhoods have to keep to the problem's
own ranges, and `heuristic` searches the node with them. The relaxation is a function written in
Python, since optopus knows no bound for your objective, and the objective must not depend on
the ranges. A node never calls `new_solution`; its moves draw from the problem's `rng`.

```python
weights = [4, 3, 2, 1]


class Flip:
    def neighbors(self, problem, x):
        return [i for i, (lower, upper) in enumerate(problem.bounds) if lower < upper]

    def apply(self, problem, x, i):
        return (*x[:i], 1 - x[i], *x[i + 1 :])


class Knapsack:
    minimize = False

    def __init__(self, bounds=((0, 1),) * 4):
        self.bounds = list(bounds)
        self.neighborhoods = {"Flip": Flip()}

    def new_solution(self, rng):
        return tuple(rng.randint(lower, upper) for lower, upper in self.bounds)

    def objective(self, x):
        over = max(sum(w * xi for w, xi in zip(weights, x)) - budget, 0)
        return sum(v * xi for v, xi in zip(values, x)) - 7.0 * over

    def variable_bounds(self):
        return self.bounds

    def variable_values(self, x):
        return list(x)

    def solution_from_values(self, values):
        return tuple(values)

    def restricted(self, bounds):
        return Knapsack(bounds)


bnb = optopus.BranchAndBound(
    heuristic=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    relaxation=fractional_fill,
)
run = bnb.run(Knapsack(), seed=42).runs[0]
print(run.best_objective, run.solution, run.proven_optimal)  # 9.0 (0, 1, 1, 0) True
```

A problem without one of the four methods, or `relaxation` left at a name, raises `ValueError`.

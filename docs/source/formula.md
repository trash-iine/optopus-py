# Modeling with Formula

`Formula` states a problem as polynomials over bounded integer variables: an objective to
maximize or minimize, and constraints on other polynomials. Nothing else needs to be written.
The search prices every move from the polynomial terms the move touches, so a `Formula` runs
nearly as fast as a built-in problem.

Use it when every decision is an integer in a known range and the objective and every
constraint are sums of products of those integers. A knapsack, an assignment with "exactly one"
rules, or a QUBO with side constraints fits. A structured solution such as a route, or an
objective that is not a polynomial (a maximum, a lookup table, a simulation), does not; write a
[problem in Python](python_problems.md) for that. [Built-in problems](problems.md) compares the
options.

## Variables

`n_vars` variables are numbered `0` to `n_vars - 1`. Without `bounds` every variable is binary,
taking 0 or 1. `bounds` gives each variable an inclusive `(lower, upper)` range instead:

```python
bounds=[(0, 1), (0, 10), (-3, 3)]   # a binary, a count from 0 to 10, a signed offset
```

A variable whose range is a single value, `(v, v)`, is fixed at `v`. Fix what the task rules out
this way rather than with a constraint: the search never visits the excluded values, and no
penalty weight needs choosing.

The repr's `binary` field says whether every variable ranges over exactly `(0, 1)`. A formula
whose variables are all binary takes a faster path, and fixing one variable at `(0, 0)` leaves
it, which ran about 1.5 times slower per move on a 56-variable model. When that matters on a large binary model,
substitute the fixed value into the polynomials instead and drop the variable's terms.

## Polynomials

The objective and both sides of every constraint are polynomials, written as a list of terms.
Each term is `(variable_indices, coefficient)`, the product of those variables times the
coefficient:

| Term | Means |
|---|---|
| `([], 3.0)` | the constant 3 |
| `([2], 1.5)` | 1.5 · x₂ |
| `([0, 1], -2.0)` | −2 · x₀ · x₁, which is an AND on binary variables |
| `([4, 4], 1.0)` | x₄², since a repeated index is a power |

Terms add up. So `[([0], 1.0), ([1], 1.0), ([0, 1], -2.0)]` is x₀ + x₁ − 2x₀x₁, which on binary
variables is x₀ XOR x₁. A term that names a variable outside `[0, n_vars)` raises
`ValueError`.

## Objective and direction

`direction` is `"Maximize"` (the default) or `"Minimize"`. The search never optimizes the
objective alone; it optimizes the objective with the constraint penalties folded in:

```text
Maximize:  objective(x) − Σ penalty(x)
Minimize:  objective(x) + Σ penalty(x)
```

An empty objective, `objective=[]`, is allowed and is 0 everywhere, which asks only for a
solution that satisfies the constraints.

## Constraints

Constraints are penalized, not enforced. The search may pass through a solution that breaks
one, and it pays `penalty_weight × violation` while it does. Each constraint is a 4-tuple:

```text
(lhs, relation, rhs, penalty_weight)        # lhs and rhs are polynomials
(expr, "Clamp", (lo, hi), penalty_weight)   # keeps expr within [lo, hi]
```

Each constraint and the `Clamp` range must be tuples; a list in their place raises
`TypeError`. A negative or non-finite `penalty_weight`, or a `Clamp` range with `lo` above
`hi`, raises `ValueError`. The relation is one of the names below, or its symbol: `"<"`, `"<="`, `"=="` (or
`"="`), `">="`, `">"`.

With `d = lhs − rhs`, the violation is:

| Relation | Holds when | Violation |
|---|---|---|
| `"Le"` | `lhs ≤ rhs` | `max(d, 0)` |
| `"Ge"` | `lhs ≥ rhs` | `max(−d, 0)` |
| `"Eq"` | `lhs = rhs` | `abs(d)` |
| `"Lt"` | `lhs < rhs` | `max(d + 1e-9, 0)` |
| `"Gt"` | `lhs > rhs` | `max(−d + 1e-9, 0)` |
| `"Clamp"` | `lo ≤ expr ≤ hi` | `max(lo − expr, 0) + max(expr − hi, 0)` |

`"Lt"` and `"Gt"` charge a margin of 1e-9 at equality, so a tie is never free, but the margin
is far too small to steer the search. On integer data, write `lhs ≤ rhs − 1` with `"Le"`
instead of `lhs < rhs`.

The violation is linear in how far the constraint is off, so breaking a budget by 3 costs three
times what breaking it by 1 does.

### Choosing a penalty weight

A hard constraint needs a weight large enough that breaking it never pays. Look at one move
of the search. The weight must exceed the most that move can gain on the objective, divided by
the least violation it can cause:

```text
penalty_weight  >  (largest objective gain of one move) / (smallest violation of one move)
```

The rule is about a move out of a feasible solution, which can only add violations; count each
constraint the move breaks by its own violation. Between infeasible solutions a move may trade
one violation for another at no net cost, and that is how the search walks back to
feasibility.

In a knapsack with integer weights, adding one item gains at most the largest item value, and
an item that overfills the budget overfills it by at least 1, so any weight above the largest
value works. Write that reasoning next to the weight in your code, because the weight is only
correct for data where the reasoning holds.

A much larger weight than needed also works, but it flattens the rest of the landscape: every
infeasible neighbor looks equally hopeless and annealing temperatures become hard to set. A soft
constraint takes whatever it costs in the task's own units.

### Equality constraints and the moves

An `"Eq"` constraint such as "exactly one of these is 1" has no feasible neighbor under a
single `"Change"`: turning one variable off breaks it until another is turned on. The search
must cross an infeasible solution to get from one feasible solution to the next, and the penalty
is exactly what makes it refuse. Two things help:

- `TabuSearch`, which takes the best allowed move even when it is worse, so it walks through
  the infeasible step.
- The `"Swap"` neighborhood, which exchanges the values of two variables and so keeps a sum
  over them fixed. It never changes how many variables hold each value, so it cannot repair a
  count the random starting solution got wrong: run it after `"Change"`, through `Sequential`
  or `VariableNeighborhoodSearch`, not on its own.

`SimulatedAnnealing` can cross those infeasible steps too, but only with a temperature near the
penalty weight, see [SimulatedAnnealing](heuristics.md#simulatedannealing).

When neither fits, write the problem in Python so every move keeps the constraint by
construction.

### Several hard constraints

The weight rule compares a constraint with the objective. Hard constraints that pull against
each other, such as "exactly one nurse per shift" against "at most four shifts per nurse", need
nothing more: once every weight passes the rule above, no move gains by breaking a constraint,
whatever the weights are relative to each other. Their ratio only shapes how the search moves
between infeasible solutions, so keep them in the same range rather than ranking them by
importance.

## What the search reports

`RunResult.solution` is a `list[int]`, one value per variable. Binary variables report 0 and 1,
which compare equal to `False` and `True`.

`best_objective`, `initial_objective` and every other objective the report holds are the
penalized value, turned so that **higher is always better**:

| Direction | Reported value |
|---|---|
| `"Maximize"` | `objective − penalty` |
| `"Minimize"` | `−(objective + penalty)` |

A minimized objective therefore comes out negated, and `RunReport.best_objective` is the
maximum over the runs whichever the direction. Read the parts back with the two evaluators,
which take a solution and return plain values:

- `eval_objective(values)` gives the objective before any penalty.
- `eval_penalty(values)` gives the summed penalty. Zero means every constraint holds.

Check hard constraints from `eval_penalty`, or better from the decoded solution in the task's
own terms, and not from the sign of `best_objective`.

A search cannot prove that no feasible solution exists. When `eval_penalty` stays above zero
across heuristics and seeds, suspect constraints that contradict each other before the search:
check the data against them, as a count of what is available against what is required. Which
constraint the search leaves broken depends on the weights, so it does not say which one is at
fault.

## Neighborhoods

| `neighbor` | Move |
|---|---|
| `"Change"` (alias `"Flip"`) | sets one variable to another value in its range; a flip on a binary variable |
| `"Swap"` | exchanges the values of two variables whose values differ and fit each other's range |
| `"Reverse"` | reverses the values of a range of consecutive variables |

`"Swap"` and `"Reverse"` only rearrange values, so they keep how many variables hold each value.
Pair them with `"Change"` unless the counts are right by construction. They are also pairwise:
a `LocalSearch` or `TabuSearch` step scores about n²/2 moves for n variables, against the
n · (range) of `"Change"`. On a 56-variable binary model a `TabuSearch` step with `"Swap"` took
about 100 times as long as one with `"Change"`, so give a `"Swap"` step inside `Sequential` a
small budget of its own.

`"Change"` reads its price from a table every solution keeps, one entry per value each variable
could take, so a solution costs memory in proportion to the sum of `upper − lower` over the
variables. A variable ranging over millions of values is better written as a problem in Python.
`"Reverse"` evaluates a copy of the solution, which makes it the slowest.

`GeneticAlgorithm` runs on a `Formula` with `crossover="Uniform"` (the default), which takes each
variable from either parent, or `"SubProblem"`, which solves the variables the parents disagree
on as a smaller `Formula`.

## Examples

### Knapsack

A knapsack: choose items to maximize their total value within a weight budget.

```python
import optopus

values = [6, 5, 4, 2]
weights = [4, 3, 2, 1]
budget = 5

# Adding one item gains at most max(values) = 6, and an item that overfills the budget
# overfills it by at least 1 (integer weights), so any weight above 6 makes it a loss.
penalty = 7.0

knapsack = optopus.Formula(
    n_vars=len(values),
    objective=[([i], float(v)) for i, v in enumerate(values)],
    direction="Maximize",
    constraints=[
        ([([i], float(w)) for i, w in enumerate(weights)], "Le", [([], float(budget))], penalty),
    ],
)

ts = optopus.TabuSearch(
    neighbor="Change", tabu_tenure=(1, 3), stop=optopus.StopCondition(max_iteration=1_000)
)
report = ts.run(knapsack, runs=4, seed=42)

best = max(report.runs, key=lambda r: r.best_objective)
print(report.best_objective)                   # 9.0
print(best.solution)                           # [0, 1, 1, 0] — items 1 and 2
print(knapsack.eval_objective(best.solution))  # 9.0 — the value, before penalties
print(knapsack.eval_penalty(best.solution))    # 0.0 — within the budget
```

The same problem as a minimization, of the value left behind, reports the negated score:

```python
left_behind = optopus.Formula(
    n_vars=len(values),
    objective=[([], float(sum(values)))] + [([i], -float(v)) for i, v in enumerate(values)],
    direction="Minimize",
    constraints=[
        ([([i], float(w)) for i, w in enumerate(weights)], "Le", [([], float(budget))], penalty),
    ],
)
report = ts.run(left_behind, runs=4, seed=42)
print(report.best_objective)  # -8.0 — 8 left behind, negated so that higher is better
```

### Assignment

A decision "who does what" is one binary variable per pair, a one-hot encoding: variable
`x(w, s)` is 1 when worker `w` takes shift `s`. "Exactly one worker per shift" is then a
linear `"Eq"`, a limit per worker a `"Le"` or `"Clamp"`, and a pair the task rules out is fixed
at 0 through `bounds`. One integer per shift holding the worker's number reads more naturally
but is not polynomial to constrain: "worker 2 works at most two shifts" has no linear form in
those integers.

```python
import optopus

workers = ["Ann", "Ben", "Cal"]
shifts = ["Mon", "Tue", "Wed", "Thu"]
cost = [  # cost[w][s]: what worker w charges for shift s
    [4, 2, 5, 3],
    [3, 4, 2, 5],
    [5, 3, 4, 2],
]
unavailable = {("Cal", "Tue")}

# One binary variable per (worker, shift): 1 when that worker takes that shift.
def x(w, s):
    return w * len(shifts) + s

n_vars = len(workers) * len(shifts)
bounds = [(0, 1)] * n_vars
for w, s in unavailable:
    bounds[x(workers.index(w), shifts.index(s))] = (0, 0)  # ruled out, never searched

# A shift left empty or doubled saves at most max(cost) = 5, and breaks an "Eq" by 1.
weight = 6.0
constraints = []
for s in range(len(shifts)):  # every shift has exactly one worker
    constraints.append(([([x(w, s)], 1.0) for w in range(len(workers))], "Eq", [([], 1.0)], weight))
for w in range(len(workers)):  # nobody works more than two shifts
    constraints.append(([([x(w, s)], 1.0) for s in range(len(shifts))], "Le", [([], 2.0)], weight))

roster = optopus.Formula(
    n_vars=n_vars,
    objective=[([x(w, s)], float(cost[w][s])) for w in range(len(workers)) for s in range(len(shifts))],
    direction="Minimize",
    constraints=constraints,
    bounds=bounds,
)
ts = optopus.TabuSearch(
    neighbor="Change", tabu_tenure=(2, 4), stop=optopus.StopCondition(max_failed_update=500)
)
report = ts.run(roster, runs=4, seed=1)
best = max(report.runs, key=lambda r: r.best_objective)  # higher is better on a Formula

print({shifts[s]: workers[w] for s in range(len(shifts)) for w in range(len(workers))
       if best.solution[x(w, s)]})  # {'Mon': 'Ben', 'Tue': 'Ann', 'Wed': 'Ben', 'Thu': 'Cal'}
print(roster.eval_objective(best.solution), roster.eval_penalty(best.solution))  # 9.0 0.0
```

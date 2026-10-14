---
source_sha256: f1a3b5b9fa83
---

# Formula によるモデル化

`Formula` は、有界な整数変数上の多項式で問題を表します。最大化または最小化する目的関数と、別の
多項式に対する制約です。それ以外に書くものはありません。探索は各移動を、その移動が触れる多項式の
項から評価するので、`Formula` は組み込みの問題とほぼ同じ速さで動きます。

すべての決定が既知の範囲の整数で、目的関数とすべての制約がそれらの整数の積の和で書けるときに
使ってください。ナップサック、「ちょうど 1 つ」の規則を持つ割当て、追加制約付きの QUBO などが
当てはまります。経路のような構造を持つ解や、多項式でない目的関数 (最大値、参照表、シミュレーション)
は当てはまりません。その場合は [Python で問題を書いて](python_problems.md)ください。選択肢の比較は
[組み込みの問題](problems.md)にあります。

(variables)=
## 変数

`n_vars` 個の変数に `0` から `n_vars - 1` の番号が付きます。`bounds` を与えなければ、どの変数も
0 か 1 を取る 2 値変数です。`bounds` を与えると、各変数は両端を含む `(lower, upper)` の範囲を取り
ます。

```python
bounds=[(0, 1), (0, 10), (-3, 3)]   # 2 値、0 から 10 の個数、符号付きのずれ
```

範囲が 1 つの値だけの変数 `(v, v)` は `v` に固定されます。タスクが除外する値は、制約ではなくこの
方法で固定してください。探索は除外した値を訪れず、ペナルティの重みを選ぶ必要もありません。

repr の `binary` フィールドは、すべての変数の範囲がちょうど `(0, 1)` かどうかを示します。変数が
すべて 2 値の式は高速な経路を通りますが、1 つの変数を `(0, 0)` に固定するとその経路から外れ、
56 変数のモデルでは移動 1 回あたり約 1.5 倍遅くなりました。大きな 2 値モデルでそれが問題になる
場合は、固定した値を多項式に代入し、その変数の項を取り除いてください。

(polynomials)=
## 多項式

目的関数と、各制約の両辺は多項式で、項のリストとして書きます。各項は
`(variable_indices, coefficient)` で、それらの変数の積に係数を掛けたものです。

| 項 | 意味 |
|---|---|
| `([], 3.0)` | 定数 3 |
| `([2], 1.5)` | 1.5 · x₂ |
| `([0, 1], -2.0)` | −2 · x₀ · x₁。2 値変数では AND |
| `([4, 4], 1.0)` | x₄²。同じ番号の繰り返しはべき乗 |

項は足し合わされます。したがって `[([0], 1.0), ([1], 1.0), ([0, 1], -2.0)]` は x₀ + x₁ − 2x₀x₁ で、
2 値変数では x₀ XOR x₁ です。`[0, n_vars)` の外の変数を含む項は `ValueError` になります。

(objective-and-direction)=
## 目的関数と向き

`direction` は `"Maximize"` (既定) か `"Minimize"` です。探索は目的関数だけを最適化することは
なく、制約のペナルティを組み込んだ目的関数を最適化します。

```text
Maximize:  objective(x) − Σ penalty(x)
Minimize:  objective(x) + Σ penalty(x)
```

空の目的関数 `objective=[]` も許され、どこでも 0 になります。これは制約を満たす解だけを求めることに
なります。

(constraints)=
## 制約

制約は強制されるのではなく、ペナルティとして課されます。探索は制約を破る解を通ることがあり、その間は
`penalty_weight × violation` を払います。各制約は 4 つ組です。

```text
(lhs, relation, rhs, penalty_weight)        # lhs と rhs は多項式
(expr, "Clamp", (lo, hi), penalty_weight)   # expr を [lo, hi] に収める
```

各制約と `Clamp` の範囲はタプルでなければならず、代わりにリストを渡すと `TypeError` になります。
負または有限でない `penalty_weight` や、`lo` が `hi` より大きい `Clamp` の範囲は `ValueError` に
なります。関係は下の名前のいずれか、またはその記号 `"<"`、`"<="`、`"=="` (または `"="`)、
`">="`、`">"` です。

`d = lhs − rhs` とすると、違反量は次のとおりです。

| 関係 | 成り立つ条件 | 違反量 |
|---|---|---|
| `"Le"` | `lhs ≤ rhs` | `max(d, 0)` |
| `"Ge"` | `lhs ≥ rhs` | `max(−d, 0)` |
| `"Eq"` | `lhs = rhs` | `abs(d)` |
| `"Lt"` | `lhs < rhs` | `max(d + 1e-9, 0)` |
| `"Gt"` | `lhs > rhs` | `max(−d + 1e-9, 0)` |
| `"Clamp"` | `lo ≤ expr ≤ hi` | `max(lo − expr, 0) + max(expr − hi, 0)` |

`"Lt"` と `"Gt"` は等号のときに 1e-9 の余裕を課すので、等号がただになることはありませんが、この
余裕は探索を導くには小さすぎます。整数データでは `lhs < rhs` の代わりに `"Le"` で
`lhs ≤ rhs − 1` と書いてください。

違反量は制約からのずれに比例するので、予算を 3 超えると 1 超えたときの 3 倍のコストがかかります。

(choosing-a-penalty-weight)=
### ペナルティの重みの選び方

厳しい制約には、破っても決して得にならないだけの大きな重みが必要です。探索の 1 回の移動を考えて
ください。重みは、その移動で目的関数が得られる最大の利得を、その移動が引き起こしうる最小の違反量で
割った値を超えなければなりません。

```text
penalty_weight  >  (1 回の移動での目的関数の最大の利得) / (1 回の移動での最小の違反量)
```

この規則は、違反を増やすことしかできない、実行可能解から出る移動についてのものです。移動が破る
制約はそれぞれの違反量で数えてください。実行不可能な解どうしの間では、移動がある違反を別の違反と
差し引きゼロで交換することがあり、探索はそうやって実行可能な領域に戻っていきます。

重みが整数のナップサックでは、品物を 1 つ加えて得られるのは高々最大の品物の価値で、予算を超える
品物は少なくとも 1 だけ超えるので、最大の価値より大きい重みならどれでも機能します。この理由づけは
コード中の重みの横に書いておいてください。重みが正しいのは、この理由づけが成り立つデータに対して
だけだからです。

必要よりずっと大きな重みでも機能しますが、それ以外の地形を平らにしてしまいます。実行不可能な近傍が
どれも同じように絶望的に見え、焼きなまし法の温度も設定しにくくなります。ソフトな制約には、タスク
自身の単位でのコストをそのまま重みにします。

(equality-constraints-and-the-moves)=
### 等式制約と移動

「これらのうちちょうど 1 つが 1」のような `"Eq"` 制約は、1 回の `"Change"` では実行可能な近傍を
持ちません。1 つの変数を 0 にすると、別の変数を 1 にするまで制約が破れるからです。探索はある
実行可能解から次の実行可能解に移るのに実行不可能な解を経由しなければならず、ペナルティはまさにそれを
拒ませるものです。役に立つのは次の 2 つです。

- `TabuSearch`。悪化する場合でも許された最良の移動を選ぶので、実行不可能なステップを通り抜けます。
- `"Swap"` 近傍。2 つの変数の値を入れ替えるので、それらの和が保たれます。各値を取る変数の個数を
  決して変えないので、ランダムな初期解が間違えた個数を直すことはできません。単独ではなく、
  `Sequential` や `VariableNeighborhoodSearch` で `"Change"` のあとに実行してください。

`SimulatedAnnealing` もこうした実行不可能なステップを越えられますが、それはペナルティの重みに近い
温度のときだけです。
[SimulatedAnnealing](https://trash-iine.github.io/optopus-py/heuristics.html#simulatedannealing) (英語)
を参照してください。

どちらも合わない場合は、すべての移動が構成上制約を守るように、問題を Python で書いてください。

(several-hard-constraints)=
### 複数の厳しい制約

重みの規則は、制約を目的関数と比べるものです。「各シフトにちょうど 1 人の看護師」と「各看護師は
高々 4 シフト」のように互いに引っ張り合う厳しい制約があっても、それ以上の対処は要りません。すべての
重みが上の規則を満たせば、重みどうしの大小によらず、制約を破って得をする移動はなくなります。重みの
比は実行不可能な解の間での探索の動き方を変えるだけなので、重要度で順位を付けるよりも同じくらいの
範囲にそろえてください。

(what-the-search-reports)=
## 探索が報告する値

`RunResult.solution` は `list[int]` で、変数ごとに 1 つの値です。2 値変数は 0 と 1 を報告し、
これらは `False` と `True` に等しいと比較されます。

`best_objective`、`initial_objective` をはじめレポートが持つ目的関数値はすべてペナルティ込みの値で、
**常に大きいほうが良い**ように向きをそろえてあります。

| 向き | 報告される値 |
|---|---|
| `"Maximize"` | `objective − penalty` |
| `"Minimize"` | `−(objective + penalty)` |

したがって最小化した目的関数は符号が反転して出てきて、`RunReport.best_objective` は向きによらず
全実行の最大値です。内訳は 2 つの評価関数で読み出せます。どちらも解を受け取って素の値を返します。

- `eval_objective(values)` はペナルティを含まない目的関数値を返します。
- `eval_penalty(values)` はペナルティの合計を返します。0 ならすべての制約が成り立っています。

厳しい制約は `best_objective` の符号ではなく `eval_penalty` で確認してください。さらに良いのは、解を
タスク自身の言葉に戻して確認することです。

探索は実行可能解が存在しないことを証明できません。ヒューリスティックやシードを変えても
`eval_penalty` が 0 より大きいままなら、探索より先に、互いに矛盾する制約を疑ってください。使えるものの
数と必要なものの数を比べるなど、データを制約と突き合わせて確認します。探索がどの制約を破ったまま
残すかは重みで決まるので、それはどの制約に問題があるかを示しません。

(formula-neighborhoods)=
## 近傍

| `neighbor` | 移動 |
|---|---|
| `"Change"` (別名 `"Flip"`) | 1 つの変数を範囲内の別の値にする。2 値変数では反転 |
| `"Swap"` | 値が異なり、互いの範囲に収まる 2 つの変数の値を入れ替える |
| `"Reverse"` | 連続する変数の範囲の値を逆順にする |

`"Swap"` と `"Reverse"` は値を並べ替えるだけなので、各値を取る変数の個数を保ちます。個数が構成上
正しい場合を除き、`"Change"` と組み合わせてください。また、これらは 2 変数の組についての近傍です。
`LocalSearch` や `TabuSearch` の 1 ステップは、n 変数に対して `"Change"` の n · (範囲の幅) に比べ、
約 n²/2 個の移動を評価します。56 変数の 2 値モデルでは、`"Swap"` での `TabuSearch` の 1 ステップは
`"Change"` での 1 ステップの約 100 倍の時間がかかったので、`Sequential` の中の `"Swap"` のステップ
には小さな予算を個別に与えてください。

`"Change"` は、各解が持つ表から移動のコストを読みます。表は各変数が取りうる値ごとに 1 つの要素を
持つので、1 つの解は変数ごとの `upper − lower` の合計に比例したメモリを使います。何百万もの値を取る
変数は、Python で問題を書いたほうが適しています。`"Reverse"` は解のコピーを評価するので、最も低速
です。

`GeneticAlgorithm` は `Formula` 上で、各変数をどちらかの親から受け継ぐ `crossover="Uniform"`
(既定) か、親の値が食い違う変数をより小さな `Formula` として解く `"SubProblem"` で動きます。

(examples)=
## 例

(knapsack)=
### ナップサック

重さの予算の範囲内で、価値の合計が最大になるように品物を選びます。

```python
import optopus

values = [6, 5, 4, 2]
weights = [4, 3, 2, 1]
budget = 5

# 品物を 1 つ加えて得られるのは高々 max(values) = 6 で、予算を超える品物は少なくとも 1 だけ
# 超える (重みが整数) ので、6 より大きい重みならどれでも違反は損になる。
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
print(best.solution)                           # [0, 1, 1, 0] — 品物 1 と 2
print(knapsack.eval_objective(best.solution))  # 9.0 — ペナルティ前の価値
print(knapsack.eval_penalty(best.solution))    # 0.0 — 予算内
```

同じ問題を、残した品物の価値の最小化として書くと、符号を反転したスコアが報告されます。

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
print(report.best_objective)  # -8.0 — 8 を残す。大きいほうが良いように符号を反転
```

(assignment)=
### 割当て

「誰が何をするか」という決定は、組ごとに 1 つの 2 値変数、つまり one-hot 符号化で表します。変数
`x(w, s)` は、作業者 `w` がシフト `s` を担当するとき 1 です。すると「各シフトにちょうど 1 人」は
線形の `"Eq"`、作業者ごとの上限は `"Le"` か `"Clamp"` になり、タスクが除外する組は `bounds` で 0 に
固定します。シフトごとに作業者の番号を持つ整数を 1 つ置くほうが自然に読めますが、それでは制約が
多項式になりません。「作業者 2 は高々 2 シフト」は、それらの整数の線形式では書けないからです。

```python
import optopus

workers = ["Ann", "Ben", "Cal"]
shifts = ["Mon", "Tue", "Wed", "Thu"]
cost = [  # cost[w][s]: 作業者 w がシフト s に請求する額
    [4, 2, 5, 3],
    [3, 4, 2, 5],
    [5, 3, 4, 2],
]
unavailable = {("Cal", "Tue")}

# (作業者, シフト) ごとに 2 値変数を 1 つ: その作業者がそのシフトを担当するとき 1。
def x(w, s):
    return w * len(shifts) + s

n_vars = len(workers) * len(shifts)
bounds = [(0, 1)] * n_vars
for w, s in unavailable:
    bounds[x(workers.index(w), shifts.index(s))] = (0, 0)  # 除外。探索されない

# シフトを空けたり重ねたりして節約できるのは高々 max(cost) = 5 で、"Eq" を 1 だけ破る。
weight = 6.0
constraints = []
for s in range(len(shifts)):  # 各シフトにちょうど 1 人
    constraints.append(([([x(w, s)], 1.0) for w in range(len(workers))], "Eq", [([], 1.0)], weight))
for w in range(len(workers)):  # 誰も 2 シフトより多くは働かない
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
best = max(report.runs, key=lambda r: r.best_objective)  # Formula では大きいほうが良い

print({shifts[s]: workers[w] for s in range(len(shifts)) for w in range(len(workers))
       if best.solution[x(w, s)]})  # {'Mon': 'Ben', 'Tue': 'Ann', 'Wed': 'Ben', 'Thu': 'Cal'}
print(roster.eval_objective(best.solution), roster.eval_penalty(best.solution))  # 9.0 0.0
```

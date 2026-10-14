---
source_sha256: 9b6be1724f64
---

# 合成ヒューリスティック

合成ヒューリスティックは、他のヒューリスティックのインスタンス (*ステップ*) から組み立てます。各
ステップは自分の近傍と、そのステップの 1 回の呼び出しを制限する自分の `StopCondition` を持つので、
1 つの探索の中で高速な降下と大きな揺さぶりや、あるステップだけが知っている移動を組み合わせられます。
ステップは合成ヒューリスティックの現在の解に対して働き、どのステップが見つけた最良解も保持されます。
合成ヒューリスティックは、すべてのステップが合う問題ならどれでも動きます。問題が一致すれば問題専用の
ステップも含められます。

このページの例は、[汎用ヒューリスティック](heuristics_generic.md)と同じ、最適値 93 のインスタンスを
使います。

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
```

| ヒューリスティック | 1 ラウンド | 用途 |
|---|---|---|
| [`Sequential(steps)`](#sequential) | すべてのステップを順に実行する | ランダムな初期解を `LocalSearch` で整えてから `TabuSearch` に渡す |
| [`Iterated(search, perturbation)`](#iterated) | `search`、次に `perturbation` を実行し、摂動した解から続ける | 1 つの探索が行き詰まる谷から抜け出す。最初に試す定番の合成 |
| [`VariableNeighborhoodSearch(search, shakes)`](#variableneighborhoodsearch) | shake `k`、次に `search` を実行する。ラウンド開始時より改善していれば結果を残して shake 0 に戻り、そうでなければ開始時の解に戻して shake `k + 1` に進む | `Vrp` の経路間・経路内の移動のように、届く範囲の異なる近傍を組み合わせる |
| [`Restart(heuristic, restart)`](#restart) | `heuristic` を実行し、`restart` を満たせば新しいランダムな解からやり直す | 各探索がすぐ収束するときに、多くの谷をサンプリングする |
| [`GeneticAlgorithm(population_size, mutation)`](#geneticalgorithm) | 問題の交叉で 2 つの親を組み換え、子を `mutation` で改善する | 多様な集団を保つ |
| [`BreakoutLocalSearch.from_parts(...)`](#breakoutlocalsearchfrom_parts) | 降下のあと、探索が停滞している長さに応じて選び大きさを決めた摂動 | Max Cut 用アルゴリズムの枠組みを別の問題で使う |

ステップの各呼び出しは、合成ヒューリスティックの現在の解からそのステップを新たに始めます。
`SimulatedAnnealing` のステップは `initial_temperature` からやり直し、`LateAcceptanceHillClimbing`
のステップは空の履歴から始めます。引き継がれるのは解だけです。そうしたステップには、実行全体に
合わせた長さではなく、それ自身の短いスケジュールを与えてください。

(how-the-outer-stop-counts)=
## 外側の停止条件の数え方

`Sequential`、`Iterated`、`VariableNeighborhoodSearch`、`Restart` の外側の `stop` は、ラウンドを
数えません。ステップが行う反復はすべてそれに数えられ、確認はステップが戻ったときだけ行われ、途中では
行われません。

- `max_iteration=n` は、ステップの反復を合計して `n` 回以上になった最初のステップの境目で止まります。
  `Iterated(search=LocalSearch(...), ..., stop=StopCondition(max_iteration=20))` は、最初の探索だけで
  20 反復かかれば、その探索のあとで止まります。
- `Sequential` はステップのたびに確認するので、最初のステップの中で上限に達すると他のステップは
  飛ばされます。`Iterated` は `search` と `perturbation` の間で、`VariableNeighborhoodSearch` は
  shake と `search` の間で確認します。
- `max_failed_update=m` は、実行全体の最良解が改善してからの反復を、すべてのステップにわたって
  数えます。どの `LocalSearch` もそうであるように、改善しない反復で終わるステップはそれに加算します。

決まった数のラウンドを実行するには、1 ラウンドの反復数を掛けます。ラウンド全体で何も見つからない
ときに止めるには、1 ラウンドの反復数より大きな `max_failed_update` を使います。時間制限ならこの計算は
まったく要りません。

## Sequential

ステップを 1 つずつ、それぞれ自分の停止条件まで順に実行し、`stop` を満たすまでその並びを繰り返します。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `steps` | | 順に実行するヒューリスティックのインスタンス。空であってはならない |
| `stop` | | 実行全体の `StopCondition`。ステップのたびに確認される |

```python
seq = optopus.Sequential(
    [
        optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
        optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=200)),
    ],
    stop=optopus.StopCondition(max_iteration=1_000),
)
print(seq.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

異なる移動による降下を並べた `Sequential` を、ラウンド全体で何も見つからなくなるまで繰り返すと、
そのすべてで解を仕上げられます。[Vrp](problems.md#vrp) では `VariableNeighborhoodSearch` の中で
それを使っています。

## Iterated

反復局所探索です。`search`、次に `perturbation` を実行し、摂動した解から次のラウンドを始めます。受理の
判定はありません。悪化させた摂動も取り消されず、それを越えて残るのは見つけた最良解だけです。改善した
ラウンドだけを残したいなら、shake が 1 つの `VariableNeighborhoodSearch` を使ってください。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `search` | | 集中化するヒューリスティック。`LocalSearch` など |
| `perturbation` | | 解を局所最適から押し出すヒューリスティック。短い `RandomWalk` など |
| `stop` | | 実行全体の `StopCondition` |

摂動は通常、`max_iteration` がランダムな移動の数になる `RandomWalk` です。

```python
ils = optopus.Iterated(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    perturbation=optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=3)),
    stop=optopus.StopCondition(max_iteration=3_000),
)
print(ils.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

ランダムな移動が少なすぎると次の降下は同じ局所最適解に戻り、多すぎるとどのラウンドもランダムな解から
始めることになります。GRASP は、摂動が
[`GreedyRandomizedConstruction`](heuristics_specific.md#greedyrandomizedconstruction) の `Iterated`
です。

## VariableNeighborhoodSearch

集中化する `search` と、だんだん強くなる `shakes` を交互に適用します。各ラウンドで shake `k`、次に
`search` を適用します。開始時の解より改善したラウンドは残して次を shake 0 から始め、改善しなければ
取り消して次は shake `k + 1` を試すので、行き詰まっている時間が長いほど強く揺さぶります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `search` | | 各 shake のあとに適用するヒューリスティック。`LocalSearch` など |
| `shakes` | | 弱いものから強いものへ並べた摂動。空であってはならない |
| `stop` | | 実行全体の `StopCondition` |

```python
vns = optopus.VariableNeighborhoodSearch(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    shakes=[
        optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=k))
        for k in (2, 4, 8)
    ],
    stop=optopus.StopCondition(max_iteration=5_000),
)
print(vns.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

shake は強さだけでなく近傍が違っても構いません。たとえば `"Flip"` の次に `"Swap"` です。

## Restart

`heuristic` を何度も実行し、`restart` を満たすたびに新しいランダムな解からやり直し、すべての試行を
通じた最良解を保持します。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `heuristic` | | やり直すヒューリスティック。それ自身の `stop` が 1 回の試行を制限する |
| `restart` | | `heuristic` の各呼び出しのあとに確認する `StopCondition`。満たされると新しいランダムな初期解から始める |
| `stop` | | 実行全体の `StopCondition` |

`restart` は試行の開始時点からではなく、実行全体のカウンタに対して判定されます。`max_iteration=n`
なら、合計で `n` 反復が過ぎて以降は `heuristic` の呼び出しのたびにやり直すので、`max_iteration=0` は
毎回やり直す、つまりマルチスタートになります。

```python
multistart = optopus.Restart(
    optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    restart=optopus.StopCondition(max_iteration=0),
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(multistart.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

`max_failed_update=m` なら、最良解が更新されないまま `m` 反復が過ぎて以降、最良解が更新されて数が
リセットされるまで、呼び出しのたびにやり直します。1 回の試行の長さは、`heuristic` 自身の
`max_failed_update` で決めてください。

## GeneticAlgorithm

定常状態型の遺伝的アルゴリズムです。各反復で集団から 2 つの親を選び、問題の交叉で組み換え、子を
`mutation` で改善し、集団の最悪のメンバーと置き換えます。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `population_size` | | メンバーの数。`>= 2` |
| `mutation` | | すべての子に適用するヒューリスティック。通常は短い `LocalSearch` か `TabuSearch` |
| `stop` | | `StopCondition`。1 反復は子 1 つ |
| `crossover` | `None` | 問題の交叉の名前。`None` はその既定。下の表を参照 |
| `sub_heuristic` | `None` | `"SubProblem"` が親の食い違う部分を解くのに使うヒューリスティック |
| `init_improvement` | `None` | ランダムな初期メンバーそれぞれに適用するヒューリスティック |
| `parent_selection` | `"Tournament"` | `"Tournament"`、`"DistantTopK"`、`"BiasedFitness"`。下を参照 |
| `parent_top_k` | `None` | `"DistantTopK"` が選ぶ対象となる上位メンバーの数。`"DistantTopK"` では必須 |
| `n_elite` | `4` | `"BiasedFitness"` がコストだけで残すメンバーの数。`population_size` より小さいこと |
| `n_closest` | `5` | `"BiasedFitness"` がメンバーの多様性を測る相手となる近傍の数 |

```python
ga = optopus.GeneticAlgorithm(
    population_size=20,
    mutation=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    stop=optopus.StopCondition(max_iteration=200),
)
print(ga.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

各問題が受け付ける交叉です。先頭が既定です。

| 問題 | `crossover` |
|---|---|
| `MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` | `"Uniform"`、`"SubProblem"` |
| `Tsp` | `"Order"`、`"SubProblem"` |
| `Vrp`、`FlowShop`、`Qap` | `"Order"` |
| `JobShopScheduling` | `"Ppx"` |
| `BinPacking` | `"Group"` |
| `GraphColoring` | `"Uniform"` |
| Python の問題 | その `crossover(a, b, rng)` メソッド。`crossover` は指定しない |

`"SubProblem"` は両親が一致する部分を残し、残りをより小さなインスタンスとして `sub_heuristic` で
解きます。タブー探索の突然変異と `init_improvement` を組み合わせると、ハイブリッド進化アルゴリズム
になります。

```python
hea = optopus.GeneticAlgorithm(
    population_size=10,
    mutation=optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=50)),
    stop=optopus.StopCondition(max_iteration=100),
    crossover="SubProblem",
    sub_heuristic=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50)),
    init_improvement=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50)),
)
print(hea.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

親の選び方は、集団が多様性を失う速さを決めます。

- `"Tournament"`: 各親は、ランダムな 2 つのメンバーのうち良いほうです。
- `"DistantTopK"`: 上位 `parent_top_k` 個のメンバーのうち最も離れた組です。異なる解どうしを組み換え
  続けます。
- `"BiasedFitness"`: コストと、最も近い `n_closest` 個のメンバーとの距離の両方による Vidal の順位付け
  で、親の選択と集団から外すメンバーの選択に使います。上位 `n_elite` 個はコストだけで残します。

後の 2 つは解の間の距離を読みます。組み込みの問題はすべて距離を持ち、Python の問題は省略可能な
`distance(a, b)` で与えます。

## BreakoutLocalSearch.from_parts

他のヒューリスティックから組み立てた breakout local search で、どの問題にも使えます。各ラウンドで
`descent` を局所最適まで実行し、次に 1 種類の摂動を `l` ステップ適用します。摂動は、最良解を更新した
直後か、乱数が directed の確率から外れたときは `random`、それ以外は `directed` のうちの 1 つを
その割合に応じて選びます。directed の確率は 1 から始まり、探索が停滞するにつれて `p0` に向かって
下がります。`l` は新しい局所最適解が現れるたびに `l0` に戻り、同じものが再び現れると 1 増えます。
最良解の更新がないまま `t` ラウンドが過ぎると、ランダムな摂動に切り替わります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `descent` | | 毎ラウンド局所最適まで実行するヒューリスティック。`LocalSearch` など |
| `random` | | 履歴を読まない摂動。`RandomWalk` など。その 1 反復が摂動の 1 ステップ |
| `directed` | | `(heuristic, share)` の組の空でないリスト。`TabuSearch` のようにタブーの記憶を読む摂動と、directed の確率に占める非負の割合 |
| `tabu_tenure` | | `(min, max)`。directed の摂動が記録する禁止期間 |
| `t` | | ランダムな摂動に切り替わるまでの、最良解の更新がないラウンドの数 |
| `l0` | | 摂動の長さの初期値。`>= 1` |
| `p0` | | directed の確率の下限。`[0, 1]` |
| `stop` | | `StopCondition` |

```python
bls = optopus.BreakoutLocalSearch.from_parts(
    descent=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100)),
    random=optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=1)),
    directed=[(optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=1)), 1.0)],
    tabu_tenure=(2, 5),
    t=50,
    l0=2,
    p0=0.5,
    stop=optopus.StopCondition(max_iteration=3_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

長さ `l` の摂動は選んだヒューリスティックの単独の反復を `l` 回行うものなので、摂動自身の停止条件は
読まれません。そこにはどんな `StopCondition` を渡しても構いません。スケジュールは局所最適解を距離で
区別するので、Python の問題は `distance(a, b)` を定義するか `==` に頼ります。`MaxCut` では
[`BreakoutLocalSearch`](heuristics_specific.md#breakoutlocalsearch) のコンストラクタが、Benlic と
Hao の調整済みの版を直接作ります。

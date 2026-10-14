---
source_sha256: 219681dab971
---

# クイックスタート

このページでは小さな問題をいくつか解いたあと、ランダムなインスタンスの生成と、より専門的な
ヒューリスティックの使い方を紹介します。パッケージは[インストール](installation.md)済みとします。

## Max Cut

`MaxCut` は**最大化**問題です。重み付きグラフの頂点を 2 つの側に分け、分割をまたぐ辺の重みの合計を
できるだけ大きくします。

```python
import optopus

# 重み付き辺リストからグラフを作る (頂点番号は 0 から)。
mc = optopus.MaxCut.from_edges([
    (0, 1, 1.0),
    (1, 2, 2.0),
    (0, 2, 3.0),
])

# ヒューリスティックと停止条件を設定する。
ls = optopus.LocalSearch(
    neighbor="Flip",
    stop=optopus.StopCondition(max_iteration=10_000),
)

# 再現できるようにシードを固定して 5 回実行する。
report = ls.run(mc, runs=5, seed=42)

print(report.best_objective)    # 5.0  (カットする辺 (0,2)=3 と (1,2)=2)
print(report.runs[0].solution)  # [True, True, False] — 各頂点がどちら側か
```

最適なカットは頂点 `2` を `{0, 1}` から切り離し、重い 2 本の辺をカットします (3.0 + 2.0 = 5.0)。

## QUBO

`Qubo` は**最小化**問題です。2 値ベクトル $x \in \{0, 1\}^n$ について $x^\top Q x$ を最小化します。
係数は `(i, j, coefficient)` の 3 つ組で与え、係数は**整数**です。

```python
import optopus

# 最小化: -x0 - x1 + 2*x0*x1
q = optopus.Qubo.from_entries([
    (0, 0, -1),
    (0, 1, 2),
    (1, 1, -1),
])

sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=10.0,
    cooling_rate=0.99,
    stop=optopus.StopCondition(max_duration_secs=0.2),
)

report = sa.run(q, runs=3, seed=7)

print(report.best_objective)    # -1.0
print(report.runs[0].solution)  # [True, False] — 各変数の値
```

## TSP

`Tsp` は**最小化**問題です。すべての都市をちょうど 1 回ずつ訪れる最短の巡回路を求めます。解は
上の 2 値問題で使った `list[bool]` ではなく、都市番号の順列 `list[int]` です。

```python
import optopus

# 単位正方形の 4 隅と中心にある 5 都市。
tsp = optopus.Tsp.from_coordinates(
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)],
    name="square",
)

ls = optopus.LocalSearch(
    neighbor="TwoOpt",
    stop=optopus.StopCondition(max_iteration=10_000),
)

report = ls.run(tsp, runs=3, seed=0)

print(report.best_objective)    # ~ 4.83  (正方形の周 ≈ 4.0 + 中心への寄り道 2 回)
print(report.runs[0].solution)  # 例: [0, 4, 2, 1, 3] — 都市を訪れる順序
```

TSP で使える近傍は `"TwoOpt"` (巡回路の一部を反転する) と `"Relocate"` (都市を 1 つ取り出して
別の位置に挿入する) です。

距離は座標から求めなくても構いません。`Tsp.from_distance_matrix(matrix)` は距離行列を直接受け取り、
`Tsp.load_file(path)` は TSPLIB のインスタンスを読み込みます。

```{note}
この問題は、これらのコンストラクタが加わる前は `TspWithCoordinates` という名前でした。optopus が
`Tsp` に改名し、バインディングもそれに合わせたため、旧名はもうありません。
```

(vehicle-routing)=
## 配送計画

`Vrp` は容量制約付き配送計画問題です。デポから出発してすべての顧客をちょうど 1 回ずつ、車両の
容量を超えずに回り、総距離を最小化します。番号 `0` がデポで、解は `list[list[int]]` です。車両
1 台につき顧客番号の経路を 1 つ持ち、両端のデポは省略します。

```python
import optopus

vrp = optopus.Vrp.from_coordinates(
    coordinates=[(0.0, 0.0), (10.0, 0.0), (11.0, 0.0), (-10.0, 0.0), (-11.0, 0.0)],
    demands=[0, 5, 5, 5, 5],   # 先頭はデポ
    capacity=15,
    num_vehicles=2,            # 0 なら optopus が台数を決める
)

hgs = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=20_000))
report = hgs.run(vrp, runs=3, seed=42)

print(report.best_objective)               # 44.0 — 各クラスタに 1 台ずつ
print(report.runs[0].solution)             # 例: [[1, 2], [3, 4]]
print(vrp.evaluate_routes([[1, 2], [3, 4]]))
# {'objective': 44.0, 'distance': 44.0, 'overload': 0, 'route_loads': [10, 10]}
```

ヒューリスティックが最小化する目的関数は `distance + penalty_weight * overload` です。容量を超える
解は棄却されるのではなく、どの実行可能解よりも悪く評価されます。距離そのものと超過量を個別に
知るには `evaluate_routes` を使います。

汎用ヒューリスティック向けの近傍 (`"Relocate"`、`"Swap"`、`"TwoOpt"`) のほかに、`Vrp` には専用の
ヒューリスティックが 2 つあります: `HybridGeneticSearch` と `AdaptiveLargeNeighborhoodSearch` です。`Vrp.load_file(path)` は CVRPLIB の
インスタンスを読み込み、`Vrp.from_distance_matrix(...)` は距離行列を直接受け取ります。

(graph-coloring)=
## グラフ彩色

`GraphColoring` は、どの辺も同じ色の頂点どうしを結ばないように各頂点に色を割り当て、使う色の数を
できるだけ少なくします。衝突は禁止ではなくペナルティで扱い、その重み (`num_vertices + 1`) は、
衝突を 1 つ解消するほうが色を 1 つ減らすよりも常に得になる大きさです。

```python
import optopus

# 5 頂点の閉路: 3 色が必要になる最小のグラフ。
gc = optopus.GraphColoring.from_edges(
    [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0)]
)

print(gc.num_colors())   # 3 — パレットの既定は最大次数 + 1

ts = optopus.TabuSearch(
    neighbor="Recolor", tabu_tenure=(3, 10),
    stop=optopus.StopCondition(max_iteration=10_000),
)
report = ts.run(gc, runs=4, seed=1)

print(report.best_objective)               # 3.0 — 正しい 3 彩色
print(gc.evaluate_colors([0, 1, 0, 1, 2]))
# {'objective': 3, 'colors_used': 3, 'conflicts': 0}
```

`best_objective` が `penalty_weight()` より小さければ彩色は正しく、その値が使った色の数です。近傍は
`"Recolor"` (1 頂点の色を変える) と `"Swap"` (2 頂点の色を入れ替える) です。

(generating-a-random-graph)=
## ランダムグラフの生成

`MaxCut`、`VertexCover`、`GraphColoring` はグラフ上で定義されるので、辺リストを手で書く代わりに
ランダムグラフのモデルから生成できます。生成器が作るのは重みなしのグラフで、整数の重みを付けるには
`with_random_weights` をつなげます。`Graph.grid_torus_2d(l)` と `Graph.grid_torus_3d(l)` は周期境界の
格子で、G-set のトーラス群と同じトポロジーです。

```python
import optopus

# Erdős-Rényi G(n, p)。重みは 1..10 から一様に選ぶ。
g = optopus.Graph.erdos_renyi(200, 0.05, seed=42).with_random_weights((1, 10), seed=42)

print(g)                 # Graph(num_vertices=200, num_edges=...)

mc = optopus.MaxCut.from_graph(g)
report = optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=50_000)).run(mc, seed=42)
print(report.best_objective)
```

`barabasi_albert(n, m, ...)` (スケールフリー) と `watts_strogatz(n, k, beta, ...)` (スモールワールド)
もあります。どちらも辺の数は決まっていて、それぞれ `m*(m-1)/2 + m*(n-m)` と `n*k/2` です。
`seed` を渡すと生成器は再現可能になり、省略すると時刻から新しいシードを取ります。

(going-beyond-a-single-neighborhood)=
## 1 つの近傍の先へ

`VariableNeighborhoodSearch` は、集中化のための `search` ヒューリスティックと、だんだん大きく解を
揺さぶる `shakes` のリストを交互に適用します。各ステップは普通のヒューリスティックのインスタンス
なので、ステップごとに近傍も予算も変えられます。

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=42))

vns = optopus.VariableNeighborhoodSearch(
    search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=200)),
    shakes=[
        optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=5)),
        optopus.RandomWalk("Swap", stop=optopus.StopCondition(max_iteration=15)),
    ],
    stop=optopus.StopCondition(max_iteration=20_000),
)

print(vns.run(mc, runs=3, seed=42).best_objective)
```

(problem-specific-heuristics)=
## 問題専用ヒューリスティック

1 種類の問題の構造を利用するアルゴリズムもあり、それらは `neighbor` 引数を取りません。別の問題で
実行すると `ValueError` になります。

```python
import optopus

# SAT 向けの WalkSAT/SKC: 充足されていない節を選び、その変数の 1 つを反転する。
sat = optopus.Sat.from_clauses(3, [[1, 2], [-1, 3], [2, -3]])
walksat = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=10_000), noise=0.3)
print(walksat.run(sat, seed=42).best_objective)   # 3.0 — すべての節が充足

# Max Cut 向けの breakout local search: 適応的な摂動を伴うタブー降下法。
mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(200, 0.05, seed=42))
bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(6, 100), t=1_000, l0=20, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=20_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)

# ユークリッド TSP 向けの Lin-Kernighan-Helsgaun。
tsp = optopus.Tsp.from_coordinates(
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], name="unit-square",
)
lkh = optopus.LinKernighanHelsgaun(stop=optopus.StopCondition(max_iteration=1_000))
print(lkh.run(tsp, seed=42).best_objective)       # 4.0 — 正方形の周長
```

`AdaptiveLargeNeighborhoodSearch` は `Tsp`、`Vrp`、`FlowShop`、`BinPacking` でも動き、
`HybridGeneticSearch` は `Vrp` で動きます。それぞれの引数と例は
[問題専用ヒューリスティック](heuristics_specific.md)
にあります。

```{note}
`BreakoutLocalSearch` の `tabu_tenure` は、`TabuSearch` のものと同じ禁止期間を意味します。Benlic と
Hao の γ はアルゴリズム内部で 2 回数えられるので、G-set で彼らの `rand[3, |V|/10]` を再現するには
`(6, len(vertices) // 5)` を渡します。
```

(next-steps)=
## 次のステップ

- [探索の実行と結果の読み方](running.md) — 実行回数、シード、停止条件、結果の全フィールド。
- [組み込みの問題](problems.md) — 問題の選び方、制約、各問題の例。
- [Formula によるモデル化](formula.md) — 多項式、制約、ペナルティの重み。
- [ヒューリスティックの選び方](heuristics.md) — どのヒューリスティックをどう設定するか。種類ごとの
  ページ: [汎用](heuristics_generic.md)、[合成](heuristics_composed.md)、
  [問題専用](heuristics_specific.md)、[厳密解法](heuristics_exact.md)。
- [グラフとテスト用インスタンス](graphs_and_instances.md) — ランダムグラフ、最適解を埋め込んだインスタンス、カーネル化。
- [例](examples.md) — 複数実行の統計とヒューリスティックの比較。
- [API リファレンス](api_reference.md) — すべてのクラス、メソッド、引数。

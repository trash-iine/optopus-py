---
source_sha256: aee1f33ee74f
---

# グラフとテスト用インスタンス

`MaxCut`、`VertexCover`、`GraphColoring` は、無向の重み付きグラフ上で定義されます。このページでは、
辺リストからグラフを作り、ランダムなモデルから生成し、格子を並べる `Graph` と、Max Cut 用の 2 つの
道具を説明します。最適値が分かっているインスタンスを作る `PlantedMaxCut` と、探索の前にインスタンスを
厳密に縮約する `MaxCutKernel` です。

## Graph

頂点は 0 から番号が付き、辺から取られます。コンストラクタはすべて静的メソッドで、ランダムなものは
省略可能な `seed` を取ります。同じシードからは同じグラフが生成され、`None` なら呼び出しごとに新しい
グラフが生成されます。

| コンストラクタ | 作るもの |
|---|---|
| `Graph.from_edges(edges)` | `(u, v, weight)` の 3 つ組からなるグラフ |
| `Graph.erdos_renyi(n, p, seed=None)` | Erdős-Rényi G(n, p): ありうる `n (n − 1) / 2` 本の辺がそれぞれ確率 `p` で存在する |
| `Graph.barabasi_albert(n, m, seed=None)` | Barabási-Albert の優先的選択によるスケールフリーグラフ: `m` 頂点のクリークから始め、以後の各頂点を次数に応じて既存の `m` 頂点とつなぐ。辺は `m (m − 1) / 2 + m (n − m)` 本、`1 <= m < n` |
| `Graph.watts_strogatz(n, k, beta, seed=None)` | Watts-Strogatz のスモールワールド: 各頂点を近い `k` 頂点とつないだ環で、各辺を確率 `beta` でつなぎ替える。辺は `n k / 2` 本、`k` は偶数で `n` 未満 |
| `Graph.grid_torus_2d(l)` | 周期境界の正方格子: 頂点 `l²`、辺 `2 l²`、4 正則、`l >= 3`。頂点 `m + l · n` が `(m, n)` にある |
| `Graph.grid_torus_3d(l)` | 周期境界の立方格子: 頂点 `l³`、辺 `3 l³`、6 正則、`l >= 3`。頂点 `m + l · n + l² · k` が `(m, n, k)` にある |
| `Graph.load_file(path)` | 辺リストのファイル。`N M` の後に辺ごとに `i j w`、頂点番号は 1 から、`w` は省略でき省略時は 1.0 (G-set の形式) |

| メソッド | 返すもの |
|---|---|
| `with_random_weights(weight_range, seed=None)` | 各重みを両端を含む整数範囲 `(min, max)` から一様に引いたコピー。0 は引かれない。範囲は ±2²⁴ 以内 |
| `num_vertices()` | 辺を 1 本以上持つ頂点の数 |
| `num_edges()` | 辺の数 |
| `edges()` | すべての辺を 1 回ずつ、`(u, v, weight)` で |
| `degree(v)` | `v` に接する辺の数 |
| `neighbors(v)` | `v` に隣接する頂点。`(neighbor, weight)` で番号の小さい順 |
| `has_edge(u, v)` | `u` と `v` を結ぶ辺があるか |
| `weight(u, v)` | `u` と `v` の間の辺の重み。辺がなければ 0.0 |
| `induced_subgraph(keep)` | `keep` の頂点からなる部分グラフ。頂点番号はそのまま |
| `write_to_file(path)` | なし。`load_file` が読む形式でグラフを書き出す。孤立頂点も重みも完全な精度で残る |

```python
import optopus

g = optopus.Graph.from_edges([(0, 1, 1.0), (1, 2, 2.0), (2, 0, 3.0), (2, 3, 1.0)])
print(g)          # Graph(num_vertices=4, num_edges=4)
print(g.edges())  # [(0, 1, 1.0), (0, 2, 3.0), (1, 2, 2.0), (2, 3, 1.0)]

print(optopus.Graph.barabasi_albert(100, 2, seed=42))     # Graph(num_vertices=100, num_edges=197)
print(optopus.Graph.watts_strogatz(100, 4, 0.1, seed=42))  # Graph(num_vertices=100, num_edges=200)
print(optopus.Graph.grid_torus_2d(8))                      # Graph(num_vertices=64, num_edges=128)
```

生成器と格子が作るのは重みなしのグラフで、重みはすべて 1.0 です。整数の重みを付けるには
`with_random_weights` をつなげます。たとえば `(-1, 1)` なら格子上の ±J スピングラスになります。

```python
spin_glass = optopus.Graph.grid_torus_2d(8).with_random_weights((-1, 1), seed=3)
print(sorted({w for _, _, w in spin_glass.edges()}))  # [-1.0, 1.0] — 0 は引かれない
```

グラフ上の問題はどれも `from_graph` でグラフを受け取ります。

```python
g = optopus.Graph.erdos_renyi(200, 0.05, seed=42).with_random_weights((1, 10), seed=42)

mc = optopus.MaxCut.from_graph(g)
vc = optopus.VertexCover.from_graph(g)
gc = optopus.GraphColoring.from_graph(g, num_colors=8)
print(mc)  # MaxCut(num_vertices=200, num_edges=1017)
```

辺を持たない頂点は `num_vertices()` に数えられませんが、解は辺に現れる最大の番号までの各番号について
1 つずつ要素を持ちます。孤立した頂点が残るほど疎なランダムグラフでは、報告される数が `n` より少なく
なります。`Graph.erdos_renyi(100, 0.05, seed=42)` は 98 です。

## PlantedMaxCut

ランダムなインスタンスでは、良い結果と最適な結果を見分けられません。`PlantedMaxCut` は構成によって
最適なカットを埋め込み、それを分からないように変形するので、最適値が正確に分かり、「このヒューリス
ティックはどれくらいの頻度で最適値に達するか」を数えられるようになります。

| コンストラクタ | 作るもの |
|---|---|
| `PlantedMaxCut.wishart(n, alpha, couplers="Gaussian", seed=None)` | `n >= 2` 頂点の完全グラフ上の Wishart planted ensemble。`alpha` は `(0, 1)` で、小さいほど難しい。`couplers="Discrete"` は整数の重みになり、最適値が厳密に保たれる |
| `PlantedMaxCut.tile_planting_2d(l, p1, p2, p3, seed=None)` | `l × l` トーラス上の tile planting。`l` は偶数で `>= 4`。`p1`、`p2`、`p3` は 4 つのタイルのクラスのうち 3 つの確率で、残りが 4 つ目になる |
| `PlantedMaxCut.tile_planting_3d(l, p_2fp, p_4fp, seed=None)` | `l³` トーラス上の tile planting。2 個と 4 個のフラストレートしたプラケットを持つクラスの確率で、残りが 6 個のクラスになる |

| メソッド | 返すもの |
|---|---|
| `problem()` | インスタンス。普通の `MaxCut` |
| `planted()` | 埋め込んだ割当て。最適なカットで、頂点ごとに `bool` |
| `optimum()` | そのカットの重み、つまり最適値 |
| `has_exact_optimum()` | すべての重みが整数で、報告される目的関数値が `optimum()` と厳密に一致しうるか |
| `verify()` | 構成を検査し直す。最初に見つかった不整合を説明する `ValueError` を送出する |

```python
# 整数の結合なら最適値が f32 の目的関数値でも厳密に保たれるので、到達の判定は丸めについての
# 判断ではなく等号になる。
planted = optopus.PlantedMaxCut.wishart(64, 0.75, couplers="Discrete", seed=1)
planted.verify()
print(planted)  # PlantedMaxCut(num_vertices=64, num_edges=2016, optimum=3056960, exact=true)

bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(6, 12), t=1_000, l0=10, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=50_000),
)
report = bls.run(planted.problem(), runs=20, seed=42)

hits = sum(1 for r in report.runs if r.best_objective == planted.optimum())
print(f"reached the optimum in {hits}/{len(report.runs)} runs")
```

既定のガウス分布の結合では重みは 32 ビット浮動小数点数で保持される実数になり、
`has_exact_optimum()` は `False` です。その場合は `math.isclose` で比べてください。tile planting の
格子は常に整数の重みを持ち、クラスの確率で難しさを調整します。

```python
tiles = optopus.PlantedMaxCut.tile_planting_2d(8, 0.2, 0.3, 0.3, seed=1)
print(tiles.optimum(), tiles.has_exact_optimum())  # 54.0 True
```

## MaxCutKernel

`MaxCutKernel.reduce(problem)` は、孤立頂点と次数 1 の頂点、次数 2 の頂点の道、重みの支配という厳密な
縮約規則を、どれも適用できなくなるまで適用します。カーネルは普通の `MaxCut` なので、どの
ヒューリスティックもそのまま動きます。小さくなるのはヒューリスティックが見るインスタンスだけです。

| メソッド | 返すもの |
|---|---|
| `MaxCutKernel.reduce(problem)` | `MaxCut` の縮約 |
| `kernel()` | 縮約したインスタンス。`MaxCut` |
| `offset()` | カーネルのカットに足すと元のカットになる定数 |
| `is_trivial()` | 何も取り除けなかったか。正則グラフや密なグラフではそうなる |
| `removed_vertices()` | 規則が取り除いた頂点の数 |
| `lift(assignment)` | カーネルの割当てを元の頂点に戻したもの。取り除いた頂点は最適に設定される |
| `project(assignment)` | 元の割当てをカーネルに制限したもの。ウォームスタート用 |

カーネルのどの割当て `y` についても `kernel_cut(y) + offset == original_cut(lift(y))` が成り立つので、
元に戻した解は報告された値のとおりの良さを持ちます。

```python
mc = optopus.MaxCut.from_graph(
    optopus.Graph.barabasi_albert(2_000, 2, seed=7).with_random_weights((1, 10), seed=7)
)

kernel = optopus.MaxCutKernel.reduce(mc)
print(kernel)
# MaxCutKernel(original_vertices=2000, kernel_vertices=771, removed=1229, offset=12841)

stop = optopus.StopCondition(max_iteration=100_000)
if kernel.is_trivial():
    # 正則グラフや密なグラフは縮約しても変わらないので、カーネルを経由しない。
    report = optopus.LocalSearch("Flip", stop).run(mc, runs=4, seed=1)
    best, assignment = report.best_objective, report.runs[0].solution
else:
    report = optopus.LocalSearch("Flip", stop).run(kernel.kernel(), runs=4, seed=1)
    best = report.best_objective + kernel.offset()
    assignment = kernel.lift(report.runs[0].solution)

print(f"cut {best} over {len(assignment)} vertices")
```

`project` は逆向きです。元の問題の解をカーネルに制限したものを `initial_solution` に渡すと、カーネル上の
探索をそこから始められます ({ref}`解から始める <starting-from-a-solution>`を参照)。

```python
start = kernel.project(assignment)
tabu = optopus.TabuSearch(
    neighbor="Flip", tabu_tenure=(5, 10), stop=optopus.StopCondition(max_iteration=200)
)
again = tabu.run(kernel.kernel(), seed=2, initial_solution=start)
```

規則がよく効くのは、Barabási-Albert グラフの多数の低次数頂点のような、疎で不規則なグラフです。格子
には何もしません。`grid_torus_2d` のインスタンスの `MaxCutKernel.reduce` は自明になります。

---
source_sha256: 547ce4cd8ee0
---

# 組み込みの問題

このページでは optopus に含まれる問題を比較します。それぞれが何を最適化するか、解がどんな形か、
どの制約を問題自身が扱うか、実行が何を報告するかを説明します。すべてのコンストラクタと引数は
[API リファレンス](api_reference.md)に、ほとんどの問題の
実例は[クイックスタート](quickstart.md)にあります。

(choosing-a-representation)=
## 表現の選び方

タスクを探索にかける方法は 3 つあります。タスクにぴったり合う最初のものを選んでください。

1. **組み込みの問題**: タスクがその問題そのもので、それ以上の要素がないとき。最も高速です。移動の
   評価は移動が触れる部分だけから計算され、問題によっては専用のヒューリスティックもあります。
   問題がモデル化していない追加の制約があれば使えません。たとえば `Qubo` には制約が一切ないので、
   QUBO に予算制約を付けたいなら `Formula` を使います。
2. **[`Formula`](formula.md)**: すべての決定変数が有界な整数で、目的関数と制約がそれらの多項式で
   書けるとき。多項式以外に書くものはなく、速度も組み込みの問題とほぼ同じです。
3. **[Python で書いた問題](python_problems.md)**: それ以外のすべて。構造を持つ解、多項式でない
   目的関数、構成によって守る制約などです。最も柔軟ですが、ステップごとに Python を呼び出すので
   最も低速です。

(at-a-glance)=
## 一覧

| 問題 | 最適化するもの | 解 | 近傍 | 問題専用ヒューリスティック |
|---|---|---|---|---|
| `MaxCut` | カットの重みの最大化 | `list[bool]`、各頂点がどちら側か | `"Flip"`、`"Swap"` | `BreakoutLocalSearch` |
| `Qubo` | x^T Q x の最小化 | `list[bool]`、各変数 | `"Flip"`、`"Swap"` | |
| `Sat` | 充足される節の数の最大化 | `list[bool]`、各変数 | `"Flip"`、`"Swap"` | `WalkSat` |
| `VertexCover` | 被覆の大きさの最小化 | `list[bool]`、各頂点が被覆に入るか | `"Flip"`、`"Swap"` | |
| `Tsp` | 巡回路長の最小化 | `list[int]`、都市の順列 | `"TwoOpt"`、`"Relocate"` | `LinKernighanHelsgaun`、`AdaptiveLargeNeighborhoodSearch`、`GreedyRandomizedConstruction` |
| `JobShopScheduling` | メイクスパンの最小化 | `list[int]`、作業の列 | `"Swap"`、`"Relocate"` | |
| `FlowShop` | メイクスパンの最小化 | `list[int]`、ジョブを処理する順序 | `"Relocate"`、`"Swap"` | `IteratedGreedy`、`AdaptiveLargeNeighborhoodSearch`、`GreedyRandomizedConstruction` |
| `Vrp` | 総距離の最小化。混成車両では時間とコスト | `list[list[int]]`、車両ごとの顧客の経路 | `"Relocate"`、`"Swap"`、`"TwoOpt"` | `HybridGeneticSearch`、`AdaptiveLargeNeighborhoodSearch`、`GreedyRandomizedConstruction` |
| `Qap` | 割当てコストの最小化 | `list[int]`、各施設の場所 | `"Swap"` | |
| `BinPacking` | 使う箱の数の最小化 | `list[int]`、各品物の箱 | `"Relocate"`、`"Swap"` | `AdaptiveLargeNeighborhoodSearch`、`GreedyRandomizedConstruction` |
| `GraphColoring` | 使う色の数の最小化 | `list[int]`、各頂点の色 | `"Recolor"`、`"Swap"` | |
| `Formula` | 多項式。向きはどちらも可 | `list[int]`、各変数 | `"Change"` (別名 `"Flip"`)、`"Swap"`、`"Reverse"` | |

汎用ヒューリスティックと合成ヒューリスティックは、どれもすべての問題で動きます。`GeneticAlgorithm`
が各問題で使える交叉は
[GeneticAlgorithm](heuristics_composed.md#geneticalgorithm)に一覧があります。

`BranchAndBound` は `MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` で最適性を証明します。
[厳密解法](heuristics_exact.md)を参照してください。

(neighborhoods)=
## 近傍

汎用ヒューリスティックは近傍を文字列として `neighbor` 引数で受け取ります。知らない名前は `run` を
呼んだときに `ValueError` になります。

| 問題 | `neighbor` | 移動 |
|---|---|---|
| `MaxCut`、`Qubo`、`Sat`、`VertexCover` | `"Flip"` | 変数を 1 つ反転する |
| | `"Swap"` | 2 つの変数の値を入れ替える。`True` の個数は変わらない |
| `Tsp` | `"TwoOpt"` | 巡回路の一部を反転する |
| | `"Relocate"` | 都市を 1 つ巡回路の別の位置に移す |
| `JobShopScheduling` | `"Swap"` | 列の中で隣り合う 2 つの作業を入れ替える |
| | `"Relocate"` | 作業を 1 つ列の別の位置に移す |
| `FlowShop` | `"Relocate"` | ジョブを 1 つ列の別の位置に移す (挿入近傍) |
| | `"Swap"` | 2 つの位置のジョブを入れ替える |
| `Qap` | `"Swap"` | 2 つの施設の場所を入れ替える |
| `BinPacking` | `"Relocate"` | 品物を 1 つ、空きのある別の箱に移す。最後の 1 つなら元の箱は空になる。ほかの品物と同じ箱にあれば、新しい箱に 1 つだけで移すこともできる |
| | `"Swap"` | 別々の箱にある大きさの異なる 2 つの品物を、両方とも収まる場合に入れ替える |
| `Vrp` | `"Relocate"` | 顧客を 1 人、別の経路のある位置に移す |
| | `"Swap"` | 別々の経路にいる 2 人の顧客を入れ替える |
| | `"TwoOpt"` | 1 つの経路の中で一部を反転する |
| `GraphColoring` | `"Recolor"` | 1 つの頂点に別の色を付ける |
| | `"Swap"` | 色の異なる 2 つの頂点の色を入れ替える |
| `Formula` | `"Change"` | 1 つの変数を範囲内の別の値にする |
| | `"Swap"` | 2 つの変数の値を入れ替える |
| | `"Reverse"` | ある範囲の変数の値を逆順にする |

`Vrp` の `"Relocate"` と `"Swap"` は経路の間で顧客を動かすだけで、`"TwoOpt"` は 1 つの経路を並べ
替えるだけです。両方を行うには `VariableNeighborhoodSearch` か `Sequential` で組み合わせるか、自分で
両方を行う `HybridGeneticSearch` を使ってください。

(what-a-run-reports)=
## 実行が報告する値

`RunResult.best_objective`、`initial_objective` とレポートの集計値は、探索が解の順位付けに使う値です。
問題によっては、これは素の目的関数値ではありません。

| 問題 | 報告される値 | 良いのは |
|---|---|---|
| `MaxCut` | カットの重み | 大きいほう |
| `Qubo` | x^T Q x | 小さいほう |
| `Sat` | 充足される節の数 | 大きいほう |
| `VertexCover` | `cover_size + (num_vertices + 1) · uncovered_edges` | 小さいほう |
| `Tsp` | 巡回路長 | 小さいほう |
| `JobShopScheduling`、`FlowShop` | メイクスパン | 小さいほう |
| `Qap` | 割当てコスト | 小さいほう |
| `BinPacking` | `num_bins − mean((load / capacity)²)`。[BinPacking](#binpacking) を参照 | 小さいほう |
| `Vrp` | `distance + penalty_weight · overload`。混成車両は[混成車両](#vrp-mixed-fleet)を参照 | 小さいほう |
| `GraphColoring` | `colors_used + (num_vertices + 1) · conflicts` | 小さいほう |
| `Formula` | `objective − penalty`、最小化のときは `−(objective + penalty)` | 大きいほう |
| Python の問題 | `objective(solution)` | `minimize` の指定に従う |

`RunReport.best_objective` と `worst_objective` はこの向きに従います。最小化問題では最も小さい値が
最良です。`Formula` は常に大きいほうが良い値として報告されるので、最小化する場合でも
`best_objective` は全実行の最大値です。[探索が報告する値](#what-the-search-reports)を参照して
ください。

目的関数値は浮動小数点数で、探索は移動のたびにそれを差分で更新します。そのため、実行が報告する値は
改めて評価した値と末尾の桁が食い違うことがあります。データが整数でない限り、目的関数値は
`math.isclose` のように許容誤差付きで比べ、手元に残す解の目的関数値は問題の評価関数で計算し直して
ください。`Vrp` ではそうしたわずかな差も改善として数えられることがあり、その結果
`n_best_updates` と `time_to_best_secs` は探索が改善を続けた期間を実際より長く示し、
`max_failed_update` による停止も遅れます。

レポート自体は最良解を持っていません。取り出し方は[最良解を取り出す](#picking-the-best-solution)に
あります。

(constraints-a-problem-handles-itself)=
## 問題自身が扱う制約

問題によっては、探索の途中で解が破ってもよい厳しい制約を持ちます。それらは、実行可能解が存在する
限り最適解が必ず実行可能になるだけの重みでペナルティを課します。こうした制約のために何かを追加する
必要はありません。結果は問題の評価関数で確認してください。

| 問題 | 制約 | ペナルティの重み | 確認方法 |
|---|---|---|---|
| `VertexCover` | すべての辺が被覆される | 被覆されない辺 1 本につき `num_vertices + 1` | 解そのもの: 両端とも `True` でない辺は被覆されていない |
| `Vrp` | 車両の容量。混成車両では経路時間の上限と最小台数も | 超過 1 単位につき `(num_customers + num_vehicles) · longest_edge + 1`。速度・サービス時間・コストがあればさらに大きい。`penalty_weight()` | `evaluate_routes(routes)["overload"] == 0`。混成車両では `time_excess` と `min_count_shortfall` も |
| `GraphColoring` | 同じ色どうしを結ぶ辺がない | 衝突 1 つにつき `num_vertices + 1`、`penalty_weight()` | `evaluate_colors(colors)["conflicts"] == 0` |
| `Formula` | 与えた制約 | それぞれに与えた重み | `eval_penalty(values) == 0` |

他の組み込みの問題は、実行不可能な解をそもそも表現できません。どの順列も巡回路・ジョブの順序・
施設の割当てになり、どの作業列も正しいスケジュールに復号され、どの割当てもカットや MaxSAT の
割当てになり、`BinPacking` の移動は品物を収まる場所にしか置きません。

(per-problem)=
## 問題ごとの説明

各節では、問題の作り方とそれ以外に提供している機能を挙げ、最適値が分かっている小さなインスタンスを
解きます。コンストラクタはすべて静的メソッドで、`name` を取るものの既定値は `""` (`Vrp` では
`"vrp"`) です。`load_file` はインスタンスにファイル名を付けます。

### MaxCut

頂点を 2 つの側に分け、両側をまたぐ辺の重みを最大化します。

| コンストラクタ | 作るもの |
|---|---|
| `MaxCut.from_edges(edges)` | `(u, v, weight)` の辺からなるインスタンス。頂点番号は 0 から |
| `MaxCut.from_graph(graph)` | [`Graph`](graphs_and_instances.md#graph) 上のインスタンス |
| `MaxCut.load_file(path)` | G-set 形式の辺リストのファイル。`N M` の後に辺ごとに `i j w`、頂点番号は 1 から |

| メソッド | 返すもの |
|---|---|
| `cut_size(assignment)` | 割り当てが切る辺の重みの合計。実行が最大化する値 |
| `graph()` | 元になる [`Graph`](graphs_and_instances.md#graph) のコピー |

```python
import optopus

# 重い辺 0–1 と 2–3、弦 0–2 を持つ 4 頂点の閉路。
mc = optopus.MaxCut.from_edges([(0, 1, 3.0), (1, 2, 1.0), (2, 3, 3.0), (3, 0, 1.0), (0, 2, 2.0)])

ts = optopus.TabuSearch("Flip", (1, 2), stop=optopus.StopCondition(max_iteration=100))
report = ts.run(mc, runs=3, seed=42)
print(report.best_objective)    # 8.0 — 弦以外のすべての辺をカット
print(report.runs[0].solution)  # [False, True, False, True] — 各頂点がどちら側か
```

重みは 32 ビット浮動小数点数で保持されるので、整数でない重みを多数足した値は丸め誤差の範囲でしか
比較できません。整数の重みなら厳密に比較できます。`MaxCutKernel` は疎なインスタンスを探索の前に
厳密に縮約し、`PlantedMaxCut` は最適値が分かっているインスタンスを作ります。
[グラフとテスト用インスタンス](graphs_and_instances.md)を参照してください。

### Qubo

2 値ベクトル x について x^T Q x を最小化します。

| コンストラクタ | 作るもの |
|---|---|
| `Qubo.from_entries(entries)` | 整数係数の `(i, j, coefficient)` からなるインスタンス |
| `Qubo.load_file(path)` | 疎行列のファイル。`N M` の後に要素ごとに `i j v`、変数番号は 1 から。同じ組が 2 回あれば後の値を使う |

`energy(assignment)` は割り当てのエネルギー、つまり実行が最小化する値を返します。

対角成分 `(i, i, c)` は 1 次の項 c·x_i、非対角成分は積の項 c·x_i·x_j です。

```python
# 最小化: −3·x0 − 2·x1 − 2·x2 + 4·x0·x1 + x1·x2 − x0·x2。
q = optopus.Qubo.from_entries(
    [(0, 0, -3), (1, 1, -2), (2, 2, -2), (0, 1, 4), (1, 2, 1), (0, 2, -1)]
)

ls = optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=100))
print([r.best_objective for r in ls.run(q, runs=3, seed=42).runs])  # [-3.0, -3.0, -6.0]

ts = optopus.TabuSearch("Flip", (1, 2), stop=optopus.StopCondition(max_iteration=100))
best = ts.run(q, runs=3, seed=42).runs[0]
print(best.best_objective, best.solution)  # -6.0 [True, False, True] — x0 = x2 = 1
```

降下法のうち 2 つは `x1` だけが 1 の局所最適解 (値 −3) で止まります。タブー探索はそこから抜け出せ
ます。

`(i, j)` と `(j, i)` は同じ係数を指し、2 回与えた成分は足し合わされずに後のもので置き換わります。
項はペアごとに 1 つの成分にまとめてから渡してください。

```python
terms = [(0, 1, 2), (1, 0, 2), (0, 0, -1)]  # 2·x0·x1 を両方向から書いたもの
summed = {}
for i, j, c in terms:
    key = (min(i, j), max(i, j))
    summed[key] = summed.get(key, 0) + c
q = optopus.Qubo.from_entries([(i, j, c) for (i, j), c in summed.items()])
```

予算などの追加制約を持つ QUBO は [`Formula`](formula.md) で書きます。

### Sat

充足される節の数を最大化します (MaxSAT。充足可能なインスタンスでは、すべての節が成り立てば解けた
ことになります)。

| コンストラクタ | 作るもの |
|---|---|
| `Sat.from_clauses(n_vars, clauses)` | `n_vars` 個の変数上のインスタンス。各節は DIMACS 形式のリテラルのリスト |
| `Sat.load_file(path)` | DIMACS CNF 形式のファイル |

| メソッド | 返すもの |
|---|---|
| `num_satisfied(assignment)` | 割り当てが満たす節の数。実行が最大化する値 |
| `num_vars()`、`num_clauses()` | インスタンスの大きさ |

リテラル `+i` は変数 `i`、`-i` はその否定で、番号は **1 から**です。一方、解のリストは 0 から
数えるので、変数 `i` は `solution[i - 1]` です。

```python
# x1、¬x1、(x1 ∨ x2): 3 つの節のうち同時に成り立つのは高々 2 つ。
sat = optopus.Sat.from_clauses(2, [[1], [-1], [1, 2]])

best = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=1_000)).run(sat, seed=42).runs[0]
print(best.best_objective)  # 2.0
x1, x2 = best.solution  # 変数 i は solution[i - 1]
```

`WalkSat` はすべての節が成り立った時点で止まります。この例のような充足不可能なインスタンスでは
`stop` まで走ります。リテラル 0 や `n_vars` を超える変数は `ValueError` になります。

### VertexCover

どの辺も少なくとも一方の端点が選ばれるように、できるだけ少ない頂点を選びます。

| コンストラクタ | 作るもの |
|---|---|
| `VertexCover.from_edges(edges)` | `(u, v, weight)` の辺からなるインスタンス。重みは無視されるので `1.0` を渡す |
| `VertexCover.from_graph(graph)` | [`Graph`](graphs_and_instances.md#graph) 上のインスタンス |
| `VertexCover.load_file(path)` | `MaxCut.load_file` と同じ形式の辺リストのファイル。重みは無視される |

| メソッド | 返すもの |
|---|---|
| `evaluate_cover(assignment)` | `objective`、`cover_size`、`uncovered_edges` を持つ辞書。目的関数は `cover_size + penalty_weight() * uncovered_edges` |
| `penalty_weight()` | 覆われていない辺 1 本あたりのペナルティ |
| `graph()` | 元になるグラフのコピー |

```python
edges = [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 0, 1.0), (0, 2, 1.0)]
vc = optopus.VertexCover.from_edges(edges)

ts = optopus.TabuSearch("Flip", (1, 3), stop=optopus.StopCondition(max_iteration=200))
best = ts.run(vc, runs=3, seed=42).runs[0]
cover = best.solution
uncovered = [(u, v) for u, v, _ in edges if not (cover[u] or cover[v])]
print(best.best_objective, cover, uncovered)  # 3.0 [False, True, True, False, True] []
```

探索は辺を取りこぼした被覆も通りますが、被覆されない辺 1 本につき `num_vertices + 1` のペナルティが
かかるので、報告された値が `num_vertices` 以下ならその大きさの正しい被覆です。被覆を使う前には、
上のように辺を確認してください。

### Tsp

すべての都市を通る最短の閉じた巡回路を求めます。

| コンストラクタ | 作るもの |
|---|---|
| `Tsp.from_coordinates(coordinates, name="", edge_weight_type="Continuous", nearest_neighbors=None)` | `(x, y)` の点の間の距離。通常のユークリッド距離か、TSPLIB の `"EUC_2D"`、`"CEIL_2D"`、`"ATT"`、`"GEO"` |
| `Tsp.from_distance_matrix(matrix, name="")` | 与えた距離。対称・有限・非負であること |
| `Tsp.load_file(path)` | TSPLIB のインスタンス |

`nearest_neighbors=k` を渡すと、各都市から近い `k` 都市への距離だけを `n²` ではなく `n × k` 個持ち、
それ以外の組は必要になったときに座標から計算します。距離は変わらず、大きなインスタンスもメモリに
収まります。`load_file` は 2000 都市を超えると自動でこうします。

| メソッド | 返すもの |
|---|---|
| `tour_length(tour)` | 閉じた巡回路の長さ。実行が最小化する値 |
| `distance(i, j)` | 都市 `i` から都市 `j` への距離 |
| `num_cities()` | 都市の数 |
| `coordinates()`、`edge_weight_type()` | 点と距離の式。行列から作ったインスタンスでは `None` |

```python
dist = [
    [0, 2, 9, 10],
    [2, 0, 6, 4],
    [9, 6, 0, 3],
    [10, 4, 3, 0],
]
tsp = optopus.Tsp.from_distance_matrix(dist)

best = optopus.LocalSearch("TwoOpt", stop=optopus.StopCondition(max_iteration=100)).run(
    tsp, seed=42
).runs[0]
print(best.best_objective, best.solution)  # 18.0 [2, 0, 1, 3] — 9 + 2 + 4 + 3
```

解は都市を訪れる順に並べたもので、最初の都市に戻る部分は省略されます。`load_file` は TSPLIB を
読み込みます。

```python
from pathlib import Path

Path("square.tsp").write_text(
    "NAME: square\nTYPE: TSP\nDIMENSION: 4\nEDGE_WEIGHT_TYPE: EUC_2D\n"
    "NODE_COORD_SECTION\n1 0 0\n2 30 0\n3 30 40\n4 0 40\nEOF\n"
)
square = optopus.Tsp.load_file("square.tsp")
print(square, square.num_cities())  # Tsp(name="square", n_cities=4) 4
```

対称・有限・非負でない距離行列は `ValueError` になります。移動は反転した区間を両端の辺から評価する
ので、どの区間も向きによらず同じ長さである場合にしか正しくありません。

### JobShopScheduling

各ジョブは順に実行しなければならない `(machine, duration)` の作業のリストで、機械は一度に 1 つの
作業しか処理できません。最後の作業が終わる時刻であるメイクスパンを最小化します。

| コンストラクタ | 作るもの |
|---|---|
| `JobShopScheduling.from_jobs(jobs, name="")` | ジョブごとの `(machine, duration)` のリストからなるインスタンス。機械番号は 0 から |
| `JobShopScheduling.load_file(path)` | Taillard / OR-Library 形式のファイル。`n_jobs n_machines` の後にジョブごとに 1 行の `machine duration` の組、機械番号は 0 から |

| メソッド | 返すもの |
|---|---|
| `makespan(operations)` | 作業の列のメイクスパン。実行が最小化する値 |
| `completion_times(operations)` | 列の各位置の作業が終わる時刻 |
| `num_jobs()`、`num_machines()` | インスタンスの大きさ |

解は、ジョブ `j` がその作業の数だけ現れる列です。`j` の `k` 回目の出現は、ジョブ `j` の `k` 番目の
作業を機械が許す最も早い時刻に割り当てます。

```python
jobs = [
    [(0, 3), (1, 2), (2, 2)],  # ジョブ 0: 機械 0 で 3、次に機械 1 で 2、...
    [(0, 2), (2, 1), (1, 4)],
    [(1, 4), (2, 3), (0, 1)],
]
shop = optopus.JobShopScheduling.from_jobs(jobs)

ts = optopus.TabuSearch("Swap", (2, 4), stop=optopus.StopCondition(max_iteration=300))
best = ts.run(shop, runs=3, seed=42).runs[0]
print(best.best_objective)  # 11.0 — 全 1,680 通りの列での最適値
print(best.solution)        # [0, 1, 1, 2, 0, 2, 0, 2, 1]
```

### FlowShop

順列フローショップです。どのジョブも同じ順序で機械を通り、どの機械も共通の 1 つの順序でジョブを
処理します。その順序が解です。メイクスパンを最小化します。

| コンストラクタ | 作るもの |
|---|---|
| `FlowShop.from_processing_times(times, name="")` | `times[machine][job]` からなるインスタンス。機械ごとに 1 行 |
| `FlowShop.load_file(path)` | Taillard の形式: `n_jobs n_machines` のあとに機械ごとに処理時間を 1 行 |

| メソッド | 返すもの |
|---|---|
| `makespan(sequence)` | ジョブの順序のメイクスパン |
| `neh()` | NEH 構築法が作る順序。標準的な基準解 |
| `num_jobs()`、`num_machines()` | インスタンスの大きさ |
| `processing_time(machine, job)` | `machine` での `job` の処理時間 |

```python
from pathlib import Path

Path("demo.fs").write_text("5 3\n5 9 8 10 1\n9 3 10 1 8\n9 4 5 8 6\n")
shop = optopus.FlowShop.load_file("demo.fs")
print(shop.num_jobs(), shop.num_machines())  # 5 3
print(shop.neh(), shop.makespan(shop.neh()))  # [4, 0, 2, 1, 3] 45

ig = optopus.IteratedGreedy(stop=optopus.StopCondition(max_iteration=200), removal_count=2)
print(ig.run(shop, runs=3, seed=42).best_objective)  # 44.0 — NEH より 1 良く、最適
```

最初に試すべきヒューリスティックは `IteratedGreedy` です。いくつかのジョブを取り出し、それぞれを
メイクスパンの増加が最も小さい位置に戻し、挿入近傍で降下します。Taillard のインスタンスでは
`AdaptiveLargeNeighborhoodSearch` に勝ります。近傍のうちフローショップの文献が頼りにするのは
`"Relocate"` で、移動は `n (n − 1)` 通り、すべて `O(n² m)` で評価されます。

### Vrp

同じ容量の車両でデポからすべての顧客を 1 回ずつ回り、総距離を最小化します。

| コンストラクタ | 作るもの |
|---|---|
| `Vrp.from_coordinates(coordinates, demands, capacity, num_vehicles=0, name="vrp", rounded=False, nearest_neighbors=None)` | ユークリッド距離。`rounded` は CVRPLIB と同じように距離を丸め、`nearest_neighbors` は `Tsp` と同じく各ノードの近傍だけを持つ |
| `Vrp.from_distance_matrix(matrix, demands, capacity, num_vehicles=0, name="vrp")` | 与えた距離。対称・有限・非負であること |
| `Vrp.with_fleet(coordinates, demands, vehicle_types, service_times=None, objective_mode="TotalTime", cost_weight=1.0, name="vrp", rounded=False)` | 混成車両。[混成車両](#vrp-mixed-fleet)を参照 |
| `Vrp.load_file(path)` | CVRPLIB のファイル、または `.toml` ファイルの混成車両 |

| メソッド | 返すもの |
|---|---|
| `evaluate_routes(routes)` | `objective`、`distance`、`overload`、`route_loads` を持つ辞書。混成車両では目的関数の全項も |
| `num_customers()`、`num_vehicles()` | インスタンスの大きさ。`num_vehicles()` はどの解にも含まれる経路の数 |
| `split_giant_tour(giant)` | 全顧客を並べた順序を、目的関数が最も小さくなる位置で切った経路 (Prins の Split) |
| `capacity()` | 車両の容量。複数の車種からなる車両では `ValueError` |
| `penalty_weight()` | 超過 1 単位あたりのペナルティ |
| `vehicle_types()`、`slot_types()`、`service_times()`、`objective_mode()`、`cost_weight()` | 車両の構成。[混成車両](#vrp-mixed-fleet)を参照 |
| `with_objective_mode(mode)` | `"TotalTime"` または `"Makespan"` で評価するコピー |

[クイックスタート](#vehicle-routing)で顧客 4 人のインスタンスを解いています。

番号 `0` がデポで、顧客は `1` から `n` です。座標と需要、距離行列、または CVRPLIB のファイルから
作ります。複数の車種からなる車両は[混成車両](#vrp-mixed-fleet)で説明します。解は車両ごとに 1 つの
経路を持ち、両端のデポは省略され、空の経路もありえます。`evaluate_routes` は距離そのもの、超過量、
各経路の積載量を報告します。

`num_vehicles` は車両の台数で、どのヒューリスティックもちょうどその数の経路を返します。使わない
車両は空の経路になります。そのため、需要に必要な台数より多く設定しても問題はなく、予備の車両は
探索に顧客を動かす余地を与えます。`num_vehicles=0` なら optopus が台数を決めます: First-Fit
Decreasing で需要を詰めた台数に、その 10% (切り捨て、最低 1 台) の予備を足した数です。結果は
`num_vehicles()` で読み出せます。需要に対して台数が少なすぎる場合や、容量を超える需要を持つ顧客が
いる場合もエラーにはなりません。探索は `overload > 0` の最良解を返すので、経路を使う前に超過量を
確認してください。

`evaluate_routes` は車両ごとに 1 つ、空のものも含めて `num_vehicles()` 個の経路を受け取り、それ
以外では `ValueError` になります。そのため、車両の台数より多くの車両を必要とする計画は評価されず
拒否されます。

`HybridGeneticSearch` と `AdaptiveLargeNeighborhoodSearch` は両方の種類の移動を自分で扱います。
汎用ヒューリスティックでは組み合わせる必要があります。`"Relocate"` と `"Swap"` は経路の間で顧客を
動かすだけで、`"TwoOpt"` は 1 つの経路を並べ替えるだけなので、どれか 1 つだけを使う
`SimulatedAnnealing` は行き詰まります。3 つすべてで仕上げる可変近傍探索の例です。

```python
import random
import optopus

rng = random.Random(7)
coords = [(0, 0)] + [(rng.randint(-50, 50), rng.randint(-50, 50)) for _ in range(30)]
demands = [0] + [rng.randint(1, 9) for _ in range(30)]
vrp = optopus.Vrp.from_coordinates(coords, demands, capacity=40, num_vehicles=6)

# 1 ラウンドで各移動を局所最適まで実行する。各 LocalSearch は改善しない反復 1 回で
# 終わるので、失敗した反復が 3 回続けば、そのラウンドは何も見つけなかったことになる。
polish = optopus.Sequential(
    [optopus.LocalSearch(nb, stop=optopus.StopCondition(max_iteration=1_000))
     for nb in ("Relocate", "Swap", "TwoOpt")],
    stop=optopus.StopCondition(max_failed_update=3),
)
vns = optopus.VariableNeighborhoodSearch(
    search=polish,
    shakes=[optopus.RandomWalk("Relocate", stop=optopus.StopCondition(max_iteration=k))
            for k in (2, 4, 8)],
    stop=optopus.StopCondition(max_duration_secs=0.25),
)
report = vns.run(vrp, runs=3, seed=42)
print(report.best_objective)  # ≈ 606.12。HybridGeneticSearch も到達する最適値
```

`polish` の `max_failed_update` を大きくすると、すでに収束したラウンドを繰り返して予算を使い切って
しまいます。

距離行列は `Tsp` と同じく対称・有限・非負でなければならず、`matrix[i][j]` はノード `i` と `j` の
距離です。需要は負であってはならず、デポの需要は無視されます。

(vrp-mixed-fleet)=
#### 混成車両

`Vrp.with_fleet` は複数の車種で回るインスタンスを作ります。`VehicleType` は容量、台数の上限
`max_count`、速度、使った車両 1 台ごとに 1 回かかる `fixed_cost`、`variable_cost_per_distance`、
必ず使わなければならない台数 `min_count`、`max_route_time` を持ちます。顧客にはサービス時間を
設定できます。車両は全車種の `max_count` を合計した `num_vehicles()` 個の経路を持ち、車種を与えた
順に車種ごとに並びます。どの解でも、経路 `i` は車種 `slot_types()[i]` が走ります。異なる車種の経路
の間で顧客を動かすと担当する車種が変わるので、同じ近傍も `HybridGeneticSearch` も
`AdaptiveLargeNeighborhoodSearch` も、経路を決めながら車両の構成も選びます。

目的関数は次のとおりです。

```text
time + cost_weight · cost + penalty_weight · (overload + time_excess + min_count_shortfall)
```

経路にかかる時間は `distance / speed` にその経路のサービス時間を足したものです。`time` は
`objective_mode="TotalTime"` (既定) ではそれらの経路時間の合計、`"Makespan"` では最長のものです。
`cost` は使った全車両の固定費と全経路の距離比例のコストを足したもので、`cost_weight` (既定 1.0) が
時間との交換比率を決めます。容量、経路時間の上限、最小台数はどれも同じように、目的関数がそれ以外に
課しうるすべてを上回る重みでペナルティを課します。速度 1 でコストもサービス時間もない車種が 1 つ
だけなら、上の CVRP とまったく同じです。

```python
import random
import optopus

rng = random.Random(3)
coords = [(0, 0)] + [(rng.randint(-30, 30), rng.randint(-30, 30)) for _ in range(20)]
demands = [0] + [rng.randint(1, 5) for _ in range(20)]
service = [0] + [2.0] * 20

truck = optopus.VehicleType("truck", capacity=30, max_count=2, fixed_cost=50.0)
van = optopus.VehicleType(
    "van", capacity=10, max_count=4, speed=1.5, fixed_cost=10.0, max_route_time=90.0
)
vrp = optopus.Vrp.with_fleet(coords, demands, [truck, van], service_times=service)

report = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=500)).run(
    vrp, runs=3, seed=42
)
best = min(report.runs, key=lambda r: r.best_objective)
result = vrp.evaluate_routes(best.solution)
print(round(result["objective"], 2), result["used_count"])  # 374.15 [1, 3]
for route, t, time in zip(best.solution, vrp.slot_types(), result["route_times"]):
    if route:
        print(vrp.vehicle_types()[t].name, route, round(time, 1))
```

最良の計画はトラック 1 台とバン 3 台を使います。トラックはバンには走れない最長の経路 (137.7 時間
単位) を走り、各バンは上限の 90 以内に収まっています。`evaluate_routes` は目的関数の各項を個別に
報告します: CVRP の `distance`、`overload`、`route_loads` に加えて、`route_distances`、
`route_times`、車種ごとの `used_count`、`total_time`、`makespan`、`time_excess`、
`min_count_shortfall`、`total_cost` です。`overload`、`time_excess`、`min_count_shortfall` がすべて 0
なら解は実行可能です。`capacity()` は複数の車種からなる車両では `ValueError` になるので、各車種の
容量は `vehicle_types()` から読んでください。`with_objective_mode("Makespan")` は、代わりに最長の
経路で評価するコピーを返します。

`Vrp.load_file` は optopus 独自の形式の `.toml` ファイルから混成車両を読み込みます。それ以外の
ファイルは CVRPLIB として読みます。

```toml
name = "demo"                  # 省略可。既定はファイル名
objective_mode = "TotalTime"   # または "Makespan"
cost_weight = 1.0              # 省略可
rounded = false                # 省略可。CVRPLIB と同じように距離を丸める

[depot]
x = 0.0
y = 0.0

[[vehicle_types]]              # 車種ごとに 1 テーブル。キーは VehicleType の引数と同じ
name = "truck"
capacity = 30
speed = 1.0
fixed_cost = 50.0
max_count = 2

[[customers]]                  # 顧客ごとに 1 テーブル。id はちょうど 1..n
id = 1
x = 3.0
y = 4.0
demand = 2
service_time = 2.0             # 省略可
```

### GraphColoring

どの辺も同じ色の 2 頂点を結ばないように頂点を塗り、使う色の数をできるだけ少なくします。

| コンストラクタ | 作るもの |
|---|---|
| `GraphColoring.from_edges(edges, num_colors=None)` | `(u, v, weight)` の辺からなるインスタンス。重みは無視される |
| `GraphColoring.from_graph(graph, num_colors=None)` | [`Graph`](graphs_and_instances.md#graph) 上のインスタンス |
| `GraphColoring.load_file(path, num_colors=None)` | `MaxCut.load_file` と同じ形式の辺リストのファイル。重みは無視される |

| メソッド | 返すもの |
|---|---|
| `num_colors()` | パレットの大きさ。上で `None` なら `max_degree + 1` になり、正しい彩色が必ず存在する |
| `penalty_weight()` | 衝突する辺 1 本あたりのペナルティ、`num_vertices + 1` |
| `graph()` | 元になるグラフのコピー |
| `evaluate_colors(colors)` | `objective`、`colors_used`、`conflicts` を持つ辞書 |

```python
# ピーターセングラフ: 外側の 5 頂点の閉路、スポーク、内側の五芒星。3 色が必要。
petersen = (
    [(i, (i + 1) % 5, 1.0) for i in range(5)]
    + [(i, i + 5, 1.0) for i in range(5)]
    + [(5 + i, 5 + (i + 2) % 5, 1.0) for i in range(5)]
)
gc = optopus.GraphColoring.from_edges(petersen)
print(gc.num_colors(), gc.penalty_weight())  # 4 11

ts = optopus.TabuSearch("Recolor", (2, 5), stop=optopus.StopCondition(max_iteration=2_000))
best = ts.run(gc, runs=3, seed=42).runs[0]
print(gc.evaluate_colors(best.solution))
# {'objective': 3, 'colors_used': 3, 'conflicts': 0}
```

報告された値が `penalty_weight()` より小さければ彩色は正しく、その値は使った色の数に等しくなります。
探索はすぐに正しい彩色に達しますが、色を減らすのは遅いと考えてください。ある色のクラスを空にするには
その色の頂点を 1 つずつすべて移す必要があり、最後の 1 つが抜けるまで何の得にもならないからです。
`k` 色で足りるかを調べるには、`num_colors=k` を渡して衝突が 0 になるか確認します。

```python
two = optopus.GraphColoring.from_edges(petersen, num_colors=2)
best = ts.run(two, runs=3, seed=42).runs[0]
print(two.evaluate_colors(best.solution)["conflicts"])  # 3 — 2 色では足りない
```

### Qap

2 次割当て問題です。`n` 個の施設を `n` 個の場所に 1 つずつ配置し、割当て `p` について
`Σ_i Σ_j flow[i][j] · distance[p[i]][p[j]]` を最小化します。

| コンストラクタ | 作るもの |
|---|---|
| `Qap.from_matrices(flow, distance, name="")` | 2 つの `n × n` 整数行列からなるインスタンス。どちらも対称でなくてよい |
| `Qap.load_file(path)` | QAPLIB のインスタンス: `n` のあとに流量行列と距離行列 |

| メソッド | 返すもの |
|---|---|
| `cost(assignment)` | 割当てのコスト。`assignment[facility]` がその施設の場所 |
| `size()` | `n` |

```python
flow = [[0, 3, 0, 2], [3, 0, 0, 1], [0, 0, 0, 4], [2, 1, 4, 0]]
distance = [[0, 22, 53, 53], [22, 0, 40, 62], [53, 40, 0, 55], [53, 62, 55, 0]]
qap = optopus.Qap.from_matrices(flow, distance)

ts = optopus.TabuSearch("Swap", (1, 2), stop=optopus.StopCondition(max_iteration=50))
best = ts.run(qap, runs=3, seed=42).runs[0]
print(best.best_objective, best.solution)  # 790.0 [2, 3, 0, 1]
print(qap.cost(best.solution))             # 790 — 全 24 通りの割当てでの最適値
```

`"Swap"` は `O(n)` で評価されます。`TabuSearch` は、両方の施設を最近離れた場所に戻してしまう交換を
禁止します。これは Taillard の robust tabu search の規則で、そのため `TabuSearch` が定番の選択です。

### BinPacking

同じ容量の箱に品物を詰め、使う箱の数をできるだけ少なくします。

| コンストラクタ | 作るもの |
|---|---|
| `BinPacking.from_sizes(capacity, sizes, name="")` | インスタンス。箱より大きい品物は `ValueError` |
| `BinPacking.load_file(path)` | `n capacity` のあとに `n` 個の大きさ。Falkenauer のインスタンスの形式 |

| メソッド | 返すもの |
|---|---|
| `evaluate_bins(bins)` | 解についての `objective`、`num_bins`、`loads`、`bins` を持つ辞書 |
| `lower_bound()` | `⌈Σ size / capacity⌉`。これに達する詰め方は最適 |
| `first_fit(order=None)` | 品物を順に、入る最初の箱へ入れたときの各品物の箱。サイズの大きい順にすると First Fit Decreasing |
| `capacity()`、`sizes()`、`num_items()` | インスタンスのデータ |

解は各品物の箱を 0 からの番号で与えます。探索が通るどの詰め方も容量に収まっています。

```python
bins = optopus.BinPacking.from_sizes(10, [6, 4, 5, 5, 7, 3, 2, 8, 3, 7])
print(bins.lower_bound())  # 5

alns = optopus.AdaptiveLargeNeighborhoodSearch(
    stop=optopus.StopCondition(max_iteration=500), removal_fraction=0.3
)
best = alns.run(bins, runs=3, seed=42).runs[0]
print(bins.evaluate_bins(best.solution))
# {'objective': 4.0, 'num_bins': 5, 'loads': [10, 10, 10, 10, 10], 'bins': [0, 0, 1, 1, 2, 3, 4, 4, 2, 3]}
```

箱の数だけで比べるとほとんどの移動が同点になるので、探索は詰め方を
`num_bins − mean((load / capacity)²)` で順位付けします。第 2 項は `(0, 1]` に収まるので、箱が少ない
ほうが常に勝ち、箱の数が同じ詰め方の中では、箱を空にしやすい、よく詰まったもののほうが上位になります。
上の例では、満杯の箱 5 つで `5 − 1 = 4.0` です。

`"Relocate"` による降下は、一連の移動を重ねないと箱を空にできない詰め方で行き詰まります。詰め方を
作り直す `AdaptiveLargeNeighborhoodSearch` と `GreedyRandomizedConstruction` は、そこを越えられます。

### Formula

有界な整数変数上の多項式と、ペナルティで重み付けした制約です。専用のページがあります:
[Formula によるモデル化](formula.md)。

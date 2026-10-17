---
source_sha256: c04ba17d00af
---

# 問題専用ヒューリスティック

これらのヒューリスティックは特定の問題の構造をもとに作られているので、`neighbor` 引数を取らず、
合う問題ではたいてい汎用ヒューリスティックに勝ります。他の問題で実行すると、`run` を呼んだときに
`ValueError` になります。

| ヒューリスティック | 動く問題 |
|---|---|
| [`WalkSat`](#walksat) | `Sat` |
| [`BreakoutLocalSearch`](#breakoutlocalsearch) | `MaxCut`。`from_parts` ならどの問題でも作れる |
| [`LinKernighanHelsgaun`](#linkernighanhelsgaun) | `Tsp` |
| [`AdaptiveLargeNeighborhoodSearch`](#adaptivelargeneighborhoodsearch) | `Vrp`、`Tsp`、`FlowShop`、`BinPacking`、ruin 系メソッドを持つ Python の問題 |
| [`IteratedGreedy`](#iteratedgreedy) | `FlowShop`、`Tsp`、`Vrp`、`BinPacking`、ruin メソッドを持つ Python の問題 |
| [`GreedyRandomizedConstruction`](#greedyrandomizedconstruction) | `Vrp`、`Tsp`、`FlowShop`、`BinPacking`、ruin 系メソッドを持つ Python の問題 |
| [`HybridGeneticSearch`](#hybridgeneticsearch) | `Vrp` |

既定値は論文で公表された設定で、出発点として妥当です。どれも `StopCondition` を受け取り、他と同じ
`run(problem, runs=1, seed=None)` を持ちます。[探索の実行と結果の読み方](running.md)を参照して
ください。

## WalkSat

MaxSAT 向けの WalkSAT/SKC です。各反復で充足されていない節をランダムに選び、その変数の 1 つを反転
します。確率 `noise` でランダムな変数を、それ以外では反転によって破れる充足済みの節が最も少ない変数を
選びます。何も破らない反転は必ず採用します。すべての節が成り立った時点で実行は終わります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `stop` | | `StopCondition` |
| `noise` | `0.3` | ランダムな反転の確率。`[0, 1]` |
| `adaptive` | `False` | Hoos (2002) に従い、実行中に `noise` を調整する |

```python
import optopus

# (x1 ∨ x2) ∧ (¬x1 ∨ x3) ∧ (¬x2 ∨ ¬x3) ∧ (x3 ∨ x4) ∧ (¬x4 ∨ x1) ∧ (¬x3 ∨ ¬x4 ∨ x2)
sat = optopus.Sat.from_clauses(4, [[1, 2], [-1, 3], [-2, -3], [3, 4], [-4, 1], [-3, -4, 2]])

walksat = optopus.WalkSat(stop=optopus.StopCondition(max_iteration=10_000))
report = walksat.run(sat, runs=3, seed=42)
print(report.best_objective)    # 6.0 — 6 つの節がすべて成り立つ
print(report.runs[0].solution)  # [True, False, True, False] — x1 から x4
```

目的関数は充足される節を数えるので、充足可能なインスタンスはそれが節の数に等しくなれば解けています。
`adaptive=True` にすれば、新しい種類のインスタンスで `noise` を調整する手間が省けます。

## BreakoutLocalSearch

Benlic と Hao による Max Cut 向けの breakout local search です。各ラウンドで局所最適まで降下し、
それを摂動します。摂動はタブーの記憶を読む directed な移動 (反転と交換) か、ランダムな反転です。
最良解の更新がない期間が長くなるほど、摂動は強く、ランダムになります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `tabu_tenure` | | `(min, max)`。禁止期間で、意味は `TabuSearch` と同じ |
| `t` | | ランダムな摂動に切り替わるまでの、最良解の更新がないラウンドの数 |
| `l0` | | 摂動の長さ (移動の数) の初期値。`>= 1` |
| `p0` | | directed な摂動の確率の下限。`[0, 1]` |
| `q` | | directed な摂動のうち、交換ではなく反転する割合。`[0, 1]` |
| `stop` | | `StopCondition` |

```python
mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
bls = optopus.BreakoutLocalSearch(
    tabu_tenure=(6, 12), t=1_000, l0=3, p0=0.8, q=0.5,
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(bls.run(mc, runs=3, seed=42).best_objective)  # 93.0、最適値
```

どの引数も既定値を持ちません。G-set での論文の設定は `t=1000`、`l0` は約 `|V| / 100`、`p0=0.8`、
`q=0.5` で、γ は `rand[3, |V|/10]` から引きます。γ はアルゴリズム内部で 2 回数えられるので、ここでは
`tabu_tenure=(6, len(vertices) // 5)` になります。
[`BreakoutLocalSearch.from_parts`](heuristics_composed.md#breakoutlocalsearchfrom_parts) は同じ
枠組みを他のヒューリスティックから組み立て、どの問題にも使えます。

## LinKernighanHelsgaun

対称 TSP 向けの Lin-Kernighan-Helsgaun です。各反復で、各都市の最も近い `num_neighbors` 都市の中から、
一度に最大 `max_depth` 本の辺を入れ替える改善的な可変深さの交換を探し、最初に見つかったものを適用
します。そうした交換が存在しない局所最適で実行は終わります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `stop` | | `StopCondition`。局所最適でそれより早く終わることがある |
| `num_neighbors` | `5` | 都市ごとに保持する候補都市の数 |
| `max_depth` | `5` | 1 回の交換で入れ替える辺の最大数 |

```python
import math

# 単位円上の 12 都市を、順不同で並べたもの。
order = [0, 5, 10, 3, 8, 1, 6, 11, 4, 9, 2, 7]
tsp = optopus.Tsp.from_coordinates(
    [(math.cos(2 * math.pi * k / 12), math.sin(2 * math.pi * k / 12)) for k in order]
)

lkh = optopus.LinKernighanHelsgaun(stop=optopus.StopCondition(max_iteration=1_000))
report = lkh.run(tsp, runs=3, seed=42)
print(round(report.best_objective, 4))         # 6.2117 — 正 12 角形
print(round(24 * math.sin(math.pi / 12), 4))   # 6.2117
```

局所最適で実行が終わるので、2 回以上降下させるには `Iterated` か `Restart` の中に入れるか、`runs` を
使ってください。

## AdaptiveLargeNeighborhoodSearch

ruin and recreate です。各反復で、3 つの破壊演算子の 1 つで解の一部を取り出し、2 つの修復演算子の
1 つで戻し、その結果を焼きなまし法の規則で受理するか決めます。成果を上げてきた演算子の組ほど頻繁に
選ばれます。

| 破壊 | 取り出すもの |
|---|---|
| Random | 一様に選んだ要素 |
| Worst | 取り出すと最も節約できる要素。最も長い寄り道など |
| Shaw | ランダムな 1 つに最も関連の深い要素。近くの顧客など |

| 修復 | 先に戻すもの |
|---|---|
| Greedy | 最も安い場所のコストが最小の要素を、その場所に |
| Regret-2 | 最良の入れ物を逃すと最も多くを失う要素 |

`Vrp`、`Tsp`、`FlowShop` と、`repair_around` を持つ Python の問題では、修復のたびに、戻したばかりの
要素の周りでの降下が続きます。`local_repair=False` にすると降下を省きます。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `stop` | | `StopCondition`。1 反復は 1 回の ruin and recreate |
| `removal_fraction` | `0.15` | 反復ごとに取り出す、`n` 個の要素に対する割合。`(0, 1]` |
| `cooling_rate` | `0.9995` | 反復ごとに受理の温度に掛ける係数。`(0, 1]` |
| `scores` | `(4.0, 2.0, 1.0)` | 演算子の組が、新しい最良解、現在の解の改善、受理された悪化のそれぞれで得る得点。比だけが意味を持つ |
| `segment_length` | `100` | 演算子の重みを更新する間隔の反復数。`>= 1` |
| `reaction` | `0.1` | 区間の平均得点を重みにどれだけ混ぜるか。`[0, 1]` |
| `max_removal` | `50` | 1 反復で取り出す要素の上限。`>= 1` |
| `local_repair` | `True` | 修復のたびに降下を続けるか |

```python
bins = optopus.BinPacking.from_sizes(10, [6, 4, 5, 5, 7, 3, 2, 8, 3, 7])
alns = optopus.AdaptiveLargeNeighborhoodSearch(
    stop=optopus.StopCondition(max_iteration=500), removal_fraction=0.3
)
best = alns.run(bins, runs=3, seed=42).runs[0]
print(bins.evaluate_bins(best.solution)["num_bins"], bins.lower_bound())  # 5 5 — 最適
```

- 取り出すのは `round(removal_fraction · n)` 個で、最低 1 個、最大で `n − 1` 個かつ `max_removal` 個です。小さな
  インスタンスでは既定値だと 1 回に 1 要素しか取り出さず、ruin and recreate ではそれでは改善でき
  ません。上の例のように値を上げて、複数の要素が一緒に動くようにしてください。大きなインスタンスでは、
  修復は取り出した要素ごとにすべての場所を評価するので、割合を小さくすると 1 反復が安くなり、同じ
  時間でより多くの反復を実行できます。
- 受理の温度は、初期解より 5% 悪い解が半分の確率で受理される値から始まります。既定の
  `cooling_rate` では数千反復の間ほとんど冷えません。時間予算が決まっている場合は、短い実行の反復数を
  測り、予算の間に温度が 2〜3 桁下がるように `cooling_rate` を選んでください:
  `cooling_rate = 0.001 ** (1 / iterations)`。
- [Python で書いた問題](#ruin-methods)も、ruin 系メソッドを定義すれば実行できます。

## IteratedGreedy

順列フローショップ向けに提案された iterated greedy です。各反復で `removal_count` 個の要素をランダムに
取り出し、それぞれを目的関数の増加が最も小さい位置に戻し、その周りで降下します。結果が悪くなければ
現在の解と置き換え、そうでなければ一定の温度のもとで確率 `exp(−Δ / temperature)` で置き換えます。
Taillard のフローショップのインスタンスでは `AdaptiveLargeNeighborhoodSearch` に勝ります。

`AdaptiveLargeNeighborhoodSearch` が動くすべての問題で、同じ降下とともに動きます。`FlowShop` では挿入
近傍、`Tsp` と `Vrp` では戻した要素の周りの降下、Python の問題では `repair_around` で、`BinPacking` には
降下がありません。フローショップ以外では、温度を目的関数の単位で `temperature` に渡してください。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `stop` | | `StopCondition`。1 反復は 1 回の破壊と再構築、およびその降下 |
| `removal_count` | `4` | 反復ごとに取り出すジョブの数。`>= 1` |
| `temperature_factor` | `0.4` | `FlowShop` での温度は、処理時間の平均の 1/10 にこの値を掛けたもの。正で有限 |
| `temperature` | `None` | 温度そのもの。目的関数の単位。`FlowShop` 以外では必須で、`FlowShop` では上の温度の代わりになる |
| `local_repair` | `True` | 再構築のたびに降下を続けるか |

```python
# 3 機械 × 5 ジョブ: times[machine][job]。
shop = optopus.FlowShop.from_processing_times(
    [[5, 9, 8, 10, 1], [9, 3, 10, 1, 8], [9, 4, 5, 8, 6]]
)
print(shop.makespan(shop.neh()))  # 45 — NEH 構築法。標準的な基準解

ig = optopus.IteratedGreedy(stop=optopus.StopCondition(max_iteration=200), removal_count=2)
report = ig.run(shop, runs=3, seed=42)
print(report.best_objective)     # 44.0 — 全 120 通りの順序での最適値
```

`removal_count=4, temperature_factor=0.4` は Ruiz と Stützle の調整済みの設定です。ジョブが数個しか
ないなら、取り出す数を減らしてください。

## GreedyRandomizedConstruction

GRASP の構築ステップです。1 回の呼び出しで全要素を取り出し、何もないところから 1 要素ずつ新しい解を
作ります。各ステップで、残っている要素それぞれを最も安い場所で評価し、最安値から `alpha` の範囲内に
あるものから 1 つ選んで、それが最も安い場所に置きます。これは探索ではなく単一のステップで、1 回の
呼び出しでちょうど 1 つの解を作ります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `alpha` | `0.2` | 候補リストの幅。`[0, 1]`。`0` は純粋な貪欲法、`1` はランダムな順序 |
| `stop` | `None` | 呼び出しを構築の前に終わらせることはできるが、より多く作らせることはできない |

GRASP は、この構築を [`Iterated`](heuristics_composed.md#iterated) の摂動にし、降下を探索にして、
降下の結果のうち最良のものを残すものです。

```python
grasp = optopus.Iterated(
    search=optopus.LocalSearch("TwoOpt", stop=optopus.StopCondition(max_iteration=1_000)),
    perturbation=optopus.GreedyRandomizedConstruction(alpha=0.2),
    stop=optopus.StopCondition(max_iteration=2_000),
)
print(round(grasp.run(tsp, runs=3, seed=42).best_objective, 4))  # 6.2117
```

単独で実行すると、1 回の呼び出しは 1 つの構築を返し、手早い出発点になります。

```python
greedy = optopus.GreedyRandomizedConstruction(alpha=0.0).run(tsp, seed=42).runs[0]
print(greedy.iterations, round(greedy.best_objective, 4))  # 1 6.2117
```

1 回の構築は各ステップで残りの要素をすべて評価し、`n` 個の要素に対して `O(n²)` 回の配置を行うので、
数千個では遅くなります。

## HybridGeneticSearch

Vidal による配送計画問題向けのハイブリッド遺伝的探索です。各個体は全顧客の巨大な巡回路で、厳密な
動的計画法で経路に分割されます。各反復で 2 つの親を交叉し、子を分割し、各顧客の最も近い
`granularity` 人を対象とする局所探索で改善して、集団に加えます。実行不可能な個体も容量のペナルティの
もとで生き残り、そのペナルティは子の約 `target_feasible` が実行可能になるように適応します。生き残る
個体はコストと多様性で選ばれます。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `stop` | | `StopCondition`。1 反復は子 1 つ |
| `min_population_size` | `25` | μ。生存者選択で切り詰める集団の大きさ。`>= 4` |
| `generation_size` | `40` | λ。2 回の切り詰めの間に作る子の数。`>= 1` |
| `granularity` | `20` | 局所探索の各移動が考慮する最近傍の数。`>= 1` |
| `target_feasible` | `0.2` | ペナルティが目指す実行可能な子の割合。`(0, 1)` |
| `restart_generations` | `20000` | 改善のないままこの世代数が過ぎると集団をやり直す。`None` ならやり直さない |
| `n_elite` | `4` | 生存選択がコストだけで残す個体の数。`>= 1` |
| `n_closest` | `5` | 多様性を平均する近い個体の数。`>= 1` |
| `penalty_adaptation` | `(100, 1.2, 0.85, 0.05)` | `(period, increase, decrease, tolerance)`。子 `period` 個ごとに、実行可能な割合が目標より `tolerance` 以上低ければペナルティに `increase` を、それだけ高ければ `decrease` を掛ける |
| `repair_probability` | `0.5` | 実行不能な子を強めたペナルティでも修復する確率。`[0, 1]` |
| `initial_population` | `(4, 4)` | `(factor, nearest_neighbor_every)`。初期個体は `factor · min_population_size` 個で、`nearest_neighbor_every` 個に 1 個を最近傍法で作る |
| `descent_passes` | `64` | 1 個の子に局所探索が使える反復の回数。`>= 1` |

```python
vrp = optopus.Vrp.from_coordinates(
    coordinates=[(0.0, 0.0), (10.0, 0.0), (11.0, 0.0), (-10.0, 0.0), (-11.0, 0.0)],
    demands=[0, 5, 5, 5, 5],
    capacity=15,
    num_vehicles=2,
)
hgs = optopus.HybridGeneticSearch(stop=optopus.StopCondition(max_iteration=200))
report = hgs.run(vrp, runs=3, seed=42)
print(report.best_objective)  # 44.0 — 各クラスタに 1 台ずつ
```

実行はまず `4 · min_population_size` 個の個体を作り、それぞれを分割・改善して、その数の反復として
数えます。`max_iteration=1` の実行でもそれらは作られます。大きな `Vrp` を短い時間制限で解くときは、
`min_population_size` を下げてください。リリースビルドでは、顧客 200 人の `Vrp` で 1 秒あたり約 600 の
子を、顧客 30 人ではその 10 倍を作ります。数十秒の同じ予算では、`Vrp` での `HybridGeneticSearch` と
`AdaptiveLargeNeighborhoodSearch` の成績は互角なので、両方を試してください。混成車両も扱えます。
{ref}`混成車両 <vrp-mixed-fleet>`を参照してください。

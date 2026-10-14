---
source_sha256: af0830283f56
---

# 探索の実行と結果の読み方

どのヒューリスティックも使い方は同じです。`StopCondition` を渡して作り、
`run(problem, runs=1, seed=None)` を呼び、返ってくる `RunReport` を読みます。このページでは
この 3 つを説明します。どのヒューリスティックを作るかは
[Choosing a heuristic](https://trash-iine.github.io/optopus-py/heuristics.html) (英語) にあります。

## `run`

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    stop=optopus.StopCondition(max_iteration=10_000),
)

report = sa.run(mc, runs=4, seed=7)
print(report)
# RunReport(num_runs=4, best_objective=93, avg_objective=93, worst_objective=93, std_objective=0)
```

| 引数 | 既定値 | 意味 |
|---|---|---|
| `problem` | | 組み込みの問題、`Formula`、または [Python で書いた問題](https://trash-iine.github.io/optopus-py/python_problems.html) (英語) |
| `runs` | `1` | 独立な実行の回数。各実行はそれぞれのランダムな初期解から始まる。`0` は `ValueError` |
| `seed` | `None` | マスターシード。実行 0 はこれをそのまま使い、残りの実行はこれから自分のシードを導く。そのため同じ呼び出しは同じレポートを再現する。`None` なら呼び出しごとに新しいシードを取る |

- 実行は**1 つずつ順に**行われます。1 実行あたり `max_duration_secs=d` の予算なら、呼び出し全体は
  約 `runs × d` かかります。
- 組み込みの問題は GIL を解放して実行するので、その間も他の Python スレッドは動き続けます。
  Python で書いた問題は、ステップごとに Python を呼び出します。
- ヒューリスティックのインスタンスが持つのは設定だけです。問題や呼び出しをまたいで再利用できます。
- 問題に合わないヒューリスティック (問題にない `neighbor` や、`MaxCut` に対する `WalkSat` など) は、
  作ったときではなく `run` を呼んだときに `ValueError` になります。

同じシードで 2 回呼ぶと同じ結果になります。

```python
a = sa.run(mc, runs=3, seed=99)
b = sa.run(mc, runs=3, seed=99)
assert [r.solution for r in a.runs] == [r.solution for r in b.runs]
```

これは反復回数で止める場合の話です。時間制限については[停止条件](#stopping)を見てください。

(stopping)=
## 停止条件

`StopCondition` は、いずれかの上限に達した時点で実行を止めます。

| 引数 | 既定値 | 止まるタイミング |
|---|---|---|
| `max_iteration` | `None` | この回数だけ反復したあと |
| `max_duration_secs` | `None` | この秒数の実時間が経ったあと。負の値や無限大は `ValueError` |
| `max_failed_update` | `None` | 最良解が更新されない反復がこの回数だけ続いたあと |

```python
optopus.StopCondition(max_iteration=10_000)
optopus.StopCondition(max_duration_secs=0.5)
optopus.StopCondition(max_iteration=100_000, max_failed_update=2_000)  # 先に達したほうで止まる
```

少なくとも 1 つは指定してください。何も指定しない `StopCondition()` も受け付けますが、自分では
止まらないヒューリスティックはそのまま永遠に走り続けます。自分で止まるのは、局所最適で止まる
`LocalSearch` と `LinKernighanHelsgaun`、すべての節が充足された時点で止まる `WalkSat`、1 回の構築で
止まる `GreedyRandomizedConstruction` です。それ以外は停止条件が満たされるまで走ります。

小さな問題では数ミリ秒で最良値に達することが多く、大きな `max_iteration` はほとんどの時間を答えが
見つかったあとに費やします。`max_failed_update` は探索が停滞した時点で実行を止めるので、探索に
どれだけ必要か分からないうちの最初の予算として適しています。固定の予算は、あとでレポートの
`time_to_best_secs` を見て決めます。その際はヒューリスティックの動き方に合わせてください。まだ
温度の高い焼きなまし法はめったに最良値を更新しないので、小さな `max_failed_update` では冷える前に
止まってしまいます。

```python
early = optopus.SimulatedAnnealing(
    "Flip", 2.0, 0.999, optopus.StopCondition(max_failed_update=500)
).run(mc, seed=7)
run = early.runs[0]
print(run.iterations, run.best_objective)  # 746 89.0 — 冷えるずっと前に停止
```

1 反復が何を指すかはヒューリスティックによって違うので、同じ `max_iteration` でも仕事量は大きく
異なります。

| ヒューリスティック | 1 反復 |
|---|---|
| `LocalSearch`、`TabuSearch` | 近傍**全体**を評価し、1 つの移動を適用する |
| `SimulatedAnnealing`、`BangBangSimulatedAnnealing`、`LateAcceptanceHillClimbing`、`RandomWalk` | ランダムな移動を 1 つ評価する |
| `ReinforcementLearningSearch` | 近傍全体、またはそのうち `max_candidates` 個の移動を評価し、1 つを適用する |
| `BeamSearch` | ビーム内のすべての候補について近傍全体を展開する |
| `PopulationAnnealing` | 全レプリカを 1 スイープする。`sweeps_per_step` スイープからなる温度ステップはその回数として数える |
| `ParallelTempering` | 全レプリカを 1 スイープする。`sweeps_per_exchange` スイープと 1 回の交換からなるラウンドはその回数として数える |
| `GeneticAlgorithm`、`HybridGeneticSearch` | 子を 1 つ作る |
| `AdaptiveLargeNeighborhoodSearch`、`IteratedGreedy` | 解を 1 回壊して作り直す |
| `GreedyRandomizedConstruction` | 解を 1 つ構築する。1 回の呼び出しでちょうど 1 つ作る |
| `WalkSat` | 変数を 1 つ反転する |
| `LinKernighanHelsgaun` | 改善する可変深さの交換を 1 回適用する |
| `BreakoutLocalSearch` | 降下または摂動の 1 手 |
| `Sequential`、`Iterated`、`VariableNeighborhoodSearch`、`Restart` | 各ステップの反復を数える。[How the outer stop counts](https://trash-iine.github.io/optopus-py/heuristics_composed.html#how-the-outer-stop-counts) (英語) を参照 |
| `BranchAndBound` | ノードを 1 つ取り出す。そこで `heuristic` が行う反復は別に数える。[Exact search](https://trash-iine.github.io/optopus-py/heuristics_exact.html) (英語) を参照 |

上限は仕事の単位と単位の間で確認され、単位の途中では確認されません。そのため、ヒューリスティックの
最小単位は必ず最後まで実行され、実行は上限を最大 1 単位ぶん超えることがあります。ほとんどの
ヒューリスティックでは単位は 1 反復です。`PopulationAnnealing` では温度ステップが単位なので、1 実行は
少なくとも `sweeps_per_step` 反復します。合成ヒューリスティックではステップ 1 回の呼び出しが単位です。
`HybridGeneticSearch` では最初の単位が初期集団全体です。
[HybridGeneticSearch](https://trash-iine.github.io/optopus-py/heuristics_specific.html#hybridgeneticsearch)
(英語) を参照してください。

時間制限で実行できる反復回数は、マシンとビルドによって変わります。既定の開発ビルドは
`maturin develop --release` やインストールした wheel より数倍遅いので、時間を測るときはそちらを
使ってください。ヒューリスティックどうしを比べるときは時間制限が自然な単位ですが、実行をマシンに
よらず正確に再現できるのは反復回数の上限だけです。

(the-report)=
## レポート

`run` は `RunReport` を返します。`report.runs` には実行ごとの `RunResult` が入り、それらの統計も
持っています。属性はすべて読み取り専用です。

| `RunReport` の属性 | 意味 |
|---|---|
| `runs` | 全実行の `RunResult`。実行順 |
| `num_runs` | 実行の回数 |
| `best_objective`、`worst_objective` | 全実行の `best_objective` のうち最良と最悪。問題の最適化の向きに従う |
| `avg_objective`、`std_objective` | `best_objective` の平均と母標準偏差 |
| `best_time_to_best_secs`、`avg_time_to_best_secs` | `time_to_best_secs` の最小値と平均 |
| `avg_total_time_secs` | `total_time_secs` の平均 |
| `avg_initial_objective`、`avg_improvement` | `initial_objective` と `improvement` の平均 |
| `avg_iterations`、`avg_n_accepted`、`avg_n_rejected`、`avg_n_best_updates` | 実行ごとのカウンタの平均 |

| `RunResult` の属性 | 意味 |
|---|---|
| `solution` | その実行の最良解。問題ごとの形は [Built-in problems](https://trash-iine.github.io/optopus-py/problems.html#at-a-glance) (英語) にある |
| `best_objective` | その目的関数値。探索が順位付けに使う尺度で表す。[What a run reports](https://trash-iine.github.io/optopus-py/problems.html#what-a-run-reports) (英語) を参照 |
| `initial_objective` | ランダムな初期解の目的関数値 |
| `improvement` | `initial_objective` から `best_objective` への改善量。最適化の向きによらず、良くなれば正 |
| `best_iteration` | 最良解が見つかった反復 |
| `iterations` | その実行の反復回数。単位はヒューリスティックによる (上の表を参照) |
| `time_to_best_secs`、`total_time_secs` | 最良解までの秒数と、実行全体の秒数 |
| `n_accepted`、`n_rejected` | 適用した移動の数と、何も適用しなかった反復の数 |
| `n_best_updates` | 最良解が真に改善した回数 |
| `seed` | その実行が使ったシード。`run` にシードを渡さなかった場合は `None` |
| `proven_optimal`、`dual_bound` | `BranchAndBound` が証明した内容。[Exact search](https://trash-iine.github.io/optopus-py/heuristics_exact.html) (英語) を参照。他のヒューリスティックでは `None` |

```python
run = report.runs[0]
print(run)
# RunResult(best_objective=93, best_iteration=1745, time_to_best_secs=..., total_time_secs=...)
print(run.seed, run.iterations, run.n_accepted, run.n_rejected)  # 7 10000 729 9271
print(run.initial_objective, run.best_objective, run.improvement)  # 60.0 93.0 33.0
```

(picking-the-best-solution)=
### 最良解を取り出す

レポート自体は最良解を持っていません。`best_objective` がレポートのものと一致する実行を選び、その
`solution` を読みます。最大化問題なら `max`、最小化問題なら `min` を使います。

```python
best = max(report.runs, key=lambda r: r.best_objective)  # 最小化問題なら min
print(best.solution)  # 頂点ごとの bool: その頂点がどちら側か
```

各問題がどちらの向きで値を報告するか、また報告された値を改めて評価した値と末尾の桁が食い違うことが
ある理由は、[What a run reports](https://trash-iine.github.io/optopus-py/problems.html#what-a-run-reports)
(英語) にあります。

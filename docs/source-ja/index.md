---
source_sha256: f7a0e0e9f78a
---

# optopus ドキュメント

`optopus` は、Rust で書かれた組合せ最適化ライブラリ
[optopus](https://github.com/trash-iine/optopus) の Python バインディングです。Python のデータから
最適化問題を組み立て、メタヒューリスティクスで解くところまでをすべて Python で書けます。探索ループ
自体はネイティブの Rust で実行されます。

```{note}
これは日本語版です。英語版が原文で、日本語版はそれに追従して更新します。まだ翻訳していないページは
英語版にリンクしています。
```

(install)=
## インストール

```bash
pip install git+https://github.com/trash-iine/optopus-py.git
```

これはソースから拡張モジュールをビルドするため、Python ≥ 3.9 と
[Rust ツールチェーン](https://rustup.rs) (rustc ≥ 1.88) が必要です。Linux x86_64 と macOS には
ビルド済みの wheel があり、Rust なしでインストールできます。そのコマンド、バージョンの固定方法、
開発環境の構築は[インストール](installation.md)にあります。

(use-it-in-three-steps)=
## 3 ステップで使う

1. データから**問題を作る**。ここでは重み付き三角形上の Max Cut を解きます。頂点を 2 つの側に
   分け、両側をまたぐ辺の重みの合計をできるだけ大きくする問題です。
2. **ヒューリスティックを設定する**。各実行を終わらせる `StopCondition` も一緒に渡します。
3. **実行して** `RunReport` を読む。全実行を通じた最良の目的関数値と、各実行の解が入っています。

```python
import optopus

# 1. 問題: 辺を (u, v, 重み) で与える。頂点番号は 0 から。
mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])

# 2. ヒューリスティック: 1 頂点を反転する近傍でのタブー探索、1 実行あたり 1,000 反復。
ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(1, 2),
    stop=optopus.StopCondition(max_iteration=1_000),
)

# 3. 独立に 5 回実行。シードを固定すれば再現できる。
report = ts.run(mc, runs=5, seed=42)
print(report.best_objective)    # 5.0 — 辺 (0, 2) と (1, 2) をカット
print(report.runs[0].solution)  # [True, True, False] — 各頂点がどちら側か
```

どの問題とどのヒューリスティックも、この 3 ステップに同じように当てはまります。`MaxCut` を `Tsp`
に、`"Flip"` を `"TwoOpt"` に、あるいは `TabuSearch` を `SimulatedAnnealing` に替えても、残りは
そのままです。

(where-to-go-next)=
## 次に読むページ

| やりたいこと | ページ |
|---|---|
| いくつかの問題を最初から最後まで解いてみる | [クイックスタート](quickstart.md) |
| 実行回数・シード・時間制限を制御し、結果の全フィールドを読む | [探索の実行と結果の読み方](running.md) |
| 用途に合う問題を選び、それぞれの解き方を見る | [Built-in problems](https://trash-iine.github.io/optopus-py/problems.html) (英語) |
| 目的関数と制約を多項式で書く | [Modeling with Formula](https://trash-iine.github.io/optopus-py/formula.html) (英語) |
| 問題を Python のクラスとして書く | [Problems written in Python](https://trash-iine.github.io/optopus-py/python_problems.html) (英語) |
| グラフや、最適値が分かっているインスタンスを生成する | [Graphs and test instances](https://trash-iine.github.io/optopus-py/graphs_and_instances.html) (英語) |
| ヒューリスティックを選んで調整する | [Choosing a heuristic](https://trash-iine.github.io/optopus-py/heuristics.html) (英語) |
| 個々のヒューリスティックの引数と例を調べる | [Generic](https://trash-iine.github.io/optopus-py/heuristics_generic.html)、[composed](https://trash-iine.github.io/optopus-py/heuristics_composed.html)、[problem-specific](https://trash-iine.github.io/optopus-py/heuristics_specific.html)、[exact](https://trash-iine.github.io/optopus-py/heuristics_exact.html) (英語) |
| シグネチャを確認する | [API reference](https://trash-iine.github.io/optopus-py/api_reference.html) (英語) |

(what-you-get)=
## 提供しているもの

- Python のデータから直接作れる**問題**:
  [`MaxCut`](https://trash-iine.github.io/optopus-py/problems.html#maxcut)、
  [`Qubo`](https://trash-iine.github.io/optopus-py/problems.html#qubo)、
  [`Sat`](https://trash-iine.github.io/optopus-py/problems.html#sat)、
  [`VertexCover`](https://trash-iine.github.io/optopus-py/problems.html#vertexcover)、
  [`Tsp`](https://trash-iine.github.io/optopus-py/problems.html#tsp)、
  [`JobShopScheduling`](https://trash-iine.github.io/optopus-py/problems.html#jobshopscheduling)、
  [`FlowShop`](https://trash-iine.github.io/optopus-py/problems.html#flowshop)、
  [`Vrp`](https://trash-iine.github.io/optopus-py/problems.html#vrp)、
  [`Qap`](https://trash-iine.github.io/optopus-py/problems.html#qap)、
  [`BinPacking`](https://trash-iine.github.io/optopus-py/problems.html#binpacking)、
  [`GraphColoring`](https://trash-iine.github.io/optopus-py/problems.html#graphcoloring)、
  [`Formula`](https://trash-iine.github.io/optopus-py/formula.html)。グラフ上の問題に渡すための
  [`Graph`](https://trash-iine.github.io/optopus-py/graphs_and_instances.html#graph) 型もあり、
  ランダムグラフの生成器と周期境界の格子を備えています。
- どの問題にも使える**[汎用ヒューリスティック](https://trash-iine.github.io/optopus-py/heuristics_generic.html)**:
  `LocalSearch`、`SimulatedAnnealing`、`BangBangSimulatedAnnealing`、`TabuSearch`、
  `LateAcceptanceHillClimbing`、`RandomWalk`、`BeamSearch`、`PopulationAnnealing`、
  `ParallelTempering`、`ReinforcementLearningSearch`。
- 他のヒューリスティックのインスタンスを組み合わせて作る**[合成ヒューリスティック](https://trash-iine.github.io/optopus-py/heuristics_composed.html)**:
  `VariableNeighborhoodSearch`、`Sequential`、`Iterated`、`Restart`、`GeneticAlgorithm`、
  `BreakoutLocalSearch.from_parts`。
- 特定の問題の構造を利用する**[問題専用ヒューリスティック](https://trash-iine.github.io/optopus-py/heuristics_specific.html)**:
  `WalkSat`、`BreakoutLocalSearch`、`LinKernighanHelsgaun`、
  `AdaptiveLargeNeighborhoodSearch`、`HybridGeneticSearch`、`IteratedGreedy`、
  `GreedyRandomizedConstruction`。
- **[厳密解法](https://trash-iine.github.io/optopus-py/heuristics_exact.html)**: `BranchAndBound` は
  `MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` で最適性を証明します。各ノードの探索には別の
  ヒューリスティックを使います。
- **[最適値が分かっているインスタンス](https://trash-iine.github.io/optopus-py/graphs_and_instances.html#plantedmaxcut)**:
  `PlantedMaxCut` は構成上最適なカットを埋め込むので、「ヒューリスティックが最適値に到達したか」を
  判定できます。[`MaxCutKernel`](https://trash-iine.github.io/optopus-py/graphs_and_instances.html#maxcutkernel)
  はインスタンスを厳密に縮約し、得られた解を元のインスタンスに戻します。
- **[自作の問題](https://trash-iine.github.io/optopus-py/python_problems.html)**: 初期解・目的関数・
  近傍を提供する Python オブジェクトなら、汎用ヒューリスティックと合成ヒューリスティックで解けます。
  ruin 系のメソッドを定義すれば ALNS でも解けます。
- **[実行結果の統計](#the-report)**: `run` はどれも `RunReport` を返し、1 回以上の実行に
  わたる目的関数値・時間・受理回数を集計します。

```{toctree}
---
maxdepth: 1
caption: はじめに
---

installation
quickstart
running
```

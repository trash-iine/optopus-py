---
source_sha256: 9824b733dc88
---

# API リファレンス

このリファレンスは、コンパイル済みの `optopus` 拡張の docstring (Rust の `///` ドキュメントコメント
として書かれています) から自動生成しています。**docstring は英語のままです。** 読み物としての入門は
[クイックスタート](quickstart.md)を見てください。以下のどのクラスにも、各見出しからリンクしている
ガイドのページに例付きの節があります。

`optopus.__version__` は、インストールされたパッケージのバージョンを `"0.1.0"` のような文字列で
表します。

(problems)=
## 問題

*問題*は、**何を**最適化するかを表します。どれも静的なファクトリメソッドで Python のデータから作り、
ヒューリスティックに渡します。

```{eval-rst}
.. autoclass:: optopus.MaxCut
   :members:

.. autoclass:: optopus.Qubo
   :members:

.. autoclass:: optopus.Sat
   :members:

.. autoclass:: optopus.VertexCover
   :members:

.. autoclass:: optopus.Tsp
   :members:

.. autoclass:: optopus.JobShopScheduling
   :members:

.. autoclass:: optopus.Vrp
   :members:

.. autoclass:: optopus.FlowShop
   :members:

.. autoclass:: optopus.Qap
   :members:

.. autoclass:: optopus.BinPacking
   :members:

.. autoclass:: optopus.VehicleType
   :members:

.. autoclass:: optopus.GraphColoring
   :members:

.. autoclass:: optopus.Formula
   :members:
```

(instances-with-a-known-optimum)=
### 最適値が分かっているインスタンス

`PlantedMaxCut` は、構成上最適値が正確に分かる Max Cut のインスタンスを生成します。これによって
「ヒューリスティックが最適値に達したか」に答えられるようになります。

```{eval-rst}
.. autoclass:: optopus.PlantedMaxCut
   :members:
```

(problem-reduction)=
### 問題の縮約

`MaxCutKernel` は、厳密なカーネル化で Max Cut のインスタンスを縮小します。カーネルは普通の
`MaxCut` なので、どのヒューリスティックもそのまま動き、`lift` が結果を元に戻します。

```{eval-rst}
.. autoclass:: optopus.MaxCutKernel
   :members:
```

Python で書いた問題は普通のオブジェクトです。[Python で書いた問題](python_problems.md)を参照して
ください。

## Graph

`MaxCut`、`VertexCover`、`GraphColoring` は、無向の重み付きグラフ上で定義されます。辺リストから直接
作るか、ランダムグラフのモデル (Erdős-Rényi、Barabási-Albert、Watts-Strogatz) から生成するか、周期
境界の格子 (`grid_torus_2d`、`grid_torus_3d`) を取り、問題の `from_graph` に渡します。例:
[グラフとテスト用インスタンス](graphs_and_instances.md)。

```{eval-rst}
.. autoclass:: optopus.Graph
   :members:
```

(heuristics)=
## ヒューリスティック

*ヒューリスティック*は、**どう**探索するかを表します。どのヒューリスティックも、`RunReport` を返す
同じ `run(problem, runs=1, seed=None)` メソッドを持ちます (下の[結果](#results)を参照)。

(generic)=
### 汎用

Python の問題を含め、どの種類の問題でも動きます。近傍は `neighbor` 引数で選びます。例:
[汎用ヒューリスティック](heuristics_generic.md)。

```{eval-rst}
.. autoclass:: optopus.LocalSearch
   :members:

.. autoclass:: optopus.SimulatedAnnealing
   :members:

.. autoclass:: optopus.TabuSearch
   :members:

.. autoclass:: optopus.LateAcceptanceHillClimbing
   :members:

.. autoclass:: optopus.RandomWalk
   :members:

.. autoclass:: optopus.BangBangSimulatedAnnealing
   :members:

.. autoclass:: optopus.BeamSearch
   :members:

.. autoclass:: optopus.PopulationAnnealing
   :members:

.. autoclass:: optopus.ParallelTempering
   :members:

.. autoclass:: optopus.ReinforcementLearningSearch
   :members:
```

(composed)=
### 合成

他のヒューリスティックのインスタンスをステップとして受け取り、ステップが合う問題ならどれでも動き
ます。`GeneticAlgorithm` は交叉も必要とします。組み込みの問題はどれも交叉を持ち、Python の問題は
メソッドとして提供します。例: [合成ヒューリスティック](heuristics_composed.md)。

```{eval-rst}
.. autoclass:: optopus.VariableNeighborhoodSearch
   :members:

.. autoclass:: optopus.Sequential
   :members:

.. autoclass:: optopus.Iterated
   :members:

.. autoclass:: optopus.Restart
   :members:

.. autoclass:: optopus.GeneticAlgorithm
   :members:
```

`BreakoutLocalSearch.from_parts` も同じように、他のヒューリスティックから breakout local search を
組み立てます。下の `BreakoutLocalSearch` を参照してください。

(exact-search)=
### 厳密解法

`BranchAndBound` も別のヒューリスティックを受け取り、最良解の最適性が証明されるまでそれで探索します。
`MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` で動き、証明した内容を
`RunResult.proven_optimal` と `RunResult.dual_bound` に報告します。[厳密解法](heuristics_exact.md)を
参照してください。

```{eval-rst}
.. autoclass:: optopus.BranchAndBound
   :members:
```

(problem-specific)=
### 問題専用

1 種類の問題の構造を利用し、`neighbor` 引数を取りません。別の問題で実行すると `ValueError` になります。
`AdaptiveLargeNeighborhoodSearch` と `GreedyRandomizedConstruction` は、ruin 系メソッドを定義した
Python の問題でも動きます。例: [問題専用ヒューリスティック](heuristics_specific.md)。

```{eval-rst}
.. autoclass:: optopus.WalkSat
   :members:

.. autoclass:: optopus.BreakoutLocalSearch
   :members:

.. autoclass:: optopus.LinKernighanHelsgaun
   :members:

.. autoclass:: optopus.AdaptiveLargeNeighborhoodSearch
   :members:

.. autoclass:: optopus.IteratedGreedy
   :members:

.. autoclass:: optopus.GreedyRandomizedConstruction
   :members:

.. autoclass:: optopus.HybridGeneticSearch
   :members:
```

(api-reference-neighborhoods)=
### 近傍

汎用ヒューリスティックは近傍を文字列で受け取ります。受け付ける値と各移動の内容は、問題ごとに
[近傍](#neighborhoods)に一覧があります。

(stop-condition)=
## 停止条件

上限の組み合わさり方と、各ヒューリスティックの 1 反復が何か: [停止条件](#stopping)。

```{eval-rst}
.. autoclass:: optopus.StopCondition
   :members:
```

(results)=
## 結果

`run` の呼び出しはどれも、1 つ以上の `RunResult` を集計した `RunReport` を返します。各問題で目的関数値
が何を意味するか (ペナルティを含む) は[実行が報告する値](#what-a-run-reports)に、全フィールドと例は
[レポート](#the-report)にあります。

```{eval-rst}
.. autoclass:: optopus.RunReport
   :members:

.. autoclass:: optopus.RunResult
   :members:
```

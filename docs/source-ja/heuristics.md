---
source_sha256: ae533f6386de
---

# ヒューリスティックの選び方

ヒューリスティックは、探索が問題の解の間をどう移動するかを決めます。このページでは、どれを使うか
と、主な引数の設定方法を説明します。各ヒューリスティックには、すべての引数と例を載せた節が次の
4 つのページのいずれかにあります。

- [汎用ヒューリスティック](heuristics_generic.md)は、どの問題でも動き、問題の移動を名前で受け取り
  ます: `LocalSearch`、`SimulatedAnnealing`、`BangBangSimulatedAnnealing`、`TabuSearch`、
  `LateAcceptanceHillClimbing`、`RandomWalk`、`BeamSearch`、`PopulationAnnealing`、
  `ParallelTempering`、`ReinforcementLearningSearch`。
- [合成ヒューリスティック](heuristics_composed.md)は、他のヒューリスティックから組み立てます:
  `Sequential`、`Iterated`、`VariableNeighborhoodSearch`、`Restart`、`GeneticAlgorithm`、
  `BreakoutLocalSearch.from_parts`。
- [問題専用ヒューリスティック](heuristics_specific.md)は、1 つの問題の構造を利用します:
  `WalkSat`、`BreakoutLocalSearch`、`LinKernighanHelsgaun`、`AdaptiveLargeNeighborhoodSearch`、
  `IteratedGreedy`、`GreedyRandomizedConstruction`、`HybridGeneticSearch`。
- [厳密解法](heuristics_exact.md)は最適性を証明します: `BranchAndBound`。

どれでも、実行・停止・結果の読み方は[探索の実行と結果の読み方](running.md)にあります。

(which-one)=
## どれを使うか

問題から考えてください。その問題専用のヒューリスティックがあれば、まずそれを試します。問題の構造を
もとに作られているので、たいていはそれが勝ちます。

| 問題 | 最初に試すもの |
|---|---|
| `Sat` | [`WalkSat`](heuristics_specific.md#walksat) |
| `MaxCut` | [`BreakoutLocalSearch`](heuristics_specific.md#breakoutlocalsearch) |
| `Tsp` | [`LinKernighanHelsgaun`](heuristics_specific.md#linkernighanhelsgaun) |
| `Vrp` | [`HybridGeneticSearch`](heuristics_specific.md#hybridgeneticsearch) または [`AdaptiveLargeNeighborhoodSearch`](heuristics_specific.md#adaptivelargeneighborhoodsearch) |
| `FlowShop` | [`IteratedGreedy`](heuristics_specific.md#iteratedgreedy) |
| `Qap` | `"Swap"` での [`TabuSearch`](heuristics_generic.md#tabusearch) |
| `BinPacking` | [`AdaptiveLargeNeighborhoodSearch`](heuristics_specific.md#adaptivelargeneighborhoodsearch) |

そうでなければ、探索の地形が必要とするものに応じて汎用ヒューリスティックを選びます。

| 状況 | 使うもの |
|---|---|
| 手早い基準解、または合成ヒューリスティックの中の仕上げのステップ | [`LocalSearch`](heuristics_generic.md#localsearch) |
| ランダムな移動 1 つを安く評価できるときの一般的な既定 | [`SimulatedAnnealing`](heuristics_generic.md#simulatedannealing) |
| [`SimulatedAnnealing`](heuristics_generic.md#simulatedannealing) と同様だが、温度スケジュールの代わりに引数 1 つで済ませたい | [`LateAcceptanceHillClimbing`](heuristics_generic.md#lateacceptancehillclimbing) |
| 近傍が小さい、または良い移動がどれも悪い解を経由する地形 (たとえば `"Eq"` 制約を持つ `Formula`) | [`TabuSearch`](heuristics_generic.md#tabusearch) |
| 1 つの谷で行き詰まる探索 | 上のどれかを包む [`Iterated`](heuristics_composed.md#iterated) か [`VariableNeighborhoodSearch`](heuristics_composed.md#variableneighborhoodsearch) |
| 多くの候補にわたる多様性 | [`PopulationAnnealing`](heuristics_generic.md#populationannealing)、[`ParallelTempering`](heuristics_generic.md#paralleltempering)、[`GeneticAlgorithm`](heuristics_composed.md#geneticalgorithm) |

`LocalSearch` と `TabuSearch` は、各ステップで近傍のすべての移動を評価します。数千変数の反転なら
安いですが、大きなインスタンスでの `"TwoOpt"` や `"Swap"` のような 2 要素の移動では高くつき、
Python の問題では移動 1 つにつき Python の呼び出しが 1 回かかります。ステップごとにランダムな移動を
1 つ評価するヒューリスティックは、近傍の大きさによらずスケールします。

(setting-the-main-parameters)=
## 主な引数の設定

### SimulatedAnnealing

大きさ Δ の悪化する移動は確率 `exp(−Δ / T)` で受理され、T はステップのたびに `cooling_rate` 倍
されます。

- `initial_temperature` は、移動 1 回の典型的な悪化量 Δ の近くに設定し、そうした移動が最初は 3 回に
  1 回ほど受理されるようにします。ずっと高いと実行の前半がランダムウォークに費やされ、ずっと低いと
  最初から局所探索になります。
- `cooling_rate` は、最後の反復までに温度が意味のある最小の Δ を十分下回るように設定します:
  `cooling_rate = (T_end / T_start) ** (1 / iterations)`。10⁶ 反復で 10 から 0.01 にするなら約
  `0.999993` です。
- 実行不可能性にペナルティを課す問題 (`Vrp`、`GraphColoring`、`VertexCover`、制約付きの
  `Formula`) では、Δ は実行可能解どうしの移動から取ります。すると制約を破る移動は棄却されます。
  これにはそうした移動が存在する必要があります。`Formula` の `"Eq"` のもとでの 1 回の `"Change"`
  のように、実行可能解から出るどの移動も制約を破る場合、探索は実行不可能な解を経由しなければ
  なりません。温度をペナルティの重みの近くに設定するか、温度なしでそこを越える `TabuSearch` を
  使ってください。
- 個数の 1 単位より小さいタイブレークのような小さな 2 次的な項を持つ目的関数では、意味のある Δ は
  その項の尺度 (多くは 1000 分の 1 程度) になり、個数を変える移動は丸 1 単位のコストになります。
  温度は小さいほうの尺度に合わせて設定します。
- スケジュールは秒ではなくステップを数えます。`max_duration_secs` だけを使うと、実行が到達する温度は
  マシンの速さで変わるので、`cooling_rate` は短い実行で測った反復回数から求めてください。

典型的な Δ は、推測するより測るほうが簡単です。Python で書いた問題なら、近傍自身のメソッドで
サンプリングします。

```python
import random

def worsening_deltas(problem, neighborhood, samples=1_000, seed=0):
    rng = random.Random(seed)
    solution = problem.new_solution(rng)
    deltas = []
    for _ in range(samples):
        move = neighborhood.random_neighbor(problem, solution, rng)
        if move is not None:
            d = neighborhood.delta(problem, solution, move)
            deltas.append(d if problem.minimize else -d)
    return sorted(d for d in deltas if d > 0)
```

その中央値は `initial_temperature` として妥当で、最小の値は `T_end` の目安になります。ランダムな解の
Δ は大きくなるので、短い `LocalSearch` の結果など、探索が実際に見るものに近い解からサンプリングして
ください。

(the-other-heuristics)=
### その他のヒューリスティック

それぞれの節で、引数の意味とどこから始めるかを説明しています:
[汎用ヒューリスティック](heuristics_generic.md)の
[`TabuSearch`](heuristics_generic.md#tabusearch)、
[`LateAcceptanceHillClimbing`](heuristics_generic.md#lateacceptancehillclimbing)、
[`PopulationAnnealing`](heuristics_generic.md#populationannealing)、
[`ParallelTempering`](heuristics_generic.md#paralleltempering)。

(heuristics-problem-specific-heuristics)=
### 問題専用ヒューリスティック

既定値は論文で公表された設定で、出発点として妥当です。変える価値のあるものは
[問題専用ヒューリスティック](heuristics_specific.md)の各節にあります。たとえば小さなインスタンスでの
`AdaptiveLargeNeighborhoodSearch` の `removal_fraction` や、時間予算のもとでのその `cooling_rate`
です。

(proving-optimality)=
## 最適性の証明

上のヒューリスティックはどれも、より良い解が存在するかどうかを判定できません。`BranchAndBound` は
`MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` でそれができ、各ノードの探索には上のどれでも
使えます。[厳密解法](heuristics_exact.md)を参照してください。

(problems-written-in-python)=
## Python で書いた問題

汎用ヒューリスティックと合成ヒューリスティックはどれも Python で書いた問題で動き、
`AdaptiveLargeNeighborhoodSearch` も問題が ruin 系メソッドを定義すれば動きます。省略可能なメンバーを
必要とするものもあります: `TabuSearch` は近傍の `tabu_keys` を、`GeneticAlgorithm` は問題の
`crossover` を必要とします。ステップごとに Python を呼び出すので、ステップごとにランダムな移動を
1 つ評価するヒューリスティックを選び、どの近傍にも `delta` と `random_neighbor` を持たせてください。
各ヒューリスティックが何を呼び出すかは [Python で書いた問題](python_problems.md)にあります。

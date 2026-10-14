---
source_sha256: 5ad136137c72
---

# 汎用ヒューリスティック

汎用ヒューリスティックは、組み込みの問題、`Formula`、[Python で書いた問題](python_problems.md)の
どれでも動きます。解の間を問題自身の移動で動き、その移動は `MaxCut` なら `"Flip"`、`Tsp` なら
`"TwoOpt"` のように `neighbor` で名前を指定して受け取ります。各問題が受け付ける名前は
[近傍](#neighborhoods)にあり、知らない名前は `run` を呼んだときに `ValueError` になります。

どのヒューリスティックも `StopCondition` を受け取り、同じ `run(problem, runs=1, seed=None)` を持ち
ます。その両方と、各ヒューリスティックの 1 反復が何かは[探索の実行と結果の読み方](running.md)に
あります。どれを最初に試すかと主な引数の調整方法は[ヒューリスティックの選び方](heuristics.md)に
あります。

このページの例はどれも同じインスタンスを使います。30 頂点のランダムな Max Cut で、その最適値 93 は
[厳密解法](heuristics_exact.md)のページで証明しています。

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
```

## LocalSearch

最良改善の貪欲な降下法です。各反復で近傍全体を評価して最良の移動を適用し、どの移動も改善しない
最初の局所最適解で実行が終わります。そこそこの解に最も速く達する方法で、基準解として、また
[合成ヒューリスティック](heuristics_composed.md)の中の仕上げのステップとして定番です。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `stop` | | `StopCondition`。局所最適でそれより早く終わることがある |

```python
ls = optopus.LocalSearch(neighbor="Flip", stop=optopus.StopCondition(max_iteration=1_000))
report = ls.run(mc, runs=5, seed=42)
print([r.best_objective for r in report.runs])  # [92.0, 87.0, 86.0, 88.0, 86.0]
print(report.avg_iterations)                    # 9.4 — どの実行も局所最適で停止
```

実行ごとに異なる局所最適解で終わり、どれも最適値 93 ではありません。実行を増やせばより多くの局所
最適解を拾えますが、以下のヒューリスティックはむしろそこから抜け出します。

## SimulatedAnnealing

各反復でランダムな移動を 1 つ引きます。改善する移動は必ず採用し、大きさ Δ の悪化する移動は確率
`exp(−Δ / T)` で採用します。温度 `T` は `initial_temperature` から始まり、反復のたびに
`cooling_rate` 倍されるので、探索は最初は歩き回り、冷えるにつれて降下に落ち着きます。ランダムな
移動 1 つを安く評価できるなら、いつでも堅実な既定の選択です。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `initial_temperature` | | 初期温度。単位は目的関数値と同じ |
| `cooling_rate` | | 反復ごとに温度に掛ける係数。`0.999` など |
| `stop` | | `StopCondition` |

```python
sa = optopus.SimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(sa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

結果を決めるのはスケジュールです。[主な引数の設定](heuristics.md#simulatedannealing)では、典型的な
悪化する移動の大きさと反復の予算から両方を導いています。

## BangBangSimulatedAnnealing

温度が一度冷えて終わるのではなく振動する焼きなまし法です。`min_wave_threshold` を下回るまで
`cooling_rate` で冷やし、次に `max_wave_threshold` を上回るまで `cooling_rate` で割って温め、これを
繰り返します。再加熱のたびに探索は凍りついた谷から出られるので、通常の焼きなまし法なら凍ったまま
過ごしてしまう長い実行に向いています。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `initial_temperature` | | 初期温度 |
| `cooling_rate` | | 冷却中の反復ごとの係数。加熱中はその逆数 |
| `min_wave_threshold` | | 冷却から加熱に切り替わる温度 |
| `max_wave_threshold` | | 加熱から冷却に切り替わる温度 |
| `stop` | | `StopCondition` |

```python
bbsa = optopus.BangBangSimulatedAnnealing(
    neighbor="Flip",
    initial_temperature=2.0,
    cooling_rate=0.999,
    min_wave_threshold=0.05,
    max_wave_threshold=2.0,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(bbsa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

6 つの引数はすべてこの順の位置引数としても渡せ、どれも既定値を持ちません。`max_wave_threshold` から
`min_wave_threshold` まで下がる 1 つの波は `log(min / max) / log(cooling_rate)` 反復かかり、ここでは
約 3,700 です。

## TabuSearch

各反復で近傍全体を評価し、タブーでない最良の移動を、解が悪くなる場合でも適用します。移動が触れた
変数や辺は `tabu_tenure` から引いた期間タブーのままになり、探索が直前の移動を取り消して、抜け出した
ばかりの局所最適解に戻るのを防ぎます。タブーの移動でも、最良解を更新するなら採用されます。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `tabu_tenure` | | `(min, max)`。各タブー期間はこの範囲から一様に引く |
| `stop` | | `StopCondition` |

```python
ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(2, 5),
    stop=optopus.StopCondition(max_iteration=500),
)
print(ts.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

調整すべき温度なしに悪い解を越えられるので、小さな近傍や、どの単独の移動も `"Eq"` 制約を破る
`Formula` では第一の選択肢です。`Qap` では `"Swap"` は Taillard の規則でタブーになり、Python の
問題では近傍が `tabu_keys` を定義していなければなりません。タブー期間の決め方:

- 期間 1 は直前の移動の取り消しを禁止するだけで、探索は 2 つの解の間を行き来します。
- 数手から始め、インスタンスに合わせて変数の数の 1/10 程度まで大きくします。変数の数に近い期間は
  ほとんどすべてを禁止してしまいます。
- `(5, 15)` のように 1 つの値ではなく範囲にすると、探索が一定の長さの周期に陥るのを防げます。

## LateAcceptanceHillClimbing

各反復でランダムな移動を 1 つ引き、結果が現在の解、または `history_length` 反復前の解より悪くなければ
受理します。焼きなまし法と同じく悪化する移動もある程度受理しますが、唯一の引数は温度スケジュールでは
なく長さで、目的関数の尺度を知る必要がありません。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `history_length` | | 比較が何反復前までさかのぼるか。`>= 1` |
| `stop` | | `StopCondition` |

```python
lahc = optopus.LateAcceptanceHillClimbing(
    neighbor="Flip",
    history_length=50,
    stop=optopus.StopCondition(max_iteration=10_000),
)
print(lahc.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

`1` は山登り法に近くなります。履歴が長いほどよく探索し、収束は遅くなるので、より多くの反復が必要
です。数百万反復の実行なら数千が適しています。

## RandomWalk

各反復で、目的関数への影響にかかわらず一様ランダムな移動を適用します。それでも見つけた最良解は
保持します。単独では、他のヒューリスティックがどれだけ上乗せしているかを示す基準にすぎません。本当の
用途は `Iterated` の摂動や `VariableNeighborhoodSearch` の shake で、そこでは `max_iteration` が
ランダムな移動の数、つまり揺さぶりの強さになります。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `stop` | | `StopCondition`。ランダムウォークは自分では止まらないので必須 |

```python
rw = optopus.RandomWalk(neighbor="Flip", stop=optopus.StopCondition(max_iteration=1_000))
print(rw.run(mc, runs=3, seed=42).best_objective)  # 81.0 — 超えるべき基準
```

## BeamSearch

`beam_width` 個の候補解を保持します。各反復ですべての候補の近傍全体を展開し、到達した解のうち最良の
`beam_width` 個を残します。幅 1 は、局所最適では最良の悪化する移動も採用する降下法です。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `beam_width` | | 保持する候補の数。`>= 1` |
| `stop` | | `StopCondition` |

```python
bs = optopus.BeamSearch(neighbor="Flip", beam_width=4, stop=optopus.StopCondition(max_iteration=50))
print(bs.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

1 反復で `beam_width` 個の近傍全体を評価するので、大きなインスタンスでの 2 要素の移動では高く
つきます。

## PopulationAnnealing

`population_size` 個のレプリカを保持し、まとめて焼きなまします。各温度ステップで逆温度 β を
`delta_beta` だけ上げ、レプリカをボルツマン重みに比例して再サンプリングし (良いレプリカは複製され、
悪いものは捨てられます)、各レプリカをメトロポリス法の移動で `sweeps_per_step` 回スイープします。
`reset_period` ステップごとに、スケジュールは `initial_beta` からやり直します。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `population_size` | | レプリカの数。`>= 2` |
| `stop` | | `StopCondition`。1 反復は全レプリカの 1 スイープ |
| `initial_beta` | `0.1` | 初期の逆温度。`> 0` |
| `delta_beta` | `0.02` | ステップごとに β に足す量。`> 0` |
| `sweeps_per_step` | `50` | ステップごと、レプリカごとのメトロポリス・スイープの回数。`>= 1` |
| `reset_period` | `400` | スケジュールをやり直すまでのステップ数。`0` ならやり直さない |
| `neighbor` | `"Flip"` | スイープを駆動する移動。問題に `"Flip"` がなければ問題自身のものを渡す |
| `sweep_length` | `None` | 1 スイープあたりの提案の数。`None` は近傍の大きさ |

```python
pa = optopus.PopulationAnnealing(
    population_size=20,
    stop=optopus.StopCondition(max_iteration=200),
    initial_beta=0.2,
    delta_beta=0.1,
    sweeps_per_step=10,
)
print(pa.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

β の単位は目的関数値の逆数で、実行は最も冷えた時点で `initial_beta + steps · delta_beta` に達します。
`steps` は `max_iteration / sweeps_per_step` です。既定値では短い実行は熱いままです: 200 反復は
4 ステップで β は約 0.18 にしかならず、10,000 反復でも β = 4.1 です。最後のステップで悪化する移動が
まれになるように `delta_beta` を決めてください。`"TwoOpt"` のような 2 要素の移動では、既定の
スイープがすべての組を提案するので、`sweep_length` を設定してください。

## ParallelTempering

`num_replicas` 個のレプリカを、`beta_min` (最も熱い) から `beta_max` (最も冷たい) までの等比の
はしごに並べた固定の逆温度で保持します。各ラウンドで、各レプリカをそれぞれの温度で
`sweeps_per_exchange` 回スイープし、そのあと隣り合うレプリカどうしに位置を交換させます。熱い
レプリカが見つけた良い配置ははしごを下って磨かれる場所へ移り、冷たい端で行き詰まった配置ははしごを
上って抜け出せる場所へ移ります。何も冷えないのが `PopulationAnnealing` との違いです。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `num_replicas` | | はしごに並べるレプリカの数。`>= 2` |
| `stop` | | `StopCondition`。1 反復は全レプリカの 1 スイープ |
| `beta_min` | `0.1` | 最も熱いレプリカの逆温度。`> 0` |
| `beta_max` | `5.0` | 最も冷たいレプリカの逆温度。`beta_min` より大きい |
| `sweeps_per_exchange` | `1` | 2 回の交換の試みの間のスイープ数。`>= 1` |
| `neighbor` | `"Flip"` | スイープを駆動する移動 |
| `sweep_length` | `None` | 1 スイープあたりの提案の数。`None` は近傍の大きさ |

```python
pt = optopus.ParallelTempering(num_replicas=8, stop=optopus.StopCondition(max_iteration=1_000))
print(pt.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

温度の単位は目的関数値と同じです。`beta_max` は最も冷たいレプリカを降下法にし、`beta_min` は最も
熱いレプリカを自由に動かすようにします。レプリカどうしは、隣のエネルギーが重なるくらい近くなければ
なりません。そうでないと良い配置がはしごを下っていきません。範囲を広げるときは、レプリカも一緒に
増やしてください。2 要素の移動では、`PopulationAnnealing` と同じく `sweep_length` を設定してください。

## ReinforcementLearningSearch

探索しながら、どの移動を選ぶかを学習します。各反復で、候補の移動それぞれを 21 個の特徴量で表します。
その利得、改善するかどうか、候補の中での順位を、それぞれ単独で、また実行の進み具合や停滞の長さなど
探索の 6 つの特徴量との積として表したものです。それらの特徴量の線形スコアに対するソフトマックスから
移動を選び、その移動が得た報酬から REINFORCE で重みを更新します。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `neighbor` | | 移動の名前 |
| `stop` | | `StopCondition` |
| `learning_rate` | `0.01` | 方策勾配のステップ。`>= 0`。`0` なら学習しない |
| `softmax_temperature` | `1.0` | 移動のスコアに対するソフトマックスの温度。`> 0`。大きいほど一様に選ぶ |
| `reward_shaping` | `"Normalized"` | `"Normalized"`: そのステップの最大の利得に対する移動の利得。`"Raw"`: 利得そのもの。`"BestImprovement"`: 最良解の更新なら 1、それ以外は 0 |
| `policy_weights` | `None` | 21 個の初期の重み。特徴量ごとに 1 つ。`None` なら 0 から始める |
| `max_candidates` | `None` | 1 反復でサンプリングする移動の数。`None` なら近傍全体を評価する |

```python
rl = optopus.ReinforcementLearningSearch(
    neighbor="Flip", stop=optopus.StopCondition(max_iteration=2_000)
)
print(rl.run(mc, runs=3, seed=42).best_objective)  # 93.0
```

方策は実行ごとにゼロから始まります。近傍が大きい場合は `max_candidates` を数百に設定してください。
遅くなる原因は、毎反復すべての移動を評価することです。

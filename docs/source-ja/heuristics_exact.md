---
source_sha256: 544935f8753b
---

# 厳密解法

他のヒューリスティックはどれも見つけた最良解を返すだけで、より良い解が存在するかどうかは判定でき
ません。`BranchAndBound` は `MaxCut`、`Qubo`、`Sat`、`VertexCover`、`Formula` でそれができます。
問題を、いくつかの変数の範囲を狭めたノードに分割し、各ノードを別のヒューリスティックで探索し、限界
(*緩和*) がそのノードの中にこれまでの最良解を上回るものがないことを示した時点でノードを捨てます。
ノードがなくなれば、最良解は最適です。

## BranchAndBound

各反復で、限界が最も良い未処理のノードを取り出します。その限界がこれまでの最良解を上回れなければ、
ノードは捨てられます。そうでなければ、`heuristic` が最良解から始めてそのノードを探索し、ノードは
1 つの変数の範囲をその解の値のところで 2 つに分割します。

| 引数 | 既定値 | 意味 |
|---|---|---|
| `heuristic` | | 各ノードを探索するヒューリスティック。その `stop` はノードごとの予算 |
| `stop` | `None` | 探索全体の `StopCondition`。`None` ならノードがなくなるまで実行する |
| `relaxation` | `None` | ノードを枝刈りする限界: 下の表の名前、関数、または問題の既定を表す `None` |

```python
import optopus

mc = optopus.MaxCut.from_graph(optopus.Graph.erdos_renyi(30, 0.3, seed=1))
bnb = optopus.BranchAndBound(
    heuristic=optopus.TabuSearch("Flip", (2, 5), stop=optopus.StopCondition(max_iteration=100)),
    stop=optopus.StopCondition(max_duration_secs=10.0),
    relaxation="Eigenvalue",
)
run = bnb.run(mc, seed=42).runs[0]
print(run.best_objective, run.proven_optimal)  # 93.0 True
```

`RunResult` の 2 つのフィールドが、探索が証明したことを示します。他のヒューリスティックではどちらも
`None` です。

- `proven_optimal` は、探索が `stop` より先にノードを使い切ったとき `True` です。
- `dual_bound` は、まだ除外されていない解が到達しうる最良の目的関数値です。最適性が証明されると
  `best_objective` に等しくなり、`stop` で止まった探索では、2 つの限界の差が最適値からどれだけ離れ
  うるかを示します。問題全体を一度も分割しないうちに探索が止まった場合は無限大です。

`heuristic` は各ノードを探索するので、短い `LocalSearch` か `TabuSearch` が定番の選択です。良い解が
どれだけ早く見つかるかは `heuristic` が、探索すべきノードがどれだけ残るかは緩和が決めます。2 値の
問題は、ノードが固定した変数を畳み込んで自身のより小さなインスタンスにするので、`WalkSat` や
`BreakoutLocalSearch` も含めその問題自身のヒューリスティックが、問題全体を探索するのと同じように
ノードを探索します。`Formula` は狭めた範囲の上で探索されます。`stop` の既定は無制限で、大きな
インスタンスでは終わらない実行を意味します。その場合は時間制限を与えてください。

(relaxations)=
## 緩和

緩和は、探索するノードの数を決めます。各問題には既定があります。

| `relaxation` | 問題 | 限界 |
|---|---|---|
| `"Binary"` (既定) | `MaxCut`、`Qubo`、`Sat`、`VertexCover` | 固定した変数を畳み込み、残りを簡単な限界で抑える。`MaxCut` では正の重みの和など |
| `"Eigenvalue"` | `MaxCut` | Delorme と Poljak の固有値限界。ずっと厳しい。ノードあたりのコストは自由な頂点の数の 3 乗で増える |
| `"Interval"` (既定) | `Formula` | 範囲上の区間演算で各項を抑える。どの式でも正しいが、たいていは弱い |

上の 30 頂点のグラフでは、`"Binary"` が同じ最適値を証明するのに `"Eigenvalue"` の約 2,500 倍の反復を
要します。組み込みの限界は数十変数程度のインスタンスに向いています。問題が対応していない名前は、
`run` を呼んだときに `ValueError` になります。

`relaxation=optopus.EigenvalueRelaxation(max_iterations=100, lanczos_steps=30,
certify_limit=3000)` は、上限を指定した `"Eigenvalue"` です。各限界に使う劣勾配法と Lanczos 法の
ステップ数、および正の重みの和に切り替える前に証明する最大のノードの頂点数を決めます。ステップを
増やすと限界は厳しくなり、ノードあたりのコストは増えます。

(a-bound-of-your-own)=
## 自作の限界

問題を知っている限界は関数として書きます。関数はノードの範囲 (変数ごとに両端を含む
`(lower, upper)` の組) を受け取り、その範囲内のどの割当ても上回らない数を返します。最大化問題では
最良の目的関数値以上、最小化問題では以下です。2 値の問題では、範囲は変数が自由な間は `(0, 1)`、
固定されると `(0, 0)` か `(1, 1)` です。目的関数は探索が順位付けに使うもので、ペナルティを含み、
`Formula` では符号を反転しません。

ナップサックでは、1 に固定した品物に、残りの容量を価値/重さの大きい順に自由な品物で埋めたもの
(最後の 1 つは部分的に) を足したものが、そうした限界になります。

```python
values = [6, 5, 4, 2]
weights = [4, 3, 2, 1]
budget = 5

knapsack = optopus.Formula(
    n_vars=len(values),
    objective=[([i], float(v)) for i, v in enumerate(values)],
    direction="Maximize",
    constraints=[
        ([([i], float(w)) for i, w in enumerate(weights)], "Le", [([], float(budget))], 7.0),
    ],
)


def fractional_fill(ranges):
    taken = [i for i, (lower, _) in enumerate(ranges) if lower == 1]
    free = [i for i, (lower, upper) in enumerate(ranges) if lower < upper]
    bound = float(sum(values[i] for i in taken))
    room = max(budget - sum(weights[i] for i in taken), 0)
    for i in sorted(free, key=lambda i: values[i] / weights[i], reverse=True):
        part = min(1.0, room / weights[i])
        bound += part * values[i]
        room -= part * weights[i]
    return bound


bnb = optopus.BranchAndBound(
    heuristic=optopus.LocalSearch("Change", stop=optopus.StopCondition(max_iteration=100)),
    relaxation=fractional_fill,
)
run = bnb.run(knapsack, seed=42).runs[0]
print(run.best_objective, run.solution, run.proven_optimal)  # 9.0 [0, 1, 1, 0] True
```

モデルとペナルティの重み 7 については [Formula によるモデル化](#knapsack)で説明しています。

限界になっていない関数は、エラーなしに最適解を枝刈りしてしまい、その実行は最適でない解に対して
`proven_optimal=True` を報告します。新しい限界は、列挙できるほど小さなインスタンスで列挙と照らし
合わせて確認してください。例外を送出する関数は実行を止め、`run` がその例外を送出します。NaN を返す
関数は `ValueError` になります。

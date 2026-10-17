---
source_sha256: d28379bf04f0
---

# Python で書いた問題

汎用ヒューリスティックは、Python で定義した問題でも動きます。`LocalSearch`、
`SimulatedAnnealing`、`BangBangSimulatedAnnealing`、`TabuSearch`、
`LateAcceptanceHillClimbing`、`RandomWalk`、`BeamSearch`、`PopulationAnnealing`、
`ParallelTempering`、`ReinforcementLearningSearch` がそれを受け付け、合成ヒューリスティック
(`VariableNeighborhoodSearch`、`Sequential`、`Iterated`、`Restart`、`GeneticAlgorithm`、
`BreakoutLocalSearch.from_parts`) も同様です。後述の ruin 系メソッドがあれば、
`AdaptiveLargeNeighborhoodSearch` と `GreedyRandomizedConstruction` も受け付けます。探索ループは
Rust に残り、問題に依存する処理はすべてあなたのオブジェクトを呼び出して行います。

(the-protocol)=
## プロトコル

継承すべき基底クラスはありません。次のメンバーを持つオブジェクトなら何でも動きます。

| 問題のメンバー | 意味 |
|---|---|
| `minimize` | `objective` を最小化するなら `True`、最大化するなら `False` |
| `new_solution(rng)` | ランダムな初期解 |
| `objective(solution)` | 解の値 (float) |
| `neighborhoods` | 名前から近傍オブジェクトへの対応 |

ヒューリスティックは `neighbor` 引数で名前を指定して近傍を選びます。1 つの問題が宣言できる近傍は
8 つまでです。

| 近傍のメンバー | 意味 |
|---|---|
| `neighbors(problem, solution)` | 移動の iterable |
| `apply(problem, solution, move)` | 移動後の解。新しいオブジェクトとして返す |
| `delta(problem, solution, move)` | 省略可。移動後の `objective` から移動前を引いた値 |
| `random_neighbor(problem, solution, rng)` | 省略可。移動を 1 つ。なければ `None` |
| `tabu_keys(problem, move)` | 省略可。その移動がタブーにするもの。`TabuSearch` に必要 |

解と移動はどんな Python オブジェクトでも構いません。探索は以前の解への参照を保持するので、`apply`
は受け取った解を変更してはいけません。タプルなどの不変な値を使えば自然にそうなります。

`rng` は実行のシードで初期化された `random.Random` です。グローバルな `random` モジュールではなく
これから乱数を引けば、`seed` を与えた実行は正確に再現されます。

`tabu_keys` はキーを 1 つ、またはキーのリストを返します。同じキーが 2 つの役割を果たします: どれか
1 つのキーがタブーである間はその移動もタブーで、移動を適用するとすべてのキーがタブー期間のあいだ
タブーになります。`tabu_keys` は解ではなく移動だけを受け取るので、品物が出ていく箱のように、キーに
必要な情報は移動自身に持たせます。

キーは非負の int、または非負の int 2 つか 3 つのタプルです。それ以外は `TypeError` になります。
タプルは 1 つのキーです: `(i, j)` はその組を禁止し、`[i, j]` は 2 つのキーで `i` と `j` をそれぞれ
禁止します。int はいくら大きくても構わず、`item * 10**15 + bin` も使えますが、同じことはタプル
`(item, bin)` のほうが素直に表せます。int とタプルは、数が一致していても衝突しません。空のリストを
返すと、その移動は決してタブーになりません。異なる近傍のキーは 1 つの記憶を共有するので、同じ変数
番号をキーにした反転と交換は互いを禁止します。

品物を移す移動では、品物だけをキーにすると、しばらくその品物を再び動かすことを禁止します。単純で、
たいていはそれで十分です。`(item, bin)` はその箱にあるその品物だけを禁止します。入れ物の番号を
キーにするのは、番号が変わらない場合だけにしてください。空になると後ろの箱の番号がすべてずれる
ような箱では、`(item, bin)` が間違った箱を禁止してしまいます。

すべての移動がタブーのとき、または `neighbors` が何も返さないとき、`TabuSearch` はその反復を棄却と
して数えて移動せず、`LocalSearch` は局所最適と同様に止まります。近傍より長いタブー期間を設定すると、
禁止が解けるまで探索は何もできなくなります。

`random_neighbor` が `None` を返すのは、その近傍に解からの移動がないことを意味します。
`SimulatedAnnealing`、`BangBangSimulatedAnnealing`、`LateAcceptanceHillClimbing` はそこで止まって
`run` が `RuntimeError` を送出し、`PopulationAnnealing` と `ParallelTempering` はその提案を飛ばして次に進み、`RandomWalk` は
そのステップを棄却として数えてやり直します。`None` を返すのは本当に移動がないときだけにしてください。
すべての品物が 1 つの箱に入っているときの箱の間の交換のように、ときどきだけ空になる近傍は、常に
移動を持つ近傍とまとめたほうが適しています。

`RunResult.solution` は、あなたのメソッドが作った解オブジェクトをそのまま返したものです。

(what-some-heuristics-need-besides)=
## 一部のヒューリスティックが追加で必要とするもの

| ヒューリスティック | 問題のメンバー |
|---|---|
| `GeneticAlgorithm` | `crossover(a, b, rng)`。2 つの解の子 |
| `GeneticAlgorithm`、`BreakoutLocalSearch.from_parts` | 省略可の `distance(a, b)`。非負の int。なければ `a == b` を 0、それ以外を 1 として数える |
| `AdaptiveLargeNeighborhoodSearch` | ruin 系メソッド |
| `GeneticAlgorithm(crossover="SubProblem")` | `extract_sub_problem(a, b)` と `lift_solution(a, b, sub_solution)` |
| `BranchAndBound` | `variable_bounds()`、`variable_values(solution)`、`solution_from_values(values)`、`restricted(bounds)`。{ref}`厳密解法 <on-a-problem-written-in-python>`を参照 |

`GeneticAlgorithm` は `crossover` を指定しなければ問題の `crossover` を使い、そのとき `sub_heuristic` も
指定しません。

### 部分問題による交叉

`crossover="SubProblem"` と `sub_heuristic` を指定すると、2 つの親の交叉を、より小さな問題を解くことで
行います。親の決定が食い違う部分だけからなり、一致する部分は固定した問題です。問題の 2 つのメソッドが
それを記述し、部分問題の探索は `sub_heuristic` が行います。

| 問題のメンバー | 返すもの |
|---|---|
| `extract_sub_problem(a, b)` | `a` と `b` が食い違う部分についての、Python で書いた問題。一致する部分は目的関数に畳み込む |
| `lift_solution(a, b, sub_solution)` | 全体の解。親が一致する部分と、残りは `sub_solution` から |

部分問題も同じプロトコルに従い、元の問題の近傍を同じ名前・同じ順で宣言します。`sub_heuristic` は元の
問題の近傍の名前で移動を指定するためです。近傍が異なる部分問題は `ValueError` になり、部分問題の
メソッドが送出した例外は、元の問題のメソッドの例外と同じく `run` から送出されます。これは optopus の
2 値の問題が使う交叉です。一致した変数を固定すると小さなインスタンスが残り、短い探索でよく解けます。

(ruin-methods)=
### ruin 系メソッド

ruin and recreate は、解から要素を取り出し、1 つずつ戻します。要素は int で、番号の付いた場所を持つ
入れ物 (経路、箱、1 つの巡回路) に入っています。操作の対象となる作業用のコピー (partial) は、
あなたのメソッドどうしで合意していればどんな Python オブジェクトでもよく、通常はリストです。

| 問題のメンバー | 意味 |
|---|---|
| `to_partial(solution)` | 解の新しい変更可能なコピー。他のメソッドはこれをその場で書き換える |
| `finish(partial)` | partial が表す解 |
| `elements(partial)` | 配置済みの要素 |
| `remove_all(partial, elements)` | これらを取り出す |
| `removal_gain(partial, element)` | それを取り出して得られる節約 |
| `relatedness(a, b)` | 2 つの要素の似ている度合い。小さいほど似ている |
| `num_buckets(partial)` | 要素を入れられる入れ物の数 |
| `num_places(partial, bucket)` | その入れ物が提供する位置の数 |
| `insertion_cost(partial, bucket, place, element)` | そこに入れるコスト |
| `insert(partial, bucket, place, element)` | そこに入れる |
| `partial_objective(partial)` | 省略可。`finish` を呼ばずに求めた目的関数値 |
| `repair_around(partial, anchors, rng)` | 省略可。戻したばかりの要素の周りでの局所探索 |

`to_partial` は必ずコピーしてください。リストを解と共有する partial は、探索の知らないところで解を
変えてしまいます。容量の違反はペナルティとして `insertion_cost` に含め、どこかの場所が必ず使える
ようにしてください。

コストは `minimize` の値によらずコストとして読まれるので、最大化問題では失うものの観点で表します。

- `removal_gain` は、要素を取り出すことで partial がどれだけ良くなるかで、取り出すと良くなるなら正
  です。worst removal は利得の大きい要素から取り出します。
- `insertion_cost` は、要素をそこに入れることで partial がどれだけ悪くなるかです。小さいほど良く、
  repair は各要素をこれが最小の場所に入れます。
- `relatedness` は数値 (int も可) を返します。Shaw removal は似た要素をまとめて取り出します。見るのは
  2 つの要素だけで partial は見ないので、要素が今どこにあるかではなく、インスタンスがそれらについて
  述べること (大きさ、位置、衝突) を測ります。
- `num_buckets` は、要素を今入れられる入れ物の数です。箱のように必要に応じて入れ物を開く問題では、
  ここに空の入れ物を 1 つ含め、repair が常に新しい入れ物を開けるようにします。探索は配置のたびに
  番号を読み直すので、入れ物の番号は、その時点の partial について `num_buckets`、`num_places`、
  `insertion_cost`、`insert` の間で一致していればよく、`remove_all` や `insert` のあとで変わっても
  構いません。空になった入れ物をその場に残し、新しい入れ物を最後に提供するのが、これを満たす最も
  簡単な方法です。
- `num_places` は、順序が意味を持たない入れ物 (箱、色のクラス) では `1`、列 (経路) では `len + 1`、
  巡回する列 (巡回路) では `len` です。
- `partial_objective` は `objective` と同じく問題自身の向きで表します。探索は受理した候補にだけ
  `finish` を呼び、それ以外では `partial_objective` を読むので、これが時間を節約するのは候補の
  ほとんどが棄却されるようになってからです。
- `repair_around` は、戻したばかりの要素のリストを `anchors` として受け取り、partial をその場で
  書き換えます。

repair は要素を 1 つ戻すたびに全入れ物の全位置を評価し、その 2 つの演算子はどちらも配置のたびに残りの
要素を評価し直します。greedy insertion は最も安い場所のコストが最小の要素を、regret insertion は
最も多くを失いうる要素を戻します。取り出した要素が `k` 個、位置が全部で `P` 個なら、どちらも
`insertion_cost` を約 `k² · P / 2` 回呼びます。これが 1 反復のコストなので、大きなインスタンスでは
`insertion_cost` を軽く、`removal_fraction` を小さくしてください。`k` は `removal_fraction · n` で、
最大 50 です。

(example)=
## 例

2-opt 近傍を持つ巡回セールスパーソン問題です。

```python
import math
import optopus


class TwoOpt:
    def neighbors(self, problem, tour):
        n = len(tour)
        return [(i, j) for i in range(1, n - 1) for j in range(i + 1, n)]

    def random_neighbor(self, problem, tour, rng):
        return tuple(sorted(rng.sample(range(1, len(tour)), 2)))

    def apply(self, problem, tour, move):
        i, j = move
        return tour[:i] + tour[i : j + 1][::-1] + tour[j + 1 :]

    def delta(self, problem, tour, move):
        i, j = move
        d = problem.dist
        a, b, c, e = tour[i - 1], tour[i], tour[j], tour[(j + 1) % len(tour)]
        return d(a, c) + d(b, e) - d(a, b) - d(c, e)

    def tabu_keys(self, problem, move):
        return move


class Tsp:
    minimize = True

    def __init__(self, points):
        self.points = points
        self.neighborhoods = {"TwoOpt": TwoOpt()}

    def dist(self, a, b):
        return math.dist(self.points[a], self.points[b])

    def new_solution(self, rng):
        rest = list(range(1, len(self.points)))
        rng.shuffle(rest)
        return (0, *rest)

    def objective(self, tour):
        return sum(self.dist(tour[i - 1], tour[i]) for i in range(len(tour)))


points = [(math.cos(k), math.sin(3 * k)) for k in range(30)]
sa = optopus.SimulatedAnnealing(
    neighbor="TwoOpt",
    initial_temperature=0.1,
    cooling_rate=0.9999,
    stop=optopus.StopCondition(max_iteration=50_000),
)
report = sa.run(Tsp(points), runs=5, seed=42)
print(report.best_objective, report.runs[0].solution)
```

(speed)=
## 速度

ステップごとに Python を呼び出すので、1 ステップのコストはそれが実行する Python コードとほぼ同じ
です。バインディング自体が加えるのは 1 マイクロ秒を大きく下回る時間です。

コストのほとんどは、省略可能な 2 つのメソッドで決まります。

- `delta` がなければ、各移動は `apply` と `objective` を呼んで評価されます。局所探索とタブー探索は
  毎ステップ近傍全体を評価します。
- `random_neighbor` がなければ、焼きなまし法、late acceptance、ランダムウォーク、population
  annealing は、移動を 1 つ選ぶためだけに毎ステップ `neighbors` から完全なリストを作ります。

移動をすぐに見つけられないときに `neighbors` にフォールバックする `random_neighbor` は、
フォールバックが起きるたびに、ないのと同じだけのコストがかかります。移動を直接見つけるようにして
ください。

両方があっても、新しい現在の解には自身の値が必要なので、ステップは受理した移動ごとに `apply` と
`objective` を 1 回ずつ呼びます。高価な `objective` のコストは受理率に比例して現れます。

(constraints-and-flat-objectives)=
## 制約と平坦な目的関数

汎用ヒューリスティックは `new_solution` と `apply` が返すどんな解も受け入れるので、厳しい制約は、
破れない移動によって構成上守るか、`objective` でペナルティを課すかのどちらかです。ペナルティには、
制約を破ることで移動が得られる最大の利得より大きな重みが必要です。`Formula` について
[ペナルティの重みの選び方](#choosing-a-penalty-weight)で説明しているとおりで、組み込みの
`VertexCover` と `GraphColoring` が `num_vertices + 1` を使うのも同じ理由です。

使った箱の数のように何かを数える目的関数は平坦です。ほとんどの移動は数を変えず、探索は箱を空にする
方向の移動とそれ以外を区別できません。箱の積載量の 2 乗和のように、進み具合に報いる 2 次的な項を、
数の 1 単位より小さく加えてください。容量 `C` の箱に合計の大きさ `T` が入っているなら、

```text
objective = bins_used − Σ min(load, C)² / (C · T + 1)
```

は、`Σ load² ≤ C · T` なので引く和が 1 未満に収まったまま、よく詰まった箱に報います。`min` は、
すでにペナルティを課している容量超過の箱が余分に得をしないようにします。意味のある差分はこの項の
大きさ、つまり箱 1 つの小さな割合になり、`SimulatedAnnealing` の温度はこの尺度に合わせて設定します
([SimulatedAnnealing](heuristics.md#simulatedannealing)を参照)。箱の間で 2 つの品物を交換するような、数を変えられない近傍は、数を変えられる近傍と
組み合わせる必要があります。

(errors)=
## エラー

問題は `run` の開始時にプロトコルと照合されます。メンバーが欠けていれば `TypeError`、知らない
`neighbor` や、`tabu_keys` のない近傍での `TabuSearch` は `ValueError` になります。

あなたのメソッドの中で例外が送出されると、探索は次のステップで止まり、`run` が同じ例外を送出します。
残りの実行は飛ばされます。Ctrl-C も同じように実行を中断します。

`WalkSat` や MaxCut 版の `BreakoutLocalSearch` のような問題専用ヒューリスティックは、それぞれの問題
でしか動かず、Python の問題では `ValueError` になります。`crossover` のない `GeneticAlgorithm` や
ruin 系メソッドのない `AdaptiveLargeNeighborhoodSearch` のように、必要なメンバーが欠けている
ヒューリスティックも同様で、欠けているものの名前をエラーに示します。

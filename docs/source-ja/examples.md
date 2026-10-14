---
source_sha256: 470a00bef764
---

# 例

これらの例では、ヒューリスティックを比較し、複数の実行を集計します。`optopus` が
[インストール](installation.md)済みであることを前提とします。

(aggregating-statistics-over-multiple-runs)=
## 複数の実行にわたる統計の集計

メタヒューリスティクスは確率的なので、1 回の実行で全体像が分かることはまれです。`runs=N` を渡して
探索を繰り返し、`RunReport` から集計した統計を読みます。

```python
import optopus

mc = optopus.MaxCut.from_edges([
    (0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0), (2, 3, 1.5), (0, 3, 2.0),
])

ts = optopus.TabuSearch(
    neighbor="Flip",
    tabu_tenure=(3, 7),
    stop=optopus.StopCondition(max_iteration=5_000),
)

report = ts.run(mc, runs=10, seed=1)

print(f"best  = {report.best_objective}")
print(f"avg   = {report.avg_objective:.2f}")
print(f"worst = {report.worst_objective}")
print(f"std   = {report.std_objective:.3f}")
print(f"avg time to best = {report.avg_time_to_best_secs * 1e3:.2f} ms")
```

(comparing-heuristics-on-the-same-problem)=
## 同じ問題でのヒューリスティックの比較

```python
import optopus

q = optopus.Qubo.from_entries([
    (0, 0, -1), (0, 1, 2), (1, 1, -1), (1, 2, 1), (2, 2, -2),
])
stop = optopus.StopCondition(max_iteration=5_000)

heuristics = {
    "LocalSearch": optopus.LocalSearch(neighbor="Flip", stop=stop),
    "SimulatedAnnealing": optopus.SimulatedAnnealing(
        neighbor="Flip", initial_temperature=5.0, cooling_rate=0.995, stop=stop
    ),
    "TabuSearch": optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 7), stop=stop),
    "LateAcceptanceHillClimbing": optopus.LateAcceptanceHillClimbing(
        neighbor="Flip", history_length=20, stop=stop
    ),
}

for name, h in heuristics.items():
    report = h.run(q, runs=5, seed=1)
    print(f"{name:28} best={report.best_objective} avg={report.avg_objective:.2f}")
```

(benchmarking-specialized-heuristics-on-a-random-instance)=
## ランダムなインスタンスでの専用ヒューリスティックのベンチマーク

問題専用ヒューリスティックと `VariableNeighborhoodSearch` は同じ `run` のインターフェースを持つので、
同じ比較のループにそのまま入れられます。インスタンスを `Graph` から生成すれば、ベンチマークは最初から
最後まで再現可能です。

```python
import optopus

mc = optopus.MaxCut.from_graph(
    optopus.Graph.erdos_renyi(500, 0.02, seed=42).with_random_weights((1, 10), seed=42)
)
stop = optopus.StopCondition(max_duration_secs=0.2)

heuristics = {
    "LocalSearch": optopus.LocalSearch(neighbor="Flip", stop=stop),
    "TabuSearch": optopus.TabuSearch(neighbor="Flip", tabu_tenure=(3, 50), stop=stop),
    "VariableNeighborhoodSearch": optopus.VariableNeighborhoodSearch(
        search=optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=200)),
        shakes=[
            optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=5)),
            optopus.RandomWalk("Flip", stop=optopus.StopCondition(max_iteration=20)),
        ],
        stop=stop,
    ),
    "BreakoutLocalSearch": optopus.BreakoutLocalSearch(
        tabu_tenure=(6, 100), t=1_000, l0=20, p0=0.8, q=0.5, stop=stop
    ),
    "PopulationAnnealing": optopus.PopulationAnnealing(population_size=20, stop=stop),
}

for name, h in heuristics.items():
    report = h.run(mc, runs=3, seed=1)
    print(f"{name:28} best={report.best_objective:.0f} avg={report.avg_objective:.1f}")
```

(more-examples)=
## その他の例

どのクラスにも、それぞれの節に実行できる例があります。

- [探索の実行と結果の読み方](running.md): `run`、シードと再現性、`StopCondition`、`improvement` を
  含む `RunReport` と `RunResult` の全フィールド。
- [組み込みの問題](#per-problem): 問題ごとに 1 つの例。
- [汎用](heuristics_generic.md)・[合成](heuristics_composed.md)・
  [問題専用](heuristics_specific.md)ヒューリスティックと[厳密解法](heuristics_exact.md):
  ヒューリスティックごとに 1 つの例。
- [グラフとテスト用インスタンス](graphs_and_instances.md): ランダムグラフ、ヒューリスティックが埋め
  込んだ最適値に到達する頻度の測定、カーネルを経由した求解。
- [Formula によるモデル化](#formula-examples)と [Python で書いた問題](#example)。

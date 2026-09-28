//! The run loop of optopus-py's `Heuristic.run(problem, runs, seed)`, ported as is.
//!
//! Copied from the `port-to-rust` skill of optopus-py, which mirrors `run_all` and
//! `derive_seed` in its `src/runner.rs` and `RunReport::from_runs` in `src/result.rs`.
//! Keep it identical: the per-run seeds, and so the comparison against the Python
//! code, depend on it.

use std::time::Instant;

use optopus::prelude::{Heuristic, ProblemTrait, SearchState};
use serde_json::{Value, json};

/// The seed of run `run_index`. Run 0 uses the master seed itself.
pub fn derive_seed(master: u64, run_index: usize) -> u64 {
    master.wrapping_add((run_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// One run, with the fields of optopus-py's `RunResult`.
pub struct RunResult {
    pub seed: Option<u64>,
    pub best_objective: f64,
    pub initial_objective: f64,
    pub improvement: f64,
    pub best_iteration: u64,
    pub n_accepted: u64,
    pub n_rejected: u64,
    pub n_best_updates: u64,
    pub time_to_best_secs: f64,
    pub total_time_secs: f64,
    pub solution: Value,
}

/// All runs, with the summary optopus-py's `RunReport` computes from them.
pub struct RunReport {
    pub minimize: bool,
    pub runs: Vec<RunResult>,
}

/// Runs a fresh heuristic from `build` `runs` times on `instance`.
///
/// - `build` returns the heuristic boxed, as optopus-py builds every one, so nested
///   heuristics (`Box<dyn Heuristic<P>>`) and plain ones go through the same call.
/// - `minimize` is the direction the report ranks `obj` in.
/// - `obj` reads a solution's objective the way optopus-py reports it (see the
///   skill's `references/api-mapping.md`, "Objective per problem").
/// - `encode` turns a solution into JSON the way optopus-py decodes it to Python.
pub fn run_all<'h, P>(
    instance: &P,
    build: impl Fn() -> Box<dyn Heuristic<P> + 'h>,
    minimize: bool,
    runs: usize,
    seed: Option<u64>,
    obj: impl Fn(&P::Solution) -> f64,
    encode: impl Fn(&P::Solution) -> Value,
) -> RunReport
where
    P: ProblemTrait,
{
    assert!(runs >= 1, "'runs' must be at least 1");
    let mut results = Vec::with_capacity(runs);
    for run_index in 0..runs {
        let run_seed = seed.map(|m| derive_seed(m, run_index));
        let mut heuristic = build();

        let start = Instant::now();
        let mut state = match run_seed {
            Some(s) => SearchState::new_with_seed(instance, s),
            None => SearchState::new(instance),
        };
        let initial_objective = obj(&state.initial_solution);
        // optopus-py ignores the error too and reports the best solution reached.
        let _ = heuristic.run(&mut state);
        let total_time = start.elapsed();

        let best_objective = obj(&state.best_solution);
        let raw_diff = best_objective - initial_objective;
        results.push(RunResult {
            seed: run_seed,
            best_objective,
            initial_objective,
            improvement: if minimize { -raw_diff } else { raw_diff },
            best_iteration: state.best_iteration,
            n_accepted: state.n_accepted,
            n_rejected: state.n_rejected,
            n_best_updates: state.n_best_updates,
            time_to_best_secs: state
                .best_time
                .saturating_duration_since(start)
                .as_secs_f64(),
            total_time_secs: total_time.as_secs_f64(),
            solution: encode(&state.best_solution),
        });
    }
    RunReport {
        minimize,
        runs: results,
    }
}

impl RunReport {
    fn objectives(&self) -> impl Iterator<Item = f64> + '_ {
        self.runs.iter().map(|r| r.best_objective)
    }

    fn mean(&self, f: impl Fn(&RunResult) -> f64) -> f64 {
        self.runs.iter().map(f).sum::<f64>() / self.runs.len() as f64
    }

    pub fn best_objective(&self) -> f64 {
        if self.minimize {
            self.objectives().fold(f64::INFINITY, f64::min)
        } else {
            self.objectives().fold(f64::NEG_INFINITY, f64::max)
        }
    }

    pub fn worst_objective(&self) -> f64 {
        if self.minimize {
            self.objectives().fold(f64::NEG_INFINITY, f64::max)
        } else {
            self.objectives().fold(f64::INFINITY, f64::min)
        }
    }

    pub fn avg_objective(&self) -> f64 {
        self.mean(|r| r.best_objective)
    }

    /// Population standard deviation, as optopus-py computes it.
    pub fn std_objective(&self) -> f64 {
        let avg = self.avg_objective();
        self.mean(|r| (r.best_objective - avg).powi(2)).sqrt()
    }

    pub fn avg_total_time_secs(&self) -> f64 {
        self.mean(|r| r.total_time_secs)
    }

    /// The report in the JSON layout the skill's `compare.py` reads.
    pub fn to_json(&self) -> Value {
        let runs: Vec<Value> = self
            .runs
            .iter()
            .map(|r| {
                json!({
                    "seed": r.seed,
                    "best_objective": r.best_objective,
                    "initial_objective": r.initial_objective,
                    "improvement": r.improvement,
                    "best_iteration": r.best_iteration,
                    "n_accepted": r.n_accepted,
                    "n_rejected": r.n_rejected,
                    "n_best_updates": r.n_best_updates,
                    "time_to_best_secs": r.time_to_best_secs,
                    "total_time_secs": r.total_time_secs,
                    "solution": r.solution,
                })
            })
            .collect();
        json!({
            "source": "rust",
            "minimize": self.minimize,
            "best_objective": self.best_objective(),
            "avg_objective": self.avg_objective(),
            "worst_objective": self.worst_objective(),
            "std_objective": self.std_objective(),
            "avg_total_time_secs": self.avg_total_time_secs(),
            "runs": runs,
        })
    }

    /// Prints the summary, and writes the JSON report when `--json <path>` was passed.
    /// A `{label}` in the path is replaced by `label`, so a port with several runs
    /// writes one file per run: `--json 'out/{label}.json'`.
    pub fn emit(&self, label: &str) {
        println!(
            "{label}: best={} avg={} worst={} std={} ({} runs, {:.3}s/run)",
            self.best_objective(),
            self.avg_objective(),
            self.worst_objective(),
            self.std_objective(),
            self.runs.len(),
            self.avg_total_time_secs(),
        );
        if let Some(path) = json_path() {
            let path = path.replace("{label}", label);
            if let Some(dir) = std::path::Path::new(&path).parent() {
                std::fs::create_dir_all(dir)
                    .unwrap_or_else(|e| panic!("cannot create {dir:?}: {e}"));
            }
            let text = serde_json::to_string_pretty(&self.to_json()).expect("serializable");
            std::fs::write(&path, text).unwrap_or_else(|e| panic!("cannot write {path}: {e}"));
        }
    }
}

/// The value of `--json <path>` on the command line, if any.
fn json_path() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--json" {
            return args.next();
        }
    }
    None
}

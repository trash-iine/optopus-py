//! Ported from `{{SOURCE}}` (optopus-py) by the `port-to-rust` skill.
//!
//! Run with `cargo run --release`, and add `-- --json report.json` to write the
//! report the skill's `compare.py` checks against the Python original.

mod harness;

use optopus::prelude::*;
use serde_json::json;

fn main() {
    // Replace this with the port. It is the Python
    //
    //     mc = optopus.MaxCut.from_edges([(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)])
    //     ls = optopus.LocalSearch("Flip", stop=optopus.StopCondition(max_iteration=10_000))
    //     report = ls.run(mc, runs=5, seed=42)
    let problem = MaxCut::from_edges(vec![(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)]);
    let stop = StopCondition::new(Some(10_000), None, None);
    let report = harness::run_all(
        &problem,
        || Box::new(LocalSearch::<MaxCutFlipNeighbor>::new(stop.clone())),
        false,
        5,
        Some(42),
        |s| s.objective as f64,
        |s| json!(s.x),
    );
    report.emit("LocalSearch");
}

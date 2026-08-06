//! Python bindings for the optopus combinatorial optimization library.
//!
//! Exposes problem types (MaxCut, Qubo, Sat, VertexCover, TspWithCoordinates,
//! JobShopScheduling, Formula), the Graph type behind the graph-based problems,
//! generic heuristics (LocalSearch, SimulatedAnnealing, TabuSearch,
//! LateAcceptanceHillClimbing, RandomWalk, BangBangSimulatedAnnealing, BeamSearch,
//! VariableNeighborhoodSearch) and problem-specific ones (WalkSat,
//! PopulationAnnealing, BreakoutLocalSearch, RlBreakoutLocalSearch,
//! LinKernighanHelsgaun) as Python classes.

mod graph;
mod heuristic;
mod problem;
mod result;
mod runner;
mod stop_condition;

use pyo3::prelude::*;

#[pymodule]
fn optopus(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<stop_condition::StopCondition>()?;
    m.add_class::<result::RunResult>()?;
    m.add_class::<result::RunReport>()?;
    m.add_class::<graph::Graph>()?;
    m.add_class::<problem::MaxCut>()?;
    m.add_class::<problem::Qubo>()?;
    m.add_class::<problem::Sat>()?;
    m.add_class::<problem::VertexCover>()?;
    m.add_class::<problem::TspWithCoordinates>()?;
    m.add_class::<problem::JobShopScheduling>()?;
    m.add_class::<problem::Formula>()?;
    m.add_class::<heuristic::LocalSearch>()?;
    m.add_class::<heuristic::SimulatedAnnealing>()?;
    m.add_class::<heuristic::TabuSearch>()?;
    m.add_class::<heuristic::LateAcceptanceHillClimbing>()?;
    m.add_class::<heuristic::RandomWalk>()?;
    m.add_class::<heuristic::BangBangSimulatedAnnealing>()?;
    m.add_class::<heuristic::BeamSearch>()?;
    m.add_class::<heuristic::VariableNeighborhoodSearch>()?;
    m.add_class::<heuristic::WalkSat>()?;
    m.add_class::<heuristic::PopulationAnnealing>()?;
    m.add_class::<heuristic::BreakoutLocalSearch>()?;
    m.add_class::<heuristic::RlBreakoutLocalSearch>()?;
    m.add_class::<heuristic::LinKernighanHelsgaun>()?;
    Ok(())
}

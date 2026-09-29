//! Python bindings for the optopus combinatorial optimization library.
//!
//! Exposes problem types (MaxCut, Qubo, Sat, VertexCover, Tsp, JobShopScheduling,
//! Vrp with its VehicleType, GraphColoring, Formula), the Graph type behind the
//! graph-based problems, generic heuristics (LocalSearch, SimulatedAnnealing, TabuSearch,
//! LateAcceptanceHillClimbing, RandomWalk, BangBangSimulatedAnnealing, BeamSearch,
//! PopulationAnnealing, ReinforcementLearningSearch), composed ones built from other
//! heuristics (VariableNeighborhoodSearch, Sequential, Iterated, Restart,
//! GeneticAlgorithm, BreakoutLocalSearch.from_parts) and problem-specific ones
//! (WalkSat, BreakoutLocalSearch, LinKernighanHelsgaun,
//! AdaptiveLargeNeighborhoodSearch, HybridGeneticSearch) as Python classes, plus the
//! MaxCutKernel reduction and the PlantedMaxCut instance generators. The generic and
//! composed heuristics, and ALNS, also run on problems written in Python.

mod graph;
mod heuristic;
mod problem;
mod python_problem;
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
    m.add_class::<problem::MaxCutKernel>()?;
    m.add_class::<problem::PlantedMaxCut>()?;
    m.add_class::<problem::Qubo>()?;
    m.add_class::<problem::Sat>()?;
    m.add_class::<problem::VertexCover>()?;
    m.add_class::<problem::Tsp>()?;
    m.add_class::<problem::JobShopScheduling>()?;
    m.add_class::<problem::Vrp>()?;
    m.add_class::<problem::VehicleType>()?;
    m.add_class::<problem::GraphColoring>()?;
    m.add_class::<problem::Formula>()?;
    m.add_class::<heuristic::LocalSearch>()?;
    m.add_class::<heuristic::SimulatedAnnealing>()?;
    m.add_class::<heuristic::TabuSearch>()?;
    m.add_class::<heuristic::LateAcceptanceHillClimbing>()?;
    m.add_class::<heuristic::RandomWalk>()?;
    m.add_class::<heuristic::BangBangSimulatedAnnealing>()?;
    m.add_class::<heuristic::BeamSearch>()?;
    m.add_class::<heuristic::VariableNeighborhoodSearch>()?;
    m.add_class::<heuristic::Sequential>()?;
    m.add_class::<heuristic::Iterated>()?;
    m.add_class::<heuristic::Restart>()?;
    m.add_class::<heuristic::GeneticAlgorithm>()?;
    m.add_class::<heuristic::ReinforcementLearningSearch>()?;
    m.add_class::<heuristic::WalkSat>()?;
    m.add_class::<heuristic::PopulationAnnealing>()?;
    m.add_class::<heuristic::BreakoutLocalSearch>()?;
    m.add_class::<heuristic::LinKernighanHelsgaun>()?;
    m.add_class::<heuristic::AdaptiveLargeNeighborhoodSearch>()?;
    m.add_class::<heuristic::HybridGeneticSearch>()?;
    Ok(())
}

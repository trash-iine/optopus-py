//! Turns a solution given from Python into the problem's own solution type.
//!
//! Each function reads the form `RunResult.solution` has for its problem and lets optopus check
//! that the solution fits the instance. `run` uses them for its `initial_solution`, and the
//! problems' scoring methods to score a solution through the same checks. `arg` names the
//! argument in the error messages.

use optopus::error::OptError;
use optopus::problem::{
    BinPacking, BinPackingSolution, FlowShop, FlowShopSolution, FormulaProblem, FormulaSolution,
    GraphColoring, GraphColoringSolution, JobShopScheduling, JobShopSolution, MaxCut,
    MaxCutSolution, Qap, QapSolution, Qubo, QuboSolution, Sat, SatSolution, Tsp, TspSolution,
    VertexCover, VertexCoverSolution, Vrp, VrpSolution,
};
use optopus::trait_defs::BinaryProblem;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBool;

/// The error for a solution optopus rejects for this instance.
fn misfit(arg: &str) -> impl FnOnce(OptError) -> PyErr + '_ {
    move |err| PyValueError::new_err(format!("'{arg}' does not fit the problem: {err}"))
}

/// A 0/1 assignment, given as bools or as the ints 0 and 1.
fn bits(value: &Bound<'_, PyAny>, arg: &str) -> PyResult<Vec<bool>> {
    value
        .try_iter()
        .map_err(|_| PyTypeError::new_err(format!("'{arg}' must be a sequence of 0/1 values")))?
        .enumerate()
        .map(|(i, item)| {
            let item = item?;
            if let Ok(b) = item.cast::<PyBool>() {
                return Ok(b.is_true());
            }
            match item.extract::<i64>() {
                Ok(0) => Ok(false),
                Ok(1) => Ok(true),
                _ => Err(PyValueError::new_err(format!(
                    "'{arg}' entry {i} must be 0, 1, True or False"
                ))),
            }
        })
        .collect()
}

/// A binary problem's assignment, one value per variable.
fn binary<P: BinaryProblem>(
    prob: &P,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<P::Solution> {
    prob.solution_from_assignment(&bits(value, arg)?)
        .map_err(misfit(arg))
}

pub fn max_cut(prob: &MaxCut, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<MaxCutSolution> {
    binary(prob, value, arg)
}

pub fn qubo(prob: &Qubo, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<QuboSolution> {
    binary(prob, value, arg)
}

pub fn sat(prob: &Sat, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<SatSolution> {
    binary(prob, value, arg)
}

pub fn vertex_cover(
    prob: &VertexCover,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<VertexCoverSolution> {
    binary(prob, value, arg)
}

/// A tour, a permutation of the cities.
pub fn tsp(prob: &Tsp, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<TspSolution> {
    let tour: Vec<usize> = value.extract()?;
    let objective = prob.calculate_tour_length(&tour).map_err(misfit(arg))?;
    Ok(TspSolution { tour, objective })
}

/// An operation sequence, each job appearing once per operation it has.
pub fn job_shop(
    prob: &JobShopScheduling,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<JobShopSolution> {
    let operations: Vec<usize> = value.extract()?;
    let (objective, completion_times) = prob.decode(&operations).map_err(misfit(arg))?;
    Ok(JobShopSolution {
        operations,
        objective,
        completion_times,
    })
}

/// One route per vehicle, empty routes included.
pub fn vrp(prob: &Vrp, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<VrpSolution> {
    prob.solution_from_routes(value.extract()?)
        .map_err(misfit(arg))
}

/// The color of each vertex.
pub fn graph_coloring(
    prob: &GraphColoring,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<GraphColoringSolution> {
    prob.solution_from_colors(value.extract()?)
        .map_err(misfit(arg))
}

/// The order the jobs run in.
pub fn flow_shop(
    prob: &FlowShop,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<FlowShopSolution> {
    prob.solution_from_sequence(value.extract()?)
        .map_err(misfit(arg))
}

/// The location of each facility.
pub fn qap(prob: &Qap, value: &Bound<'_, PyAny>, arg: &str) -> PyResult<QapSolution> {
    prob.solution_from_assignment(value.extract()?)
        .map_err(misfit(arg))
}

/// The bin of each item.
pub fn bin_packing(
    prob: &BinPacking,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<BinPackingSolution> {
    prob.solution_from_bins(value.extract()?)
        .map_err(misfit(arg))
}

/// The value of each variable.
pub fn formula(
    prob: &FormulaProblem,
    value: &Bound<'_, PyAny>,
    arg: &str,
) -> PyResult<FormulaSolution> {
    prob.solution_from(value.extract()?).map_err(misfit(arg))
}

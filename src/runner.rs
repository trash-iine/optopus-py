use std::sync::Arc;
use std::time::Instant;

use optopus::building_blocks::representation::BinaryRelaxation;
use optopus::heuristic::reinforcement_learning::feature::NUM_FEATURES;
use optopus::heuristic::{
    AdaptiveLargeNeighborhoodSearch, AdaptivePerturbation, BreakoutLocalSearch, GeneticAlgorithm,
    GreedyRandomizedConstruction as OptGreedyRandomizedConstruction, Iterated, IteratedGreedy,
    ParallelTempering as OptParallelTempering, ParentSelection,
    PopulationAnnealing as OptPopulationAnnealing, ReinforcementLearningSearch, Restart,
    RewardShaping, Sequential, SubProblemBasedCrossover,
};
use optopus::prelude::{
    BangBangSimulatedAnnealing, BeamSearch, EnabledTabu, Evaluate, Heuristic,
    HybridGeneticSearchForVrp, LateAcceptanceHillClimbing, LinKernighanHelsgaunForTsp, LocalSearch,
    MoveToNeighbor, ProblemTrait, RandomWalk, Rankable, SearchState, SimulatedAnnealing,
    TabuSearch, VariableNeighborhoodSearch, WalkSatForSat, bls_for_max_cut,
};
use optopus::problem::flow_shop::FlowShopInsertionDescent;
use optopus::problem::tsp::AnchoredTourDescent;
use optopus::problem::vrp::AnchoredRouteDescent;
use optopus::problem::{
    BinPacking as OptBinPacking, BinPackingGroupCrossover, BinPackingRelocateNeighbor,
    BinPackingSwapNeighbor, FlowShop as OptFlowShop, FlowShopInsertNeighbor,
    FlowShopOrderCrossover, FlowShopSwapNeighbor, FormulaProblem,
    GraphColoring as OptGraphColoring, GraphColoringRecolorNeighbor, GraphColoringSwapNeighbor,
    GraphColoringUniformCrossover, IntChangeNeighbor, IntCrossover, IntReverseNeighbor,
    IntSwapNeighbor, JobShopPpxCrossover, JobShopRelocateNeighbor, JobShopScheduling as OptJobShop,
    JobShopSwapNeighbor, MaxCut as OptMaxCut, MaxCutFlipNeighbor, MaxCutSwapNeighbor,
    MaxCutUniformCrossover, Qap as OptQap, QapOrderCrossover, QapSwapNeighbor, Qubo as OptQubo,
    QuboFlipNeighbor, QuboSwapNeighbor, QuboUniformCrossover, Sat as OptSat, SatFlipNeighbor,
    SatSwapNeighbor, SatUniformCrossover, Tsp as OptTsp, TspOrderCrossover, TspRelocateNeighbor,
    TspTwoOptNeighbor, VertexCover as OptVc, VertexCoverFlipNeighbor, VertexCoverSwapNeighbor,
    VertexCoverUniformCrossover, Vrp as OptVrp, VrpOrderCrossover, VrpRelocateNeighbor,
    VrpSwapNeighbor, VrpTwoOptNeighbor,
};
use optopus::problem::{EigenvalueRelaxation, IntervalRelaxation};
use optopus::search_state::TrajectoryPoint;
use optopus::trait_defs::{
    BranchSpace, Crossover, Distance, FixVariables, LocalRepair, Ruinable, SubProblemExtractable,
};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use rand::rngs::SmallRng;

use crate::branch_and_bound::{self as bnb, PyRelaxation, RelaxationSpec};
use crate::details::{self, Details};
use crate::error;
use crate::heuristic as py_heuristic;
use crate::initial;
use crate::problem::{
    BinPacking, FlowShop, Formula, GraphColoring, JobShopScheduling, MaxCut, Qap, Qubo, Sat, Tsp,
    VertexCover, Vrp,
};
use crate::progress::{self, Reported};
use crate::python_problem::{Guarded, MAX_NEIGHBORHOODS, PyCrossover, PyMove, PyProblem, PyRepair};
use crate::result::{RunReport, RunResult};
use crate::stop_condition::StopCondition;

/// Algorithm selection plus algorithm-specific parameters.
///
/// Variants split into three groups: generic heuristics that work on any
/// problem through a neighborhood, metaheuristics that nest other specs
/// ([`build_nested`] builds those once for every problem), and problem-specific
/// algorithms that only one `build_*` function can construct.
pub enum HeuristicKind {
    LocalSearch,
    SimulatedAnnealing {
        initial_temperature: f64,
        cooling_rate: f64,
    },
    TabuSearch {
        tabu_tenure: (u64, u64),
    },
    LateAcceptance {
        history_length: usize,
    },
    RandomWalk,
    BangBangSimulatedAnnealing {
        initial_temperature: f64,
        cooling_rate: f64,
        min_wave_threshold: f64,
        max_wave_threshold: f64,
    },
    BeamSearch {
        beam_width: usize,
    },
    VariableNeighborhoodSearch {
        search: Box<HeuristicSpec>,
        shakes: Vec<HeuristicSpec>,
    },
    Sequential {
        steps: Vec<HeuristicSpec>,
    },
    Iterated {
        search: Box<HeuristicSpec>,
        perturbation: Box<HeuristicSpec>,
    },
    Restart {
        inner: Box<HeuristicSpec>,
        restart: StopCondition,
    },
    GeneticAlgorithm {
        population_size: usize,
        mutation: Box<HeuristicSpec>,
        init_improvement: Option<Box<HeuristicSpec>>,
        crossover: Option<String>,
        sub_heuristic: Option<Box<HeuristicSpec>>,
        parent_selection: ParentSelection,
    },
    ReinforcementLearningSearch {
        learning_rate: f64,
        softmax_temperature: f64,
        reward_shaping: RewardShaping,
        policy_weights: Option<[f64; NUM_FEATURES]>,
        max_candidates: Option<usize>,
    },
    BranchAndBound {
        heuristic: Box<HeuristicSpec>,
        relaxation: RelaxationSpec,
    },
    /// Breakout local search assembled from other specs, for any problem.
    ComposedBreakoutLocalSearch {
        tabu_tenure: (u64, u64),
        t: u64,
        l0: u64,
        p0: f64,
        descent: Box<HeuristicSpec>,
        random: Box<HeuristicSpec>,
        directed: Vec<(HeuristicSpec, f64)>,
    },
    WalkSat {
        noise: f64,
        adaptive: bool,
    },
    PopulationAnnealing {
        population_size: usize,
        initial_beta: f64,
        delta_beta: f64,
        sweeps_per_step: usize,
        reset_period: Option<usize>,
        sweep_length: Option<usize>,
    },
    ParallelTempering {
        num_replicas: usize,
        beta_min: f64,
        beta_max: f64,
        sweeps_per_exchange: usize,
        sweep_length: Option<usize>,
    },
    BreakoutLocalSearch {
        tabu_tenure: (u64, u64),
        t: u64,
        l0: u64,
        p0: f64,
        q: f64,
    },
    LinKernighanHelsgaun {
        num_neighbors: usize,
        max_depth: usize,
    },
    AdaptiveLargeNeighborhoodSearch {
        removal_fraction: f64,
        cooling_rate: f64,
        tuning: AlnsTuning,
    },
    IteratedGreedy {
        removal_count: usize,
        temperature_factor: f64,
        /// A fixed temperature, in place of the flow shop's one from `temperature_factor`.
        temperature: Option<f64>,
        local_repair: bool,
    },
    GreedyRandomizedConstruction {
        alpha: f64,
    },
    HybridGeneticSearch {
        min_population_size: usize,
        generation_size: usize,
        granularity: usize,
        target_feasible: f64,
        restart_generations: Option<u64>,
        tuning: HgsTuning,
    },
}

/// What `AdaptiveLargeNeighborhoodSearch` sets beyond its removal share and cooling.
#[derive(Clone, Copy)]
pub struct AlnsTuning {
    /// The rewards for a new best, a better incumbent and an accepted worse candidate.
    pub scores: (f64, f64, f64),
    pub segment_length: u64,
    pub reaction: f64,
    pub max_removal: usize,
    /// Whether the problem's own descent follows each recreation.
    pub local_repair: bool,
}

/// What `HybridGeneticSearch` sets beyond its constructor's arguments.
#[derive(Clone, Copy)]
pub struct HgsTuning {
    pub n_elite: usize,
    pub n_closest: usize,
    /// `(period, increase, decrease, tolerance)`.
    pub penalty_adaptation: (u32, f64, f64, f64),
    pub repair_probability: f64,
    /// `(factor, nearest_neighbor_every)`.
    pub initial_population: (usize, usize),
    pub descent_passes: usize,
}

impl HeuristicKind {
    /// The Python class name, used to build error messages.
    fn name(&self) -> &'static str {
        match self {
            Self::LocalSearch => "LocalSearch",
            Self::SimulatedAnnealing { .. } => "SimulatedAnnealing",
            Self::TabuSearch { .. } => "TabuSearch",
            Self::LateAcceptance { .. } => "LateAcceptanceHillClimbing",
            Self::RandomWalk => "RandomWalk",
            Self::BangBangSimulatedAnnealing { .. } => "BangBangSimulatedAnnealing",
            Self::BeamSearch { .. } => "BeamSearch",
            Self::VariableNeighborhoodSearch { .. } => "VariableNeighborhoodSearch",
            Self::Sequential { .. } => "Sequential",
            Self::Iterated { .. } => "Iterated",
            Self::Restart { .. } => "Restart",
            Self::GeneticAlgorithm { .. } => "GeneticAlgorithm",
            Self::ReinforcementLearningSearch { .. } => "ReinforcementLearningSearch",
            Self::BranchAndBound { .. } => "BranchAndBound",
            Self::ComposedBreakoutLocalSearch { .. } => "BreakoutLocalSearch",
            Self::WalkSat { .. } => "WalkSat",
            Self::PopulationAnnealing { .. } => "PopulationAnnealing",
            Self::ParallelTempering { .. } => "ParallelTempering",
            Self::BreakoutLocalSearch { .. } => "BreakoutLocalSearch",
            Self::LinKernighanHelsgaun { .. } => "LinKernighanHelsgaun",
            Self::AdaptiveLargeNeighborhoodSearch { .. } => "AdaptiveLargeNeighborhoodSearch",
            Self::HybridGeneticSearch { .. } => "HybridGeneticSearch",
            Self::IteratedGreedy { .. } => "IteratedGreedy",
            Self::GreedyRandomizedConstruction { .. } => "GreedyRandomizedConstruction",
        }
    }

    /// The problem this kind is restricted to, or `None` when it is generic.
    fn only_for(&self) -> Option<&'static str> {
        match self {
            Self::WalkSat { .. } => Some("Sat"),
            Self::BreakoutLocalSearch { .. } => Some("MaxCut"),
            Self::LinKernighanHelsgaun { .. } => Some("Tsp"),
            Self::AdaptiveLargeNeighborhoodSearch { .. } => {
                Some("Vrp, Tsp, FlowShop, BinPacking and Python problems with the ruin methods")
            }
            Self::HybridGeneticSearch { .. } => Some("Vrp"),
            Self::IteratedGreedy { .. } => {
                Some("FlowShop, Tsp, Vrp, BinPacking and Python problems with the ruin methods")
            }
            Self::GreedyRandomizedConstruction { .. } => {
                Some("Tsp, Vrp, FlowShop, BinPacking and Python problems with the ruin methods")
            }
            Self::BranchAndBound { .. } => Some(
                "MaxCut, Qubo, Sat, VertexCover, Formula and Python problems with the branching \
                 methods",
            ),
            _ => None,
        }
    }
}

/// Fully-resolved heuristic specification handed to the dispatcher.
pub struct HeuristicSpec {
    pub kind: HeuristicKind,
    pub neighbor: String,
    pub stop: StopCondition,
}

/// Builds a boxed heuristic for a single problem type. Each problem has exactly
/// one of these; it is also the recursion step for nested specs. A closure
/// rather than a `fn` so a Python problem can resolve neighbor names against
/// its own neighborhoods.
type BuildFn<'b, P> = &'b dyn Fn(&HeuristicSpec) -> Result<Box<dyn Heuristic<P>>, String>;

/// Makes a problem's GA crossover from its name (`None` picks the problem's
/// default), handing a `SubProblem` crossover the heuristic that solves each
/// sub-problem.
type CrossoverFn<'b, P> = &'b dyn Fn(
    Option<&str>,
    Option<Box<dyn Heuristic<P>>>,
) -> Result<Box<dyn Crossover<P>>, String>;

/// Builds the heuristics that work on any problem `P` through neighbor type `N`.
fn build_generic<P, N>(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<P>>, String>
where
    P: ProblemTrait + 'static,
    P::Solution: Evaluate,
    N: MoveToNeighbor<P> + Rankable + Evaluate + Clone + EnabledTabu + 'static,
{
    let cond = spec.stop.to_opt();
    match &spec.kind {
        HeuristicKind::LocalSearch => Ok(Box::new(LocalSearch::<N>::new(cond))),
        HeuristicKind::SimulatedAnnealing {
            initial_temperature,
            cooling_rate,
        } => Ok(Box::new(SimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
        ))),
        HeuristicKind::TabuSearch { tabu_tenure } => {
            Ok(Box::new(TabuSearch::<N>::new(cond, *tabu_tenure)))
        }
        HeuristicKind::LateAcceptance { history_length } => Ok(Box::new(
            LateAcceptanceHillClimbing::<N>::new(cond, *history_length),
        )),
        HeuristicKind::RandomWalk => Ok(Box::new(RandomWalk::<N>::new(cond))),
        HeuristicKind::BangBangSimulatedAnnealing {
            initial_temperature,
            cooling_rate,
            min_wave_threshold,
            max_wave_threshold,
        } => Ok(Box::new(BangBangSimulatedAnnealing::<N>::new(
            cond,
            *initial_temperature,
            *cooling_rate,
            *min_wave_threshold,
            *max_wave_threshold,
        ))),
        HeuristicKind::BeamSearch { beam_width } => {
            Ok(Box::new(BeamSearch::<P, N>::new(cond, *beam_width)))
        }
        HeuristicKind::ReinforcementLearningSearch {
            learning_rate,
            softmax_temperature,
            reward_shaping,
            policy_weights,
            max_candidates,
        } => {
            let rl = ReinforcementLearningSearch::<N>::new(
                cond,
                *learning_rate,
                *softmax_temperature,
                reward_shaping.clone(),
                *max_candidates,
            );
            let rl = match policy_weights {
                Some(w) => rl.with_policy_weights(*w),
                None => rl,
            };
            Ok(details::recording(rl, |rl| Details {
                policy_weights: Some(rl.policy.weights.to_vec()),
                ..Details::default()
            }))
        }
        HeuristicKind::PopulationAnnealing {
            population_size,
            initial_beta,
            delta_beta,
            sweeps_per_step,
            reset_period,
            sweep_length,
        } => {
            let pa = OptPopulationAnnealing::<P, N>::new(
                cond,
                *population_size,
                *initial_beta,
                *delta_beta,
                *sweeps_per_step,
                *reset_period,
            );
            Ok(Box::new(match sweep_length {
                Some(length) => pa.with_sweep_length(*length),
                None => pa,
            }))
        }
        HeuristicKind::ParallelTempering {
            num_replicas,
            beta_min,
            beta_max,
            sweeps_per_exchange,
            sweep_length,
        } => {
            let pt = OptParallelTempering::<P, N>::new(
                cond,
                *num_replicas,
                *beta_min,
                *beta_max,
                *sweeps_per_exchange,
            );
            let pt = match sweep_length {
                Some(length) => pt.with_sweep_length(*length),
                None => pt,
            };
            Ok(details::recording(pt, |pt| Details {
                betas: Some(pt.betas().to_vec()),
                exchange_acceptance: pt.exchange_acceptance(),
                ..Details::default()
            }))
        }
        other => Err(unsupported(other)),
    }
}

/// Builds the kinds that carry no neighborhood of their own, so they can be
/// resolved before a problem picks its concrete neighbor type. Returns `None`
/// when `spec` has to go through that neighbor match instead.
fn build_nested<P>(
    spec: &HeuristicSpec,
    recur: BuildFn<'_, P>,
    crossover: CrossoverFn<'_, P>,
) -> Result<Option<Box<dyn Heuristic<P>>>, String>
where
    P: ProblemTrait + 'static,
    P::Solution: Distance + Evaluate + 'static,
{
    let cond = spec.stop.to_opt();
    let built: Box<dyn Heuristic<P>> = match &spec.kind {
        HeuristicKind::VariableNeighborhoodSearch { search, shakes } => {
            let search = recur(search)?;
            let shakes = shakes.iter().map(recur).collect::<Result<Vec<_>, _>>()?;
            Box::new(VariableNeighborhoodSearch::new(cond, search, shakes))
        }
        HeuristicKind::Sequential { steps } => {
            let steps = steps.iter().map(recur).collect::<Result<Vec<_>, _>>()?;
            Box::new(Sequential::new(cond, steps))
        }
        HeuristicKind::Iterated {
            search,
            perturbation,
        } => Box::new(Iterated::new(cond, recur(search)?, recur(perturbation)?)),
        HeuristicKind::Restart { inner, restart } => {
            Box::new(Restart::new(cond, recur(inner)?, restart.to_opt()))
        }
        HeuristicKind::GeneticAlgorithm {
            population_size,
            mutation,
            init_improvement,
            crossover: kind,
            sub_heuristic,
            parent_selection,
        } => {
            let sub = sub_heuristic.as_deref().map(recur).transpose()?;
            let crossover = crossover(kind.as_deref(), sub)?;
            let ga = GeneticAlgorithm::new(
                cond,
                *population_size,
                crossover,
                recur(mutation)?,
                *parent_selection,
            );
            Box::new(match init_improvement {
                Some(op) => ga.with_init_improvement(recur(op)?),
                None => ga,
            })
        }
        HeuristicKind::ComposedBreakoutLocalSearch {
            tabu_tenure,
            t,
            l0,
            p0,
            descent,
            random,
            directed,
        } => {
            let directed = directed
                .iter()
                .map(|(h, share)| Ok((recur(h)?, *share)))
                .collect::<Result<Vec<_>, String>>()?;
            let schedule = AdaptivePerturbation::new(*t, *l0, *p0, recur(random)?, directed);
            Box::new(BreakoutLocalSearch::new(
                cond,
                *tabu_tenure,
                recur(descent)?,
                schedule,
            ))
        }
        _ => return Ok(None),
    };
    Ok(Some(built))
}

/// Makes one crossover, or one around the heuristic that solves its sub-problem.
type MakeCrossover<P> = fn() -> Box<dyn Crossover<P>>;
type MakeSubProblemCrossover<P> = fn(Box<dyn Heuristic<P>>) -> Box<dyn Crossover<P>>;

/// Picks a GA crossover by name. The first of `named` is the default, and
/// `sub_problem` is set on the problems that can solve the sub-problem the two
/// parents disagree on.
fn pick_crossover<P: ProblemTrait>(
    problem: &str,
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<P>>>,
    named: &[(&str, MakeCrossover<P>)],
    sub_problem: Option<MakeSubProblemCrossover<P>>,
) -> Result<Box<dyn Crossover<P>>, String> {
    let kind = kind.unwrap_or(named[0].0);
    if let (Some(make), "SubProblem") = (sub_problem, kind) {
        let sub = sub.ok_or("crossover 'SubProblem' needs a 'sub_heuristic'")?;
        return Ok(make(sub));
    }
    if sub.is_some() {
        return Err("'sub_heuristic' only applies to crossover 'SubProblem'".to_string());
    }
    match named.iter().find(|(name, _)| *name == kind) {
        Some((_, make)) => Ok(make()),
        None => {
            let mut valid: Vec<_> = named.iter().map(|(name, _)| format!("'{name}'")).collect();
            if sub_problem.is_some() {
                valid.push("'SubProblem'".to_string());
            }
            Err(format!(
                "invalid crossover '{kind}' for {problem} (use {})",
                valid.join(" or ")
            ))
        }
    }
}

fn sub_problem_crossover<P: SubProblemExtractable + 'static>(
    sub_heuristic: Box<dyn Heuristic<P>>,
) -> Box<dyn Crossover<P>> {
    Box::new(SubProblemBasedCrossover { sub_heuristic })
}

/// Error text for a problem-specific heuristic applied to the wrong problem.
fn unsupported(kind: &HeuristicKind) -> String {
    match kind.only_for() {
        Some(problem) => format!("{} is only available for {problem}", kind.name()),
        None => format!("{} is not supported for this problem", kind.name()),
    }
}

/// Error for a spec that fell through a problem's neighbor match.
///
/// Problem-specific kinds carry no neighborhood, so reaching here means the
/// problem simply does not implement them; they get the "wrong problem" message
/// rather than a confusing complaint about an empty neighbor.
fn neighbor_error(spec: &HeuristicSpec, problem: &str, valid: &str) -> String {
    if spec.kind.only_for().is_some() {
        return unsupported(&spec.kind);
    }
    let given = &spec.neighbor;
    format!("invalid neighbor '{given}' for {problem} (use {valid})")
}

/// Builds a `GreedyRandomizedConstruction` on any problem that can be ruined and recreated, or
/// returns `None` when `spec` is something else.
fn grasp<P>(spec: &HeuristicSpec) -> Option<Box<dyn Heuristic<P>>>
where
    P: Ruinable + 'static,
    P::Solution: Evaluate,
{
    match spec.kind {
        HeuristicKind::GreedyRandomizedConstruction { alpha } => Some(Box::new(
            OptGreedyRandomizedConstruction::<P>::new(spec.stop.to_opt(), alpha),
        )),
        _ => None,
    }
}

/// Builds the ruin-and-recreate heuristic `spec` names on `problem`, an
/// `AdaptiveLargeNeighborhoodSearch` or an `IteratedGreedy` tuned as the spec says, or returns
/// `None` for any other spec.
///
/// `repair` gives the problem's own descent, run around the reinserted elements after each
/// recreation unless the spec turns it off. `flow_shop_temperature` gives the iterated greedy's
/// temperature from `temperature_factor` on the one problem that defines it; elsewhere the
/// spec's `temperature` is required.
fn ruin_and_recreate<P>(
    spec: &HeuristicSpec,
    problem: &str,
    repair: impl FnOnce() -> Option<Box<dyn LocalRepair<P>>>,
    flow_shop_temperature: Option<fn(&P, f64) -> f64>,
) -> Option<Result<Box<dyn Heuristic<P>>, String>>
where
    P: Ruinable + 'static,
    P::Solution: Evaluate,
{
    let stop = spec.stop.to_opt();
    Some(Ok(match spec.kind {
        HeuristicKind::AdaptiveLargeNeighborhoodSearch {
            removal_fraction,
            cooling_rate,
            tuning,
        } => {
            let (best, better, accept) = tuning.scores;
            let alns =
                AdaptiveLargeNeighborhoodSearch::<P>::new(stop, removal_fraction, cooling_rate)
                    .with_scoring(best, better, accept)
                    .with_adaptation(tuning.segment_length, tuning.reaction)
                    .with_max_removal(tuning.max_removal);
            match repair().filter(|_| tuning.local_repair) {
                Some(local) => Box::new(alns.with_local_repair(local)),
                None => Box::new(alns),
            }
        }
        HeuristicKind::IteratedGreedy {
            removal_count,
            temperature_factor,
            temperature,
            local_repair,
        } => {
            let greedy = match (temperature, flow_shop_temperature) {
                (Some(t), _) => IteratedGreedy::<P>::new(stop, removal_count, move |_: &P| t),
                (None, Some(of)) => IteratedGreedy::<P>::new(stop, removal_count, move |p: &P| {
                    of(p, temperature_factor)
                }),
                (None, None) => {
                    return Some(Err(format!(
                        "IteratedGreedy on {problem} needs a 'temperature', in the units of the \
                         objective"
                    )));
                }
            };
            match repair().filter(|_| local_repair) {
                Some(local) => Box::new(greedy.with_local_repair(local)),
                None => Box::new(greedy),
            }
        }
        _ => return None,
    }))
}

/// The flow shop's iterated greedy temperature, as `iterated_greedy_for_flow_shop` sets it: a
/// tenth of the mean processing time, times `factor`.
fn flow_shop_temperature(prob: &OptFlowShop, factor: f64) -> f64 {
    let operations = (prob.n_jobs * prob.n_machines).max(1) as f64;
    factor * prob.total_processing_time() as f64 / (operations * 10.0)
}

/// Builds a `BranchAndBound` pruning by the relaxation `spec` names: one written in Python on
/// any problem, or one of the problem's own, which `native` builds and answers `None` to when
/// it does not apply. Returns `None` when `spec` is something else.
fn branch_and_bound<P>(
    spec: &HeuristicSpec,
    problem: &str,
    maximize: bool,
    recur: BuildFn<'_, P>,
    native: impl FnOnce(&RelaxationSpec, Box<dyn Heuristic<P>>) -> Option<Box<dyn Heuristic<P>>>,
) -> Option<Result<Box<dyn Heuristic<P>>, String>>
where
    P: BranchSpace + 'static,
{
    let HeuristicKind::BranchAndBound {
        heuristic,
        relaxation,
    } = &spec.kind
    else {
        return None;
    };
    Some(recur(heuristic).and_then(|inner| match relaxation {
        RelaxationSpec::Python(f) => Ok(bnb::build(
            &spec.stop,
            inner,
            PyRelaxation::new(Python::attach(|py| f.clone_ref(py)), maximize),
        )),
        other => native(other, inner).ok_or_else(|| other.unsupported(problem)),
    }))
}

/// Builds a `BranchAndBound` on a binary problem, pruning by `BinaryRelaxation` unless the
/// spec names a Python relaxation. Returns `None` when `spec` is something else.
fn branch_and_bound_binary<P>(
    spec: &HeuristicSpec,
    problem: &str,
    maximize: bool,
    recur: BuildFn<'_, P>,
) -> Option<Result<Box<dyn Heuristic<P>>, String>>
where
    P: BranchSpace + FixVariables + 'static,
{
    branch_and_bound(
        spec,
        problem,
        maximize,
        recur,
        |relaxation, inner| match relaxation {
            RelaxationSpec::Default | RelaxationSpec::Binary => {
                Some(bnb::build(&spec.stop, inner, BinaryRelaxation))
            }
            _ => None,
        },
    )
}

fn crossover_max_cut(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptMaxCut>>>,
) -> Result<Box<dyn Crossover<OptMaxCut>>, String> {
    pick_crossover(
        "MaxCut",
        kind,
        sub,
        &[("Uniform", || Box::new(MaxCutUniformCrossover))],
        Some(sub_problem_crossover::<OptMaxCut>),
    )
}

fn build_max_cut(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptMaxCut>>, String> {
    if let Some(built) = build_nested(spec, &build_max_cut, &crossover_max_cut)? {
        return Ok(built);
    }
    let bnb =
        branch_and_bound(
            spec,
            "MaxCut",
            true,
            &build_max_cut,
            |relaxation, inner| match relaxation {
                RelaxationSpec::Default | RelaxationSpec::Binary => {
                    Some(bnb::build(&spec.stop, inner, BinaryRelaxation))
                }
                RelaxationSpec::Eigenvalue {
                    max_iterations,
                    lanczos_steps,
                    certify_limit,
                } => {
                    let eigen = EigenvalueRelaxation::new()
                        .with_max_iterations(*max_iterations)
                        .with_lanczos_steps(*lanczos_steps)
                        .with_certify_limit(*certify_limit);
                    Some(bnb::build(&spec.stop, inner, eigen))
                }
                _ => None,
            },
        );
    if let Some(result) = bnb {
        return result;
    }
    match &spec.kind {
        HeuristicKind::BreakoutLocalSearch {
            tabu_tenure,
            t,
            l0,
            p0,
            q,
        } => Ok(Box::new(bls_for_max_cut(
            spec.stop.to_opt(),
            *tabu_tenure,
            *t,
            *l0,
            *p0,
            *q,
        ))),
        _ => match spec.neighbor.as_str() {
            "Flip" => build_generic::<OptMaxCut, MaxCutFlipNeighbor>(spec),
            "Swap" => build_generic::<OptMaxCut, MaxCutSwapNeighbor>(spec),
            _ => Err(neighbor_error(spec, "MaxCut", "'Flip' or 'Swap'")),
        },
    }
}

fn crossover_qubo(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptQubo>>>,
) -> Result<Box<dyn Crossover<OptQubo>>, String> {
    pick_crossover(
        "Qubo",
        kind,
        sub,
        &[("Uniform", || Box::new(QuboUniformCrossover))],
        Some(sub_problem_crossover::<OptQubo>),
    )
}

fn build_qubo(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptQubo>>, String> {
    if let Some(built) = build_nested(spec, &build_qubo, &crossover_qubo)? {
        return Ok(built);
    }
    if let Some(result) = branch_and_bound_binary(spec, "Qubo", false, &build_qubo) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Flip" => build_generic::<OptQubo, QuboFlipNeighbor>(spec),
        "Swap" => build_generic::<OptQubo, QuboSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "Qubo", "'Flip' or 'Swap'")),
    }
}

fn crossover_sat(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptSat>>>,
) -> Result<Box<dyn Crossover<OptSat>>, String> {
    pick_crossover(
        "Sat",
        kind,
        sub,
        &[("Uniform", || Box::new(SatUniformCrossover))],
        Some(sub_problem_crossover::<OptSat>),
    )
}

fn build_sat(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptSat>>, String> {
    if let Some(built) = build_nested(spec, &build_sat, &crossover_sat)? {
        return Ok(built);
    }
    if let Some(result) = branch_and_bound_binary(spec, "Sat", true, &build_sat) {
        return result;
    }
    match &spec.kind {
        HeuristicKind::WalkSat { noise, adaptive } => Ok(Box::new(WalkSatForSat::new(
            spec.stop.to_opt(),
            *noise,
            *adaptive,
        ))),
        _ => match spec.neighbor.as_str() {
            "Flip" => build_generic::<OptSat, SatFlipNeighbor>(spec),
            "Swap" => build_generic::<OptSat, SatSwapNeighbor>(spec),
            _ => Err(neighbor_error(spec, "Sat", "'Flip' or 'Swap'")),
        },
    }
}

fn crossover_vertex_cover(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptVc>>>,
) -> Result<Box<dyn Crossover<OptVc>>, String> {
    pick_crossover(
        "VertexCover",
        kind,
        sub,
        &[("Uniform", || Box::new(VertexCoverUniformCrossover))],
        Some(sub_problem_crossover::<OptVc>),
    )
}

fn build_vertex_cover(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptVc>>, String> {
    if let Some(built) = build_nested(spec, &build_vertex_cover, &crossover_vertex_cover)? {
        return Ok(built);
    }
    if let Some(result) = branch_and_bound_binary(spec, "VertexCover", false, &build_vertex_cover) {
        return result;
    }
    match spec.neighbor.as_str() {
        "Flip" => build_generic::<OptVc, VertexCoverFlipNeighbor>(spec),
        "Swap" => build_generic::<OptVc, VertexCoverSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "VertexCover", "'Flip' or 'Swap'")),
    }
}

fn crossover_tsp(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptTsp>>>,
) -> Result<Box<dyn Crossover<OptTsp>>, String> {
    pick_crossover(
        "Tsp",
        kind,
        sub,
        &[("Order", || Box::new(TspOrderCrossover))],
        Some(sub_problem_crossover::<OptTsp>),
    )
}

fn build_tsp(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptTsp>>, String> {
    if let Some(built) = build_nested(spec, &build_tsp, &crossover_tsp)? {
        return Ok(built);
    }
    if let Some(grasp) = grasp(spec) {
        return Ok(grasp);
    }
    if let Some(built) = ruin_and_recreate(
        spec,
        "Tsp",
        || Some(Box::new(AnchoredTourDescent::new())),
        None,
    ) {
        return built;
    }
    match &spec.kind {
        HeuristicKind::LinKernighanHelsgaun {
            num_neighbors,
            max_depth,
        } => Ok(Box::new(LinKernighanHelsgaunForTsp::new(
            spec.stop.to_opt(),
            *num_neighbors,
            *max_depth,
        ))),
        _ => match spec.neighbor.as_str() {
            "TwoOpt" => build_generic::<OptTsp, TspTwoOptNeighbor>(spec),
            "Relocate" => build_generic::<OptTsp, TspRelocateNeighbor>(spec),
            _ => Err(neighbor_error(spec, "Tsp", "'TwoOpt' or 'Relocate'")),
        },
    }
}

fn crossover_job_shop(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptJobShop>>>,
) -> Result<Box<dyn Crossover<OptJobShop>>, String> {
    pick_crossover(
        "JobShopScheduling",
        kind,
        sub,
        &[("Ppx", || Box::new(JobShopPpxCrossover))],
        None,
    )
}

fn build_job_shop(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptJobShop>>, String> {
    if let Some(built) = build_nested(spec, &build_job_shop, &crossover_job_shop)? {
        return Ok(built);
    }
    match spec.neighbor.as_str() {
        "Swap" => build_generic::<OptJobShop, JobShopSwapNeighbor>(spec),
        "Relocate" => build_generic::<OptJobShop, JobShopRelocateNeighbor>(spec),
        _ => Err(neighbor_error(
            spec,
            "JobShopScheduling",
            "'Swap' or 'Relocate'",
        )),
    }
}

fn crossover_vrp(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptVrp>>>,
) -> Result<Box<dyn Crossover<OptVrp>>, String> {
    pick_crossover(
        "Vrp",
        kind,
        sub,
        &[("Order", || Box::new(VrpOrderCrossover))],
        None,
    )
}

fn build_vrp(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptVrp>>, String> {
    if let Some(built) = build_nested(spec, &build_vrp, &crossover_vrp)? {
        return Ok(built);
    }
    if let Some(grasp) = grasp(spec) {
        return Ok(grasp);
    }
    if let Some(built) = ruin_and_recreate(
        spec,
        "Vrp",
        || Some(Box::new(AnchoredRouteDescent::new())),
        None,
    ) {
        return built;
    }
    let cond = spec.stop.to_opt();
    match &spec.kind {
        HeuristicKind::HybridGeneticSearch {
            min_population_size,
            generation_size,
            granularity,
            target_feasible,
            restart_generations,
            tuning,
        } => {
            let (period, increase, decrease, tolerance) = tuning.penalty_adaptation;
            let (factor, nearest_neighbor_every) = tuning.initial_population;
            Ok(Box::new(
                HybridGeneticSearchForVrp::new(
                    cond,
                    *min_population_size,
                    *generation_size,
                    *granularity,
                    *target_feasible,
                    *restart_generations,
                )
                .with_elite(tuning.n_elite, tuning.n_closest)
                .with_penalty_adaptation(period, increase, decrease, tolerance)
                .with_repair_probability(tuning.repair_probability)
                .with_initial_population(factor, nearest_neighbor_every)
                .with_descent_passes(tuning.descent_passes),
            ))
        }
        _ => match spec.neighbor.as_str() {
            "Relocate" => build_generic::<OptVrp, VrpRelocateNeighbor>(spec),
            "Swap" => build_generic::<OptVrp, VrpSwapNeighbor>(spec),
            "TwoOpt" => build_generic::<OptVrp, VrpTwoOptNeighbor>(spec),
            _ => Err(neighbor_error(
                spec,
                "Vrp",
                "'Relocate', 'Swap' or 'TwoOpt'",
            )),
        },
    }
}

fn crossover_flow_shop(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptFlowShop>>>,
) -> Result<Box<dyn Crossover<OptFlowShop>>, String> {
    pick_crossover(
        "FlowShop",
        kind,
        sub,
        &[("Order", || Box::new(FlowShopOrderCrossover))],
        None,
    )
}

fn build_flow_shop(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptFlowShop>>, String> {
    if let Some(built) = build_nested(spec, &build_flow_shop, &crossover_flow_shop)? {
        return Ok(built);
    }
    if let Some(grasp) = grasp(spec) {
        return Ok(grasp);
    }
    if let Some(built) = ruin_and_recreate(
        spec,
        "FlowShop",
        || Some(Box::new(FlowShopInsertionDescent::new())),
        Some(flow_shop_temperature),
    ) {
        return built;
    }
    match spec.neighbor.as_str() {
        "Relocate" => build_generic::<OptFlowShop, FlowShopInsertNeighbor>(spec),
        "Swap" => build_generic::<OptFlowShop, FlowShopSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "FlowShop", "'Relocate' or 'Swap'")),
    }
}

fn crossover_qap(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptQap>>>,
) -> Result<Box<dyn Crossover<OptQap>>, String> {
    pick_crossover(
        "Qap",
        kind,
        sub,
        &[("Order", || Box::new(QapOrderCrossover))],
        None,
    )
}

fn build_qap(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptQap>>, String> {
    if let Some(built) = build_nested(spec, &build_qap, &crossover_qap)? {
        return Ok(built);
    }
    match spec.neighbor.as_str() {
        "Swap" => build_generic::<OptQap, QapSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "Qap", "'Swap'")),
    }
}

fn crossover_bin_packing(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptBinPacking>>>,
) -> Result<Box<dyn Crossover<OptBinPacking>>, String> {
    pick_crossover(
        "BinPacking",
        kind,
        sub,
        &[("Group", || Box::new(BinPackingGroupCrossover))],
        None,
    )
}

fn build_bin_packing(spec: &HeuristicSpec) -> Result<Box<dyn Heuristic<OptBinPacking>>, String> {
    if let Some(built) = build_nested(spec, &build_bin_packing, &crossover_bin_packing)? {
        return Ok(built);
    }
    if let Some(grasp) = grasp(spec) {
        return Ok(grasp);
    }
    if let Some(built) = ruin_and_recreate(spec, "BinPacking", || None, None) {
        return built;
    }
    match spec.neighbor.as_str() {
        "Relocate" => build_generic::<OptBinPacking, BinPackingRelocateNeighbor>(spec),
        "Swap" => build_generic::<OptBinPacking, BinPackingSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "BinPacking", "'Relocate' or 'Swap'")),
    }
}

fn crossover_formula(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<FormulaProblem>>>,
) -> Result<Box<dyn Crossover<FormulaProblem>>, String> {
    pick_crossover(
        "Formula",
        kind,
        sub,
        &[("Uniform", || Box::new(IntCrossover))],
        Some(sub_problem_crossover::<FormulaProblem>),
    )
}

/// Builds a heuristic for a `Formula`. `maximize` is its direction, which a relaxation written
/// in Python states its bound in and the core keeps private, and `permutation` whether its
/// variables are a permutation, which rules out the moves and the crossover that would leave
/// one.
fn build_formula(
    maximize: bool,
    permutation: bool,
    spec: &HeuristicSpec,
) -> Result<Box<dyn Heuristic<FormulaProblem>>, String> {
    let recur = |s: &HeuristicSpec| build_formula(maximize, permutation, s);
    let crossover = |kind: Option<&str>, sub| {
        // optopus asserts a sub-problem is not cut out of a permutation.
        if permutation && kind == Some("SubProblem") {
            return Err(
                "the 'SubProblem' crossover does not apply to a permutation Formula; use \
                 'Uniform', which is an order crossover there"
                    .to_string(),
            );
        }
        crossover_formula(kind, sub)
    };
    if let Some(built) = build_nested(spec, &recur, &crossover)? {
        return Ok(built);
    }
    let bnb =
        branch_and_bound(
            spec,
            "Formula",
            maximize,
            &recur,
            |relaxation, inner| match relaxation {
                RelaxationSpec::Default | RelaxationSpec::Interval => {
                    Some(bnb::build(&spec.stop, inner, IntervalRelaxation))
                }
                _ => None,
            },
        );
    if let Some(result) = bnb {
        return result;
    }
    match spec.neighbor.as_str() {
        // On a binary variable an integer change *is* a flip, which upstream says in as many
        // words, so "Flip" stays accepted as the name it had before the integer layer.
        "Change" | "Flip" if permutation => Err(format!(
            "invalid neighbor '{}' for a permutation Formula, which no single change keeps a \
             permutation (use 'Swap' or 'Reverse')",
            spec.neighbor
        )),
        "Change" | "Flip" => build_generic::<FormulaProblem, IntChangeNeighbor>(spec),
        "Swap" => build_generic::<FormulaProblem, IntSwapNeighbor>(spec),
        "Reverse" => build_generic::<FormulaProblem, IntReverseNeighbor>(spec),
        _ => Err(neighbor_error(
            spec,
            "Formula",
            "'Change' (or its alias 'Flip'), 'Swap' or 'Reverse'",
        )),
    }
}

fn crossover_graph_coloring(
    kind: Option<&str>,
    sub: Option<Box<dyn Heuristic<OptGraphColoring>>>,
) -> Result<Box<dyn Crossover<OptGraphColoring>>, String> {
    pick_crossover(
        "GraphColoring",
        kind,
        sub,
        &[("Uniform", || Box::new(GraphColoringUniformCrossover))],
        None,
    )
}

fn build_graph_coloring(
    spec: &HeuristicSpec,
) -> Result<Box<dyn Heuristic<OptGraphColoring>>, String> {
    if let Some(built) = build_nested(spec, &build_graph_coloring, &crossover_graph_coloring)? {
        return Ok(built);
    }
    match spec.neighbor.as_str() {
        "Recolor" => build_generic::<OptGraphColoring, GraphColoringRecolorNeighbor>(spec),
        "Swap" => build_generic::<OptGraphColoring, GraphColoringSwapNeighbor>(spec),
        _ => Err(neighbor_error(spec, "GraphColoring", "'Recolor' or 'Swap'")),
    }
}

/// Builds a heuristic for a Python problem, with the neighbor name resolved
/// against the problem's own neighborhoods. The result is always [`Guarded`],
/// so a Python exception stops nested heuristics too.
fn build_python(
    prob: &PyProblem,
    spec: &HeuristicSpec,
) -> Result<Box<dyn Heuristic<PyProblem>>, String> {
    let recur = |s: &HeuristicSpec| build_python(prob, s);
    let crossover = |kind: Option<&str>, sub: Option<Box<dyn Heuristic<PyProblem>>>| match kind {
        Some("SubProblem") => {
            prob.check_sub_problem()?;
            let sub = sub.ok_or("crossover 'SubProblem' needs a 'sub_heuristic'")?;
            Ok(sub_problem_crossover(sub))
        }
        Some(other) => Err(format!(
            "invalid crossover '{other}' for a Python problem (use 'SubProblem', or leave it \
             unset for the problem's 'crossover' method)"
        )),
        None if sub.is_some() => {
            Err("'sub_heuristic' only applies to crossover 'SubProblem'".to_string())
        }
        None => Ok(Box::new(PyCrossover::new(prob)?) as Box<dyn Crossover<PyProblem>>),
    };
    let inner: Box<dyn Heuristic<PyProblem>> = match build_nested(spec, &recur, &crossover)? {
        Some(built) => built,
        None => match spec.kind {
            HeuristicKind::AdaptiveLargeNeighborhoodSearch { .. }
            | HeuristicKind::IteratedGreedy { .. } => {
                prob.check_ruinable(spec.kind.name())?;
                let repair =
                    || PyRepair::new(prob).map(|r| Box::new(r) as Box<dyn LocalRepair<PyProblem>>);
                ruin_and_recreate(spec, "a Python problem", repair, None)
                    .expect("the spec is one ruin_and_recreate builds")?
            }
            HeuristicKind::BranchAndBound { ref relaxation, .. } => {
                prob.check_branchable()?;
                if !matches!(relaxation, RelaxationSpec::Python(_)) {
                    return Err(
                        "BranchAndBound on a Python problem needs a relaxation written \
                                in Python, a callable taking the node's ranges"
                            .to_string(),
                    );
                }
                branch_and_bound(
                    spec,
                    "a Python problem",
                    !prob.minimize(),
                    &recur,
                    |_, _| None,
                )
                .expect("the spec is a BranchAndBound")?
            }
            HeuristicKind::GreedyRandomizedConstruction { alpha } => {
                prob.check_ruinable(spec.kind.name())?;
                Box::new(OptGreedyRandomizedConstruction::<PyProblem>::new(
                    spec.stop.to_opt(),
                    alpha,
                ))
            }
            _ => {
                if spec.kind.only_for().is_some() {
                    return Err(unsupported(&spec.kind));
                }
                let k = prob.neighborhood_index(&spec.neighbor)?;
                if matches!(spec.kind, HeuristicKind::TabuSearch { .. }) {
                    prob.enable_tabu(k)?;
                }
                macro_rules! by_index {
                ($($k:literal),+) => {
                    match k {
                        $($k => build_generic::<PyProblem, PyMove<$k>>(spec)?,)+
                        _ => unreachable!("PyProblem caps its neighborhoods at MAX_NEIGHBORHOODS"),
                    }
                };
            }
                const _: () = assert!(MAX_NEIGHBORHOODS == 8);
                by_index!(0, 1, 2, 3, 4, 5, 6, 7)
            }
        },
    };
    Ok(Box::new(Guarded { inner }))
}

/// Derives a deterministic per-run seed from a master seed. Run index 0 maps to
/// the master itself, so a single seeded run reproduces with `new_with_seed`.
fn derive_seed(master: u64, run_index: usize) -> u64 {
    master.wrapping_add((run_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Runs the heuristic `spec` describes `runs` times on `instance` and aggregates a report.
#[allow(clippy::too_many_arguments)]
fn run_all<P>(
    instance: &P,
    spec: &HeuristicSpec,
    build: impl Fn(&HeuristicSpec) -> Result<Box<dyn Heuristic<P>>, String>,
    minimize: bool,
    runs: usize,
    seed: Option<u64>,
    on_improvement: Option<&Arc<Py<PyAny>>>,
    initial: Option<&P::Solution>,
    start_from_given: impl Fn(&mut SmallRng),
    decode: impl Fn(&P::Solution, Python<'_>) -> Py<PyAny>,
) -> PyResult<RunReport>
where
    P: ProblemTrait + 'static,
    P::Solution: Reported,
{
    let mut results = Vec::with_capacity(runs);
    for run_index in 0..runs {
        let run_seed = seed.map(|m| derive_seed(m, run_index));
        let mut heuristic = build(spec).map_err(PyValueError::new_err)?;

        let start = Instant::now();
        let mut state = match (initial, run_seed) {
            (None, Some(s)) => SearchState::new_with_seed(instance, s),
            (None, None) => SearchState::new(instance),
            (Some(sol), Some(s)) => SearchState::with_solution_and_seed(instance, sol.clone(), s),
            (Some(sol), None) => SearchState::with_solution(instance, sol.clone()),
        };
        if initial.is_some() {
            start_from_given(&mut state.rng);
        }
        state.set_objective_probe(progress::probe::<P::Solution>);
        let initial_objective = state.initial_solution.reported();
        let watch = on_improvement.map(|callback| {
            progress::Watch::start(Arc::clone(callback), run_index, start, minimize)
        });
        let ran = heuristic.run(&mut state);
        if let Some(watch) = watch
            && !watch.finish()
        {
            return Err(PyRuntimeError::new_err(
                "the on_improvement callback raised an exception",
            ));
        }
        // A heuristic stops with an error when the search cannot go on, an empty neighborhood
        // for one. The run's numbers would read as a finished search, so no report is made.
        ran.map_err(error::run_error)?;
        let total_time = start.elapsed();
        // Only a top-level search proves anything about the run's best solution; a nested one
        // searched a part of the run.
        let outcome = bnb::take_outcome()
            .filter(|_| matches!(spec.kind, HeuristicKind::BranchAndBound { .. }));
        let details = details::take()
            .filter(|_| {
                matches!(
                    spec.kind,
                    HeuristicKind::ParallelTempering { .. }
                        | HeuristicKind::ReinforcementLearningSearch { .. }
                )
            })
            .unwrap_or_default();

        let best_objective = state.best_solution.reported();
        let raw_diff = best_objective - initial_objective;
        let improvement = if minimize { -raw_diff } else { raw_diff };

        let solution = Python::attach(|py| decode(&state.best_solution, py));

        results.push(RunResult {
            best_objective,
            solution,
            best_iteration: state.best_iteration,
            iterations: state.iteration,
            time_to_best_secs: state
                .best_time
                .saturating_duration_since(start)
                .as_secs_f64(),
            total_time_secs: total_time.as_secs_f64(),
            initial_objective,
            improvement,
            n_accepted: state.n_accepted,
            n_rejected: state.n_rejected,
            n_best_updates: state.n_best_updates,
            seed: run_seed,
            proven_optimal: outcome.as_ref().map(|o| o.proven_optimal),
            dual_bound: outcome.map(|o| bnb::dual_on_report_scale(o.dual_bound, minimize)),
            betas: details.betas,
            exchange_acceptance: details.exchange_acceptance,
            policy_weights: details.policy_weights,
            trajectory: monotone_trajectory(&state.trajectory, start, minimize),
        });
    }

    Ok(RunReport::from_runs(results, minimize))
}

/// The recorded best-solution improvements as `(elapsed_secs, iteration, objective)`, keeping
/// only the points that improve on the one before. A `ClearBest` / `StartBest` sub-run records
/// its own best, which can be worse than the run's best so far.
fn monotone_trajectory(
    points: &[TrajectoryPoint],
    start: Instant,
    minimize: bool,
) -> Vec<(f64, u64, f64)> {
    let mut out: Vec<(f64, u64, f64)> = Vec::with_capacity(points.len());
    for p in points {
        let improves = out.last().is_none_or(|&(_, _, incumbent)| {
            if minimize {
                p.objective < incumbent
            } else {
                p.objective > incumbent
            }
        });
        if improves {
            let elapsed = p.instant.saturating_duration_since(start).as_secs_f64();
            out.push((elapsed, p.iteration, p.objective));
        }
    }
    out
}

fn decode_list<'py, T: Copy + IntoPyObject<'py>>(x: &[T], py: Python<'py>) -> Py<PyAny> {
    PyList::new(py, x.iter().copied()).unwrap().into()
}

/// Decodes a VRP route partition into a `list[list[int]]`.
fn decode_routes(routes: &[Vec<usize>], py: Python<'_>) -> Py<PyAny> {
    let rows: Vec<Py<PyAny>> = routes.iter().map(|r| decode_list(r, py)).collect();
    PyList::new(py, rows).unwrap().into()
}

/// Rebuilds a `HeuristicSpec` from any heuristic instance handed in from Python.
///
/// Used by `VariableNeighborhoodSearch`, whose search and shake steps are
/// ordinary Python heuristic objects.
pub fn spec_from_py(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<HeuristicSpec> {
    macro_rules! try_heuristics {
        ($($ty:ident),+ $(,)?) => {
            $(
                if let Ok(h) = obj.extract::<PyRef<'_, py_heuristic::$ty>>() {
                    return h.to_spec(py);
                }
            )+
        };
    }

    try_heuristics!(
        LocalSearch,
        SimulatedAnnealing,
        TabuSearch,
        LateAcceptanceHillClimbing,
        RandomWalk,
        BangBangSimulatedAnnealing,
        BeamSearch,
        VariableNeighborhoodSearch,
        Sequential,
        Iterated,
        Restart,
        GeneticAlgorithm,
        ReinforcementLearningSearch,
        WalkSat,
        PopulationAnnealing,
        ParallelTempering,
        BreakoutLocalSearch,
        LinKernighanHelsgaun,
        AdaptiveLargeNeighborhoodSearch,
        HybridGeneticSearch,
        IteratedGreedy,
        GreedyRandomizedConstruction,
        BranchAndBound,
    );

    Err(PyTypeError::new_err(
        "expected a heuristic instance (e.g. LocalSearch, SimulatedAnnealing, RandomWalk)",
    ))
}

/// Entry point: dispatch on the concrete Python problem type and run.
pub fn solve(
    problem: &Bound<'_, PyAny>,
    spec: &HeuristicSpec,
    runs: usize,
    seed: Option<u64>,
    on_improvement: Option<Py<PyAny>>,
    initial_solution: Option<Py<PyAny>>,
) -> PyResult<RunReport> {
    if runs == 0 {
        return Err(PyValueError::new_err("'runs' must be at least 1"));
    }
    if let Some(callback) = &on_improvement
        && !callback.bind(problem.py()).is_callable()
    {
        return Err(PyTypeError::new_err("'on_improvement' must be callable"));
    }
    let on_improvement = on_improvement.map(Arc::new);
    let on_improvement = on_improvement.as_ref();
    progress::take_error();
    // Drop what an earlier call left behind, should it have stopped before taking it.
    bnb::take_outcome();
    bnb::take_error();
    details::take();

    let py = problem.py();
    let finish = |report: PyResult<RunReport>| {
        if let Some(err) = bnb::take_error() {
            return Err(err);
        }
        if let Some(err) = progress::take_error() {
            return Err(err);
        }
        report
    };

    // A built-in problem never calls into Python during a run (a relaxation written in Python
    // attaches on its own), so the run releases the GIL for other Python threads.
    macro_rules! built_in {
        ($($ty:ident: $build:expr, minimize = $minimize:literal, $encode:path, $decode:expr;)+) => {
            $(
                if let Ok(p) = problem.extract::<PyRef<'_, $ty>>() {
                    let inner = &p.inner;
                    let initial = initial_solution
                        .as_ref()
                        .map(|value| $encode(inner, value.bind(py), "initial_solution"))
                        .transpose()?;
                    return finish(py.detach(|| {
                        run_all(
                            inner,
                            spec,
                            $build,
                            $minimize,
                            runs,
                            seed,
                            on_improvement,
                            initial.as_ref(),
                            |_| {},
                            $decode,
                        )
                    }));
                }
            )+
        };
    }
    built_in! {
        MaxCut: build_max_cut, minimize = false, initial::max_cut,
            |s, py| decode_list(&s.x, py);
        Qubo: build_qubo, minimize = true, initial::qubo,
            |s, py| decode_list(&s.x, py);
        Sat: build_sat, minimize = false, initial::sat,
            |s, py| decode_list(&s.x, py);
        VertexCover: build_vertex_cover, minimize = true, initial::vertex_cover,
            |s, py| decode_list(&s.x, py);
        Tsp: build_tsp, minimize = true, initial::tsp,
            |s, py| decode_list(&s.tour, py);
        JobShopScheduling: build_job_shop, minimize = true, initial::job_shop,
            |s, py| decode_list(&s.operations, py);
        Vrp: build_vrp, minimize = true, initial::vrp,
            |s, py| decode_routes(&s.routes, py);
        GraphColoring: build_graph_coloring, minimize = true, initial::graph_coloring,
            |s, py| decode_list(&s.colors, py);
        FlowShop: build_flow_shop, minimize = true, initial::flow_shop,
            |s, py| decode_list(&s.sequence, py);
        Qap: build_qap, minimize = true, initial::qap,
            |s, py| decode_list(&s.assignment, py);
        BinPacking: build_bin_packing, minimize = true, initial::bin_packing,
            |s, py| decode_list(&s.bin_of, py);
    }

    if let Ok(f) = problem.extract::<PyRef<'_, Formula>>() {
        let (inner, maximize) = (&f.inner, f.maximize);
        let permutation = inner.variables().is_permutation();
        let initial = initial_solution
            .as_ref()
            .map(|value| initial::formula(inner, value.bind(py), "initial_solution"))
            .transpose()?;
        // A Formula reports a higher-is-better score whichever way it optimizes (its `Reported`
        // impl), so the runner tracks it as a maximization.
        return finish(py.detach(|| {
            run_all(
                inner,
                spec,
                |s| build_formula(maximize, permutation, s),
                false,
                runs,
                seed,
                on_improvement,
                initial.as_ref(),
                |_| {},
                |s, py| decode_list(s.values(), py),
            )
        }));
    }

    if !problem.hasattr("neighborhoods")? {
        return Err(PyTypeError::new_err(
            "problem must be a MaxCut, Qubo, Sat, VertexCover, Tsp, JobShopScheduling, Vrp, \
             FlowShop, Qap, BinPacking, GraphColoring, or Formula instance, or a Python problem \
             with 'neighborhoods'",
        ));
    }
    let prob = PyProblem::from_py(problem)?;
    let initial = initial_solution
        .as_ref()
        .map(|value| prob.given_solution(value.bind(py)))
        .transpose()?;
    let report = run_all(
        &prob,
        spec,
        |s| build_python(&prob, s),
        prob.minimize(),
        runs,
        seed,
        on_improvement,
        initial.as_ref(),
        |rng| prob.start_from_given(rng),
        |s, py| s.value.clone_ref(py),
    );
    if let Some(err) = prob.take_error() {
        return Err(err);
    }
    finish(report)
}

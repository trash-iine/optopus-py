//! What a run of `ParallelTempering` or `ReinforcementLearningSearch` learned about itself:
//! the temperature ladder and how often its replicas swapped, or the policy weights it trained.
//!
//! `run_all` only sees a `Box<dyn Heuristic<P>>`, as with `BranchAndBound`'s outcome, so
//! [`Recording`] reads them off the heuristic when its run returns and leaves them in a
//! thread-local the runner takes after the run. A run stays on the thread that started it.

use std::cell::RefCell;

use optopus::error::OptError;
use optopus::prelude::{Heuristic, ProblemTrait, SearchState, StopCondition};

/// What a run read off its heuristic, each `None` where the heuristic has no such thing.
#[derive(Default)]
pub struct Details {
    pub betas: Option<Vec<f64>>,
    pub exchange_acceptance: Option<f64>,
    pub policy_weights: Option<Vec<f64>>,
}

thread_local! {
    static DETAILS: RefCell<Option<Details>> = const { RefCell::new(None) };
}

/// Takes the details the last recorded run left, leaving none.
pub fn take() -> Option<Details> {
    DETAILS.with(|d| d.borrow_mut().take())
}

/// A heuristic that records what `read` gives about it each time its run returns.
pub struct Recording<H> {
    inner: H,
    read: fn(&H) -> Details,
}

/// `inner`, recording what `read` gives about it after each run.
pub fn recording<P, H>(inner: H, read: fn(&H) -> Details) -> Box<dyn Heuristic<P>>
where
    P: ProblemTrait,
    H: Heuristic<P> + 'static,
{
    Box::new(Recording { inner, read })
}

impl<P: ProblemTrait, H: Heuristic<P>> Heuristic<P> for Recording<H> {
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn stop_condition(&self) -> &StopCondition {
        self.inner.stop_condition()
    }

    fn is_done<'a>(&self, state: &SearchState<'a, P>) -> bool {
        self.inner.is_done(state)
    }

    fn run_once<'a>(&mut self, state: &mut SearchState<'a, P>) -> Result<(), OptError> {
        self.inner.run_once(state)
    }

    fn run<'a>(&mut self, state: &mut SearchState<'a, P>) -> Result<(), OptError> {
        let result = self.inner.run(state);
        let details = (self.read)(&self.inner);
        DETAILS.with(|d| *d.borrow_mut() = Some(details));
        result
    }
}

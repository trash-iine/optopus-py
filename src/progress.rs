//! `on_improvement`: calling back into Python while a run improves its best solution.
//!
//! The core calls the objective probe every time a run's best solution improves, possibly from
//! a sub-run or a worker thread. The probe is a plain `fn`, so [`probe`] is instantiated per
//! solution type and reaches the callback through a process-wide [`Sink`].
//!
//! The first improvement of a run is sent at once; later ones are held, and a ticker thread
//! sends the latest held one every [`INTERVAL`], so a fast search neither calls into Python
//! thousands of times nor leaves the caller looking at a stale best. Python is never called
//! while the sink's lock is held: the thread holding the GIL may be waiting for that lock.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use optopus::prelude::Evaluate;
use optopus::problem::{
    BinPackingSolution, FlowShopSolution, FormulaSolution, GraphColoringSolution, JobShopSolution,
    MaxCutSolution, QapSolution, QuboSolution, SatSolution, TspSolution, VertexCoverSolution,
    VrpSolution,
};
use pyo3::prelude::*;

use crate::python_problem::PySolution;

const INTERVAL: Duration = Duration::from_millis(100);

/// The objective a run reports for a solution, on the scale of `RunResult.best_objective`.
pub trait Reported {
    fn reported(&self) -> f64;
}

macro_rules! reported {
    ($($ty:ty => |$s:ident| $value:expr;)+) => {
        $(impl Reported for $ty {
            fn reported(&self) -> f64 {
                let $s = self;
                $value
            }
        })+
    };
}

reported! {
    MaxCutSolution => |s| s.objective as f64;
    QuboSolution => |s| s.objective as f64;
    SatSolution => |s| s.n_satisfied as f64;
    VertexCoverSolution => |s| s.objective as f64;
    TspSolution => |s| s.objective;
    JobShopSolution => |s| s.objective as f64;
    VrpSolution => |s| s.objective;
    GraphColoringSolution => |s| s.objective as f64;
    FlowShopSolution => |s| f64::from(s.objective);
    QapSolution => |s| s.objective as f64;
    BinPackingSolution => |s| s.objective;
    // `minimized()` has the direction applied; negating it gives the higher-is-better score a
    // Formula reports. Subtracting rather than negating keeps a zero objective reported as 0.0.
    FormulaSolution => |s| 0.0 - s.evaluate().minimized();
    PySolution => |s| s.objective;
}

struct Sink {
    // An Arc, so the callback is taken out of the lock without the GIL.
    callback: Arc<Py<PyAny>>,
    run: usize,
    start: Instant,
    minimize: bool,
    best: Option<f64>,
    held: Option<(f64, f64)>,
    error: Option<PyErr>,
}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);
static RAISED: Mutex<Option<PyErr>> = Mutex::new(None);
static WATCHING: AtomicBool = AtomicBool::new(false);

fn sink() -> MutexGuard<'static, Option<Sink>> {
    // A poisoned lock only means a callback panicked; the sink itself is still consistent.
    SINK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The objective probe a run installs: reports `solution` and passes the improvement on.
pub fn probe<S: Reported>(solution: &S) -> f64 {
    let objective = solution.reported();
    if WATCHING.load(Ordering::Relaxed) {
        improved(objective);
    }
    objective
}

fn improved(objective: f64) {
    let send = {
        let mut guard = sink();
        let Some(sink) = guard.as_mut() else { return };
        // A sub-run reports its own best, which can be worse than the run's.
        let better = sink.best.is_none_or(|best| {
            if sink.minimize {
                objective < best
            } else {
                objective > best
            }
        });
        if !better {
            return;
        }
        let first = sink.best.is_none();
        sink.best = Some(objective);
        let point = (sink.start.elapsed().as_secs_f64(), objective);
        if first {
            Some(point)
        } else {
            sink.held = Some(point);
            None
        }
    };
    if let Some(point) = send {
        call(point);
    }
}

/// Calls the callback with `point`, outside the lock. The first exception is kept, and no
/// further calls are made once one was raised.
fn call((elapsed, objective): (f64, f64)) {
    let Some((callback, run)) = sink()
        .as_ref()
        .filter(|s| s.error.is_none())
        .map(|s| (Arc::clone(&s.callback), s.run))
    else {
        return;
    };
    let result = Python::attach(|py| callback.call1(py, (run, elapsed, objective)).map(drop));
    if let Err(err) = result
        && let Some(sink) = sink().as_mut()
    {
        sink.error.get_or_insert(err);
    }
}

/// Watches one run: from [`Watch::start`] until [`Watch::finish`], improvements reach
/// `callback`.
pub struct Watch {
    stop: mpsc::Sender<()>,
    ticker: JoinHandle<()>,
}

impl Watch {
    pub fn start(callback: Arc<Py<PyAny>>, run: usize, start: Instant, minimize: bool) -> Self {
        *sink() = Some(Sink {
            callback,
            run,
            start,
            minimize,
            best: None,
            held: None,
            error: None,
        });
        WATCHING.store(true, Ordering::SeqCst);
        let (stop, stopped) = mpsc::channel::<()>();
        let ticker = std::thread::spawn(move || {
            while let Err(mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(INTERVAL) {
                let held = sink().as_mut().and_then(|s| s.held.take());
                if let Some(point) = held {
                    call(point);
                }
            }
        });
        Watch { stop, ticker }
    }

    /// Stops watching, sending the improvement still held. Returns `false`, keeping the
    /// exception for [`take_error`], when the callback raised one.
    pub fn finish(self) -> bool {
        WATCHING.store(false, Ordering::SeqCst);
        let _ = self.stop.send(());
        // The ticker may be waiting for the GIL; release it while waiting for the ticker.
        Python::attach(|py| py.detach(|| self.ticker.join()))
            .expect("the progress ticker does not panic");
        // The run's last improvement, if it came within INTERVAL of the end.
        let held = sink().as_mut().and_then(|s| s.held.take());
        if let Some(point) = held {
            call(point);
        }
        let error = sink().take().and_then(|s| s.error);
        let raised = error.is_some();
        if raised {
            *RAISED.lock().unwrap_or_else(|p| p.into_inner()) = error;
        }
        !raised
    }
}

/// The exception an `on_improvement` callback raised during the last call, if any.
pub fn take_error() -> Option<PyErr> {
    RAISED.lock().unwrap_or_else(|p| p.into_inner()).take()
}

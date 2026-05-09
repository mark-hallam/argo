//! Decorator that records per-`Executor::dispatch` wall-clock
//! latency into the process-global
//! [`argo_store::latency_tracker::global`] tracker (Stream
//! `claude-latency-percentiles-tracking`).
//!
//! Lives in a sibling file (rather than inside
//! `worker_actions_executor.rs`) so the parent file stays under
//! its 500-LOC ceiling. `resolve_executor` wraps the inner
//! `Box<dyn Executor>` in a `TimedExecutor` so every production
//! dispatch path is timed without each implementor having to opt
//! in. Recording happens on both `Ok` and `Err`: latency-to-
//! failure is just as operationally interesting as latency-to-
//! success.

use std::path::Path;
use std::time::Instant;

use argo_types::ArgoResult;

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind, Preflight};

pub(crate) struct TimedExecutor {
    inner: Box<dyn Executor>,
}

impl TimedExecutor {
    pub(crate) fn new(inner: Box<dyn Executor>) -> Self {
        Self { inner }
    }
}

impl Executor for TimedExecutor {
    fn kind(&self) -> ExecutorKind {
        self.inner.kind()
    }

    fn preflight(&self, action: &ActionRecord) -> ArgoResult<Preflight> {
        self.inner.preflight(action)
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let start = Instant::now();
        let result = self.inner.dispatch(action, cwd);
        argo_store::latency_tracker::global()
            .record(self.inner.kind().as_str(), start.elapsed());
        result
    }
}

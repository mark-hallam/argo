//! `argo-eval` — eval harness runtime.
//!
//! Powers `argo forge bench` (§26 acceptance) and feeds the example
//! batches that `argo forge optimize` consumes. The crate is the
//! shared runtime; the on-disk task layout lives under
//! `eval/tasks/<category>/*.jsonl` and is described in §3 of the
//! build plan.
//!
//! Pieces:
//!
//! - **[`EvalTask`]** — `(name, input, expected, metric)`; one per
//!   JSONL line.
//! - **[`EvalMetric`]** — `ExactMatch`, `SubstringContains`,
//!   `JaccardTokens`. Continuous metrics produce a `[0.0, 1.0]`
//!   score; the runner derives a binary `passed` from `>= 0.5`.
//! - **[`load_jsonl`]** — read a `*.jsonl` task file.
//! - **[`EvalSubject`]** — async trait for what's under test
//!   (typically wraps an `Agent` or a routing classifier).
//! - **[`run_eval`]** — drives `subject` over `tasks`, returning
//!   per-task `EvalRecord`s + an aggregate `EvalSummary`.
//!
//! ```
//! use argo_eval::{run_eval, EvalMetric, EvalSubject, EvalTask};
//! use argo_types::ArgoResult;
//! struct Echo;
//! #[async_trait::async_trait]
//! impl EvalSubject for Echo {
//!     async fn run(&self, task: &EvalTask) -> ArgoResult<String> {
//!         Ok(task.expected.clone())
//!     }
//! }
//! # async fn demo() {
//! let tasks = vec![EvalTask {
//!     name: "demo".into(),
//!     input: "in".into(),
//!     expected: "out".into(),
//!     metric: EvalMetric::ExactMatch,
//! }];
//! let (recs, summary) = run_eval(&Echo, &tasks).await;
//! assert_eq!(recs.len(), 1);
//! assert!((summary.mean_score - 1.0).abs() < 1e-9);
//! # }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod runner;
mod compare;
pub mod honesty;
mod streaming;
mod task;

#[cfg(test)]
mod tests;

pub use compare::{
    compare_runs, ComparisonGroup, ComparisonItem, ComparisonReport, EvalRunFixture,
    EvalRunReport, GroupComparison,
};
pub use honesty::{
    run_honesty_test, AuditSampler, HonestyConfig, HonestyResult, SampleVerdict, SkillSample,
};
pub use runner::{run_eval, EvalRecord, EvalSubject, EvalSummary};
pub use streaming::{
    format_simple_line, format_summary_line, format_verbose_line, StreamMode,
};
pub use task::{load_jsonl, EvalMetric, EvalTask};

//! `argo-forge` — Forge skill-optimization orchestration (§26).
//!
//! Phase-7.5 ship: the orchestration surface (data types,
//! [`Forge`] trait, [`finalize_run`] event-folder, persistence
//! helpers). The real DSPy/GEPA worker runs in a Python subprocess
//! per §26.4 — exposed via [`StubForge`] until the
//! `argo-plugin-py:dspy-gepa` worker plugin lands.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use argo_forge::{Forge, ForgeOptions, StubForge};
//! let f = StubForge::new();
//! let opts = ForgeOptions::new("alpha", "0.1");
//! // Stub returns a clear "configure the Python worker" error.
//! assert!(f.optimize(opts).await.is_err());
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod config;
pub mod cost;
pub mod feedback_eval;
mod forge;
mod persist;
mod run;

#[cfg(test)]
mod tests;

pub use config::{FEEDBACK_CASE_WEIGHT, FEEDBACK_WINDOW_DAYS};
pub use cost::{
    accumulate_e2b_seconds, accumulate_llm_call, pricing_for_model, ForgeCost, PricingTable,
};
pub use feedback_eval::{
    ingest_operator_feedback, weighted_loss_score, CaseKind, ExpectedOutcome, FeedbackEvalCase,
    ScoredCase,
};
pub use forge::{finalize_run, Forge, ForgeEvent, StubForge};
pub use persist::{
    count_forge_runs, load_forge_cost, load_forge_status, persist_forge_run, query_forge_costs,
    CostQueryFilter, SkillCostRow,
};
pub use run::{ForgeOptions, ForgeRun, ForgeStatus};

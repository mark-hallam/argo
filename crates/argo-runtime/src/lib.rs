//! Stream `claude-runtime-planner-trait` — runtime worker
//! primitives. v1 ships the [`Planner`] trait + two impls
//! ([`TemplatePlanner`], [`LlmPlanner`]) that decompose a coarse
//! goal into an [`argo_workflow::Workflow`] DAG.
//!
//! Mission claim 2 says delegation is the unit — operators
//! delegate ROLES, not tasks. But coarse delegations like
//! "promote this product launch" need to break into smaller
//! steps. This crate is the seam where that decomposition lives.
//!
//! ```
//! use argo_runtime::{Planner, PlannerContext, TemplatePlanner};
//! # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
//! let planner = TemplatePlanner::shipped_defaults();
//! let ctx = PlannerContext::for_worker("wkr-1");
//! let result = planner.decompose("promote our latest blog post", &ctx).await?;
//! assert!(!result.workflow.tasks.is_empty());
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

/// Dreams plan-template learning — distill cohorts of similar
/// successful Planner decompositions into [`PlanTemplate`]
/// candidates that ride the TrustGrade ladder. Stream
/// `claude-dreams-plan-templates`.
pub mod dreams_plan_templates;
/// Decompose-or-dispatch gate — `DecomposeGate` trait + the v1
/// deterministic `HeuristicGate` impl. Wired into the agent
/// dispatch loop (mission-planner-dispatch-loop-wire) so coarse
/// goals get a planner pass before falling through to single-task
/// dispatch.
pub mod dispatch_gate;
/// OOM-kill detection + bounded exponential-backoff worker
/// restart (mission-oom-restart-handler). Pure-logic primitives
/// (`classify_exit` / `decide_restart`) plus an
/// `emit_audit_row` helper for the supervisor wiring step.
pub mod oom_restart;
mod planner;
mod planner_templates;
/// Worker-pool elasticity recommender + applier
/// (`mission-worker-pool-elasticity`).
pub mod pool_elasticity;
/// Cron-tick wrapper for the elasticity recommender.
pub mod pool_elasticity_cron;

#[cfg(test)]
mod dispatch_gate_tests;
#[cfg(test)]
mod planner_tests;

pub use dispatch_gate::{
    DecomposeGate, GateError, HeuristicGate, HEURISTIC_LENGTH_THRESHOLD, HEURISTIC_LIST_MARKERS,
};
pub use planner::{
    record_decomposition, AnthropicCaller, DecomposeResult, LlmPlanner, PlanTemplate, PlannedSubtask,
    Planner, PlannerContext, PlannerError, SubtaskSink, TemplatePlanner,
};
pub use pool_elasticity::{
    apply_recommendation, emit_audit_row, latest_tick, recommend, scale_down_total, scale_up_total,
    ElasticityAuditRow, ElasticityConfig, ElasticityMode, PoolController, PoolMetricsSnapshot,
    ScaleRecommendation, TickState,
};

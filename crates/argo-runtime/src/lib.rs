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
mod planner;

#[cfg(test)]
mod planner_tests;

pub use planner::{
    record_decomposition, AnthropicCaller, DecomposeResult, LlmPlanner, PlanTemplate, PlannedSubtask,
    Planner, PlannerContext, PlannerError, SubtaskSink, TemplatePlanner,
};

//! System-job handlers wired into [`argo_cron::SystemJobRegistry`]
//! (`mission-cron-tick-maintenance-jobs`).
//!
//! Each submodule implements one canonical maintenance job:
//!
//! - [`forge_optimize`]    — every-6h Forge re-optimize sweep.
//! - [`skill_quality_recompute`] — daily 3am quality-score refresh.
//! - [`dreams_curation_tick`]    — daily 4:30am Dreams curation pass.
//! - [`ab_promotion_tick`]       — weekly Sun 5am A/B winner promote.
//!
//! All four are wrappers around existing argo-cli / argo-skill /
//! argo-forge primitives — they DO NOT re-implement the work; they
//! select the candidate set, dispatch the existing handler, and
//! aggregate per-sub-task outcomes into a structured
//! [`argo_cron::SystemJobOutcome`].
pub mod ab_promotion_tick;
pub mod audit_evidence_archival_tick;
pub mod dreams_curation_tick;
pub mod forge_optimize;
pub mod skill_quality_recompute;

#[cfg(test)]
mod ab_promotion_tick_tests;
#[cfg(test)]
mod dreams_curation_tick_tests;
#[cfg(test)]
mod forge_optimize_tests;
#[cfg(test)]
mod skill_quality_recompute_tests;

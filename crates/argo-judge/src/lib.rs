//! `argo-judge` — external-LLM adjudication of skill creation /
//! refinement (§14.3 + §25, Amendment 2).
//!
//! Every reflective skill creation / refinement event flows through
//! [`Judge::adjudicate`] before the downstream `skill_manage` write
//! commits. The judge:
//!
//! 1. Receives a [`Rubric`] (default per §25.2 or one loaded from YAML).
//! 2. Receives [`JudgeEvidence`] — task statement, agent output, tool
//!    calls + results, the proposed artifact, optional ground-truth
//!    checks.
//! 3. Calls a judge model (any [`argo_provider::Transport`]) with a
//!    structured prompt + JSON-only reply contract.
//! 4. Aggregates per-check scores into a single normalised score and a
//!    [`argo_types::JudgeVerdict`] via the rubric's thresholds.
//! 5. Returns a [`argo_types::JudgeRecord`] ready to persist;
//!    [`persist_judge_record`] writes it to `judge_record` (§7).
//!
//! ```
//! # use argo_judge::Rubric;
//! let r = Rubric::default_rubric();
//! r.validate().unwrap();
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod evidence;
mod judge;
mod persist;
mod rubric;

#[cfg(test)]
mod tests;

pub use evidence::{GroundTruthCheck, JudgeEvidence, ProposedArtifact};
pub use judge::{verdict_from_scores, Judge, LlmJudge};
pub use persist::persist_judge_record;
pub use rubric::{Check, Rubric, VerdictThresholds, DEFAULT_RUBRIC_ID, DEFAULT_RUBRIC_VERSION};

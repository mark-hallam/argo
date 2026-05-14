//! `Forge` trait + a `StubForge` for Phase-7.5.
//!
//! The real DSPy/GEPA implementation runs in a Python subprocess
//! (per §26.4); this crate exposes the orchestration surface so the
//! CLI + agent loop can talk to it through a stable Rust trait. The
//! stub returns a structured "optimizer not configured" error so the
//! CLI surfaces a useful message until the worker plugin lands.
use crate::run::{ForgeOptions, ForgeRun, ForgeStatus};
use argo_types::ArgoError;
use async_trait::async_trait;
use chrono::Utc;

/// Operator runbook for the intentionally deferred Forge optimizer.
pub const FORGE_OPTIMIZER_STATUS_DOCS: &str = "docs/skycot/operator/forge-optimizer-status.md";

/// Structured audit event emitted whenever the v1 Forge stub is invoked.
pub const FORGE_STUB_INVOCATION_EVENT: &str = "FORGE_STUB_INVOCATION";

/// Errors returned by Forge optimizer implementations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ForgeError {
    /// The real DSPy/GEPA optimizer worker is not configured for v1.
    #[error(
        "forge optimizer not configured for skill {skill_name} base {base_version}; see {docs_url}"
    )]
    OptimizerNotConfigured {
        /// Skill requested for optimization.
        skill_name: String,
        /// Base skill version requested for optimization.
        base_version: String,
        /// Operator-facing runbook with the post-v1 implementation path.
        docs_url: &'static str,
    },
}

impl ForgeError {
    fn optimizer_not_configured(opts: &ForgeOptions) -> Self {
        Self::OptimizerNotConfigured {
            skill_name: opts.skill_name.clone(),
            base_version: opts.base_version.clone(),
            docs_url: FORGE_OPTIMIZER_STATUS_DOCS,
        }
    }

    /// Operator-facing runbook link.
    #[must_use]
    pub fn docs_url(&self) -> &'static str {
        match self {
            Self::OptimizerNotConfigured { docs_url, .. } => docs_url,
        }
    }
}

impl From<ForgeError> for ArgoError {
    fn from(value: ForgeError) -> Self {
        Self::Provider(value.to_string())
    }
}

/// The Forge orchestration seam.
///
/// ```
/// # use argo_forge::Forge;
/// fn assert_object_safe(_f: &dyn Forge) {}
/// ```
#[async_trait]
pub trait Forge: Send + Sync + std::fmt::Debug {
    /// Stable id (`"stub"` / `"dspy-gepa"`).
    fn id(&self) -> &str;
    /// Run one optimization. The implementation persists progress
    /// to `forge_run` (via the caller-supplied store helpers); this
    /// trait method just returns the final state.
    async fn optimize(&self, opts: ForgeOptions) -> Result<ForgeRun, ForgeError>;
}

/// Phase-7.5 stub. Returns a `ForgeRun` in `Failed` state with a
/// clear reason so the CLI can render "configure the Python worker
/// plugin" without a crash.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubForge;

impl StubForge {
    /// New stub.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Forge for StubForge {
    fn id(&self) -> &str {
        "stub"
    }
    async fn optimize(&self, opts: ForgeOptions) -> Result<ForgeRun, ForgeError> {
        let err = ForgeError::optimizer_not_configured(&opts);
        tracing::warn!(
            target: "argo_forge::audit",
            event = FORGE_STUB_INVOCATION_EVENT,
            skill = %opts.skill_name,
            base_version = %opts.base_version,
            docs_url = err.docs_url(),
            "forge optimizer stub invoked"
        );
        Err(err)
    }
}

/// Helper used by the CLI / agent loop: aggregate one run's events
/// into a final `ForgeRun` row. Pure, no I/O — exposed for the §26.5
/// "hitting the budget cleanly aborts and persists partial state"
/// gate.
#[must_use]
pub fn finalize_run(mut run: ForgeRun, events: &[ForgeEvent], budget_usd_micros: u64) -> ForgeRun {
    let mut score = run.final_score;
    let mut candidate = run.candidate_version.clone();
    let mut iterations = run.iterations;
    let mut spend = run.spend_usd_micros;
    for e in events {
        match e {
            ForgeEvent::Iteration {
                spend_delta_usd_micros,
            } => {
                iterations = iterations.saturating_add(1);
                spend = spend.saturating_add(*spend_delta_usd_micros);
            }
            ForgeEvent::CandidateScored { version, score: s } => {
                candidate = Some(version.clone());
                score = Some(*s);
            }
            ForgeEvent::Aborted => {
                run.status = ForgeStatus::Aborted;
            }
            ForgeEvent::Failed { .. } => {
                run.status = ForgeStatus::Failed;
            }
        }
        if spend >= budget_usd_micros {
            run.status = ForgeStatus::BudgetExceeded;
            break;
        }
    }
    run.iterations = iterations;
    run.spend_usd_micros = spend;
    run.candidate_version = candidate;
    run.final_score = score;
    run.ended_at = Some(Utc::now());
    if matches!(run.status, ForgeStatus::Pending | ForgeStatus::Running) {
        run.status = ForgeStatus::Completed;
    }
    run
}

/// Streaming events the worker emits during a run; the CLI / agent
/// loop folds them into a `ForgeRun` via [`finalize_run`].
#[derive(Debug, Clone)]
pub enum ForgeEvent {
    /// One GEPA iteration completed.
    Iteration {
        /// LLM spend on this iteration (USD-micros).
        spend_delta_usd_micros: u64,
    },
    /// A candidate scored against the held-out set.
    CandidateScored {
        /// Candidate version (e.g. `"0.2-c4"`).
        version: String,
        /// Score on `[0, 1]`.
        score: f64,
    },
    /// User aborted the run.
    Aborted,
    /// Worker failure.
    Failed {
        /// Free-form reason.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::ForgeOptions;

    #[tokio::test]
    async fn stub_returns_clear_provider_error() {
        let f = StubForge::new();
        let opts = ForgeOptions::new("alpha", "0.1");
        let err = f.optimize(opts).await.unwrap_err();
        match err {
            ForgeError::OptimizerNotConfigured {
                skill_name,
                base_version,
                docs_url,
            } => {
                assert_eq!(skill_name, "alpha");
                assert_eq!(base_version, "0.1");
                assert_eq!(docs_url, FORGE_OPTIMIZER_STATUS_DOCS);
            }
        }
        let argo_error: ArgoError =
            ForgeError::optimizer_not_configured(&ForgeOptions::new("alpha", "0.1")).into();
        assert!(format!("{argo_error}").contains(FORGE_OPTIMIZER_STATUS_DOCS));
    }

    #[test]
    fn finalize_run_collects_iterations_and_spend() {
        let opts = ForgeOptions::new("alpha", "0.1");
        let run = ForgeRun::pending(&opts);
        let events = vec![
            ForgeEvent::Iteration {
                spend_delta_usd_micros: 1_000,
            },
            ForgeEvent::Iteration {
                spend_delta_usd_micros: 2_000,
            },
            ForgeEvent::CandidateScored {
                version: "0.2-c1".into(),
                score: 0.91,
            },
        ];
        let r = finalize_run(run, &events, opts.budget_usd_micros);
        assert_eq!(r.iterations, 2);
        assert_eq!(r.spend_usd_micros, 3_000);
        assert_eq!(r.candidate_version.as_deref(), Some("0.2-c1"));
        assert!((r.final_score.unwrap() - 0.91).abs() < 1e-9);
        assert_eq!(r.status, ForgeStatus::Completed);
        assert!(r.ended_at.is_some());
    }

    #[test]
    fn finalize_run_marks_budget_exceeded_when_cap_hit() {
        // §26.5 acceptance: hitting the budget cleanly aborts and
        // persists partial state.
        let mut opts = ForgeOptions::new("alpha", "0.1");
        opts.budget_usd_micros = 1_000;
        let run = ForgeRun::pending(&opts);
        let events = vec![
            ForgeEvent::Iteration {
                spend_delta_usd_micros: 600,
            },
            ForgeEvent::Iteration {
                spend_delta_usd_micros: 700, // pushes spend ≥ budget
            },
            // Trailing events should be ignored after budget exceeded.
            ForgeEvent::CandidateScored {
                version: "should-not-be-recorded".into(),
                score: 1.0,
            },
        ];
        let r = finalize_run(run, &events, opts.budget_usd_micros);
        assert_eq!(r.status, ForgeStatus::BudgetExceeded);
        assert_eq!(r.iterations, 2);
        assert_eq!(r.spend_usd_micros, 1_300);
        assert!(r.candidate_version.is_none());
    }

    #[test]
    fn finalize_run_aborted_event_marks_aborted() {
        let opts = ForgeOptions::new("alpha", "0.1");
        let run = ForgeRun::pending(&opts);
        let events = vec![ForgeEvent::Aborted];
        let r = finalize_run(run, &events, opts.budget_usd_micros);
        assert_eq!(r.status, ForgeStatus::Aborted);
    }
}

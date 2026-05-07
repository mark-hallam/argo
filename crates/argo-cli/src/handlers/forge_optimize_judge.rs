//! Judge + threshold gates for `forge optimize` auto-promote
//! (mission-wire-judge-on-forge-promote).
//!
//! Two gates run before
//! [`super::forge_optimize::run_with_events_and_run_id`] calls
//! [`argo_skill::SkillStore::promote`]:
//!
//! 1. **Threshold gate** — best eval-score must clear
//!    `auto_promote_threshold` (env override
//!    `ARGO_FORGE_PROMOTE_THRESHOLD`, default `0.85`). The 0.85
//!    default is documented in the v0.2 spec but was unenforced
//!    before this brief.
//! 2. **Judge gate** — when a [`Judge`] is wired through the call
//!    chain, the candidate body is adjudicated. Verdict below
//!    `ARGO_JUDGE_FORGE_FLOOR` (default `0.7`, higher than the
//!    skill-creation floor since promotion is more consequential)
//!    refuses promotion. Judge timeout / transport error is
//!    treated as **fail-CLOSED** (no promotion) — opposite of
//!    skill-creation, where a degraded judge falls open. Promotion
//!    is consequential; better to defer it than ship un-adjudicated
//!    code.
//!
//! Operators can override either gate manually via the existing
//! `argo skill promote` CLI.
use argo_audit::{AuditKind, AuditSink};
use argo_judge::{Judge, JudgeEvidence, ProposedArtifact};
use argo_store::SessionStore;
use argo_types::{JudgeSubject, JudgeVerdict, SessionId};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const ENV_PROMOTE_THRESHOLD: &str = "ARGO_FORGE_PROMOTE_THRESHOLD";
const DEFAULT_PROMOTE_THRESHOLD: f64 = 0.85;

const ENV_JUDGE_FORGE_FLOOR: &str = "ARGO_JUDGE_FORGE_FLOOR";
const DEFAULT_JUDGE_FORGE_FLOOR: f64 = 0.7;

const ENV_JUDGE_FORGE_TIMEOUT_MS: &str = "ARGO_JUDGE_FORGE_TIMEOUT_MS";
const DEFAULT_JUDGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Best-score-must-clear-this gate value. Reads
/// `ARGO_FORGE_PROMOTE_THRESHOLD` if present; otherwise `0.85`.
pub(crate) fn promote_threshold() -> f64 {
    std::env::var(ENV_PROMOTE_THRESHOLD)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_PROMOTE_THRESHOLD)
}

/// Judge-verdict floor for forge promotion. Higher than the
/// skill-creation floor because rolling out a regression to every
/// agent turn is more disruptive than rejecting a candidate that
/// can be re-proposed later.
pub(crate) fn judge_floor() -> f64 {
    std::env::var(ENV_JUDGE_FORGE_FLOOR)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_JUDGE_FORGE_FLOOR)
}

pub(crate) fn judge_timeout() -> Duration {
    std::env::var(ENV_JUDGE_FORGE_TIMEOUT_MS)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_JUDGE_TIMEOUT)
}

/// Outcome of running the judge against a forge candidate.
/// Internal to this module — translated into [`PromoteDecision`]
/// by [`evaluate_promote_gates`].
#[derive(Debug, PartialEq)]
pub(crate) enum JudgeOutcome {
    Allow,
    Reject(String),
    Unavailable(String),
}

/// What the gates decided. The forge_optimize loop matches on
/// this and either runs the existing [`SkillStore::promote`]
/// path or marks the run with the appropriate failure status.
#[derive(Debug, PartialEq)]
pub(crate) enum PromoteDecision {
    /// Both gates passed; proceed to promote.
    Allow,
    /// Eval score under the threshold gate.
    BelowThreshold { score: f64, threshold: f64 },
    /// Judge verdict was below floor.
    JudgeBlocked { reason: String },
    /// Judge transport error or timeout. Fail-CLOSED — skip promote.
    JudgeUnavailable { reason: String },
}

impl PromoteDecision {
    /// Returns the `last_status` string the forge_run row should
    /// carry when a gate refuses promotion. `None` for [`Self::Allow`].
    pub(crate) fn run_status(&self) -> Option<&'static str> {
        match self {
            Self::Allow => None,
            Self::BelowThreshold { .. } => Some("below_threshold"),
            Self::JudgeBlocked { .. } => Some("judge_blocked"),
            Self::JudgeUnavailable { .. } => Some("judge_unavailable"),
        }
    }
}

/// Map a [`JudgeVerdict`] to a [`JudgeOutcome`] using the same
/// rule as the skill-creation wire (`Success` always passes;
/// `Partial` passes only in lenient mode i.e. floor < 0.5;
/// `Failure` always rejects). The default floor `0.7` rejects
/// `Partial`.
pub(crate) fn classify_verdict(verdict: &JudgeVerdict, floor: f64) -> JudgeOutcome {
    match verdict {
        JudgeVerdict::Success => JudgeOutcome::Allow,
        JudgeVerdict::Partial { reason } => {
            if floor < 0.5 {
                JudgeOutcome::Allow
            } else {
                JudgeOutcome::Reject(format!("judge_low_confidence: partial: {reason}"))
            }
        }
        JudgeVerdict::Failure { reason } => {
            JudgeOutcome::Reject(format!("judge_low_confidence: failure: {reason}"))
        }
    }
}

/// Run the judge against a refined skill body, with timeout. The
/// candidate is presented as a [`JudgeSubject::SkillRefine`] (the
/// optimize loop produces refined versions of an existing skill).
pub(crate) async fn adjudicate_with_timeout(
    judge: &dyn Judge,
    session_id: SessionId,
    skill_name: &str,
    baseline_version: &str,
    candidate_body: &str,
) -> JudgeOutcome {
    let subject = JudgeSubject::SkillRefine {
        name: skill_name.to_string(),
        version_from: baseline_version.to_string(),
        version_to_hash: skill_body_hash(candidate_body),
    };
    let evidence = JudgeEvidence {
        task_statement: format!("Promote refined skill '{skill_name}' to active"),
        agent_output: String::new(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        proposed_artifact: ProposedArtifact::SkillCreate {
            name: skill_name.to_string(),
            body: candidate_body.to_string(),
        },
        ground_truth: Vec::new(),
    };
    let fut = judge.adjudicate(session_id, subject, evidence);
    let timeout = judge_timeout();
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(record)) => classify_verdict(&record.verdict, judge_floor()),
        Ok(Err(e)) => JudgeOutcome::Unavailable(format!("judge transport error: {e}")),
        Err(_) => JudgeOutcome::Unavailable(format!("judge timeout after {timeout:?}")),
    }
}

/// Decide whether the forge optimize loop is allowed to call
/// [`SkillStore::promote`]. Threshold gate runs first (cheap,
/// no async); judge gate runs only if threshold passes AND a
/// judge was wired through.
///
/// `judge = None` is the back-compat path used by the legacy
/// callers (`argo forge optimize`, `POST /forge/optimize`,
/// the daemon's `run_claimed`). They get the threshold gate
/// only — the judge gate is opt-in via callers that thread a
/// judge through `run_with_events_and_run_id_judged`.
pub(crate) async fn evaluate_promote_gates(
    score: f64,
    threshold: f64,
    judge: Option<&dyn Judge>,
    session_id: SessionId,
    skill_name: &str,
    baseline_version: &str,
    candidate_body: &str,
) -> PromoteDecision {
    if !score.is_finite() || score < threshold {
        return PromoteDecision::BelowThreshold { score, threshold };
    }
    let Some(j) = judge else {
        return PromoteDecision::Allow;
    };
    match adjudicate_with_timeout(j, session_id, skill_name, baseline_version, candidate_body).await
    {
        JudgeOutcome::Allow => PromoteDecision::Allow,
        JudgeOutcome::Reject(reason) => PromoteDecision::JudgeBlocked { reason },
        JudgeOutcome::Unavailable(reason) => PromoteDecision::JudgeUnavailable { reason },
    }
}

/// Persist an audit row recording why promotion was refused.
/// Threshold-gate refusals use `AuditKind::ForgeRunEnd` (the
/// existing run-summary kind) carrying `event:
/// forge_promote_below_threshold` in the payload. Judge-gate
/// refusals use `AuditKind::JudgeVerdict` (the canonical judge
/// kind) carrying the verdict outcome — the
/// `forge_promote_judge_blocked` / `forge_promote_judge_unavailable`
/// label lives in the payload's `event` field so downstream
/// consumers can filter on it.
pub(crate) async fn write_promote_blocked_audit(
    store: &Arc<SessionStore>,
    run_id: Uuid,
    skill_name: &str,
    best_version: &str,
    decision: &PromoteDecision,
) {
    let sink = AuditSink::new(Arc::clone(store));
    match decision {
        PromoteDecision::Allow => {}
        PromoteDecision::BelowThreshold { score, threshold } => {
            let _ = sink
                .write(
                    None,
                    AuditKind::ForgeRunEnd,
                    serde_json::json!({
                        "event": "forge_promote_below_threshold",
                        "run_id": run_id.to_string(),
                        "skill_name": skill_name,
                        "best_version": best_version,
                        "score": score,
                        "threshold": threshold,
                    }),
                )
                .await;
        }
        PromoteDecision::JudgeBlocked { reason } => {
            let _ = sink
                .write(
                    None,
                    AuditKind::JudgeVerdict,
                    serde_json::json!({
                        "event": "forge_promote_judge_blocked",
                        "run_id": run_id.to_string(),
                        "skill_name": skill_name,
                        "best_version": best_version,
                        "reason": reason,
                    }),
                )
                .await;
        }
        PromoteDecision::JudgeUnavailable { reason } => {
            let _ = sink
                .write(
                    None,
                    AuditKind::JudgeVerdict,
                    serde_json::json!({
                        "event": "forge_promote_judge_unavailable",
                        "run_id": run_id.to_string(),
                        "skill_name": skill_name,
                        "best_version": best_version,
                        "reason": reason,
                    }),
                )
                .await;
        }
    }
}

fn skill_body_hash(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    h.finalize().into()
}

#[cfg(test)]
#[path = "forge_optimize_judge_tests.rs"]
mod tests;

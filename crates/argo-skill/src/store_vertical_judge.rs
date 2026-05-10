//! Judge-gate helpers for [`crate::store_vertical`]
//! (`propose_for_review_judged`).
//!
//! Extracted from `store_vertical.rs` to keep that file under the
//! 500-LOC budget. Items here are crate-private; the only externally
//! visible behaviour is via `SkillStore::propose_for_review_judged`
//! and `propose_for_review_judged_with_evals`, which live in
//! `store_vertical.rs` and call into this module.
use argo_judge::{Judge, JudgeEvidence, ProposedArtifact};
use argo_types::{JudgeSubject, JudgeVerdict, SessionId};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Outcome of judge adjudication on a skill candidate. Internal —
/// `propose_for_review_judged` translates this into the persisted
/// lifecycle state.
#[derive(Debug, PartialEq)]
pub(crate) enum JudgeOutcome {
    Allow,
    Reject(String),
    Unavailable(String),
}

/// Default judge timeout for skill adjudication. The brief specifies
/// 30s; tight enough that a flapping judge doesn't pin every
/// `propose_for_review_judged` call, generous enough for cold-start
/// LLM calls. Tests override via `ARGO_JUDGE_SKILL_TIMEOUT_MS`.
const DEFAULT_JUDGE_TIMEOUT: Duration = Duration::from_secs(30);

const ENV_JUDGE_SKILL_TIMEOUT_MS: &str = "ARGO_JUDGE_SKILL_TIMEOUT_MS";

fn judge_timeout() -> Duration {
    std::env::var(ENV_JUDGE_SKILL_TIMEOUT_MS)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_JUDGE_TIMEOUT)
}

/// Default confidence floor below which the judge "vetoes" a skill
/// candidate. Configurable via env `ARGO_JUDGE_SKILL_FLOOR`. The
/// 3-state [`JudgeVerdict`] enum doesn't carry a numeric score
/// itself — we map it: `Success` always passes; `Partial` passes
/// only when the floor is `< 0.5` (lenient mode); `Failure` always
/// fails. Default `0.6` enforces the strict default the mission
/// claim implies ("vetoes uncertain ones").
const DEFAULT_JUDGE_FLOOR: f64 = 0.6;

const ENV_JUDGE_SKILL_FLOOR: &str = "ARGO_JUDGE_SKILL_FLOOR";

fn judge_floor() -> f64 {
    std::env::var(ENV_JUDGE_SKILL_FLOOR)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_JUDGE_FLOOR)
}

pub(crate) async fn adjudicate_with_timeout(
    judge: &dyn Judge,
    session_id: SessionId,
    skill_name: &str,
    body: &str,
) -> JudgeOutcome {
    let subject = JudgeSubject::SkillCreate {
        name: skill_name.to_string(),
        hash: skill_body_hash(body),
    };
    let evidence = JudgeEvidence {
        task_statement: format!("Create skill '{skill_name}'"),
        agent_output: String::new(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        proposed_artifact: ProposedArtifact::SkillCreate {
            name: skill_name.to_string(),
            body: body.to_string(),
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

fn skill_body_hash(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    h.finalize().into()
}

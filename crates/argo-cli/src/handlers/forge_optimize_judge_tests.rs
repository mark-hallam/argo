//! Tests for the forge-promote judge + threshold gates.
//!
//! 5 unit tests exercise the gate decision logic directly
//! (env helpers, verdict-floor mapping, threshold comparisons,
//! judge-unavailable mapping). 1 mock-judge integration smoke
//! drives a stub [`Judge`] through [`evaluate_promote_gates`]
//! end-to-end and asserts that a rejection verdict produces a
//! `JudgeBlocked` decision (i.e. promotion is refused).
use super::*;
use argo_judge::{Judge, JudgeEvidence};
use argo_types::{
    ArgoError, ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel,
    SessionId,
};
use async_trait::async_trait;
use chrono::Utc;

#[derive(Clone)]
struct StubJudge {
    verdict: JudgeVerdict,
    fail_with: Option<String>,
}

#[async_trait]
impl Judge for StubJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        _evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        if let Some(reason) = self.fail_with.as_ref() {
            return Err(ArgoError::Provider(reason.clone()));
        }
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict: self.verdict.clone(),
            judge_model: ProviderModel {
                provider: "stub".into(),
                model: "stub-judge".into(),
            },
            rubric_id: "stub".into(),
            rubric_version: "0".into(),
            evidence_hashes: Vec::new(),
            created_at: Utc::now(),
        })
    }
}

#[test]
fn classify_verdict_strict_floor_rejects_partial() {
    // Default floor (0.7, ≥ 0.5) rejects Partial verdicts —
    // refined skill needs an unambiguous Success to ship.
    let v = JudgeVerdict::Partial {
        reason: "ambiguous".into(),
    };
    let outcome = classify_verdict(&v, 0.7);
    assert!(matches!(outcome, JudgeOutcome::Reject(_)));
    let JudgeOutcome::Reject(msg) = outcome else {
        unreachable!()
    };
    assert!(msg.contains("partial"), "msg={msg}");
    assert!(msg.contains("ambiguous"), "msg={msg}");
}

#[test]
fn classify_verdict_lenient_floor_accepts_partial() {
    // Lenient floor (< 0.5) maps Partial → Allow. Failure still
    // rejects regardless of floor.
    assert_eq!(
        classify_verdict(
            &JudgeVerdict::Partial {
                reason: "x".into()
            },
            0.4
        ),
        JudgeOutcome::Allow,
    );
    assert!(matches!(
        classify_verdict(
            &JudgeVerdict::Failure {
                reason: "no".into()
            },
            0.0
        ),
        JudgeOutcome::Reject(_),
    ));
}

#[test]
fn promote_threshold_default_returns_0_85() {
    // With no env override, the default threshold matches the
    // documented 0.85 from the v0.2 spec.
    let _guard = scoped_env_unset(ENV_PROMOTE_THRESHOLD);
    assert!((promote_threshold() - 0.85).abs() < 1e-9);
}

#[test]
fn judge_floor_default_returns_0_7() {
    // Forge floor is intentionally stricter than the
    // skill-creation floor (0.6) since promotion blast-radius
    // is bigger.
    let _guard = scoped_env_unset(ENV_JUDGE_FORGE_FLOOR);
    assert!((judge_floor() - 0.7).abs() < 1e-9);
}

#[test]
fn promote_decision_run_status_labels() {
    assert_eq!(PromoteDecision::Allow.run_status(), None);
    assert_eq!(
        PromoteDecision::BelowThreshold {
            score: 0.5,
            threshold: 0.85
        }
        .run_status(),
        Some("below_threshold"),
    );
    assert_eq!(
        PromoteDecision::JudgeBlocked {
            reason: "x".into()
        }
        .run_status(),
        Some("judge_blocked"),
    );
    assert_eq!(
        PromoteDecision::JudgeUnavailable {
            reason: "x".into()
        }
        .run_status(),
        Some("judge_unavailable"),
    );
}

#[tokio::test]
async fn evaluate_promote_gates_threshold_skips_judge_call() {
    // Score below threshold → BelowThreshold; the judge isn't
    // even consulted (verified by giving it a poison-verdict that
    // would be rejected if reached, then asserting we got
    // BelowThreshold not JudgeBlocked).
    let judge = StubJudge {
        verdict: JudgeVerdict::Failure {
            reason: "would block".into(),
        },
        fail_with: None,
    };
    let decision = evaluate_promote_gates(
        0.40,
        0.85,
        Some(&judge as &dyn Judge),
        SessionId::new(),
        "alpha",
        "0.1",
        "body",
    )
    .await;
    assert_eq!(
        decision,
        PromoteDecision::BelowThreshold {
            score: 0.40,
            threshold: 0.85
        }
    );
}

#[tokio::test]
async fn evaluate_promote_gates_judge_rejects_blocks_promotion() {
    // Mock-judge integration smoke: judge returns Failure;
    // gate evaluator translates to `JudgeBlocked` so the
    // forge_optimize loop refuses promotion + writes the
    // judge-blocked audit row.
    let judge = StubJudge {
        verdict: JudgeVerdict::Failure {
            reason: "regression in error handling".into(),
        },
        fail_with: None,
    };
    let decision = evaluate_promote_gates(
        0.95,
        0.85,
        Some(&judge as &dyn Judge),
        SessionId::new(),
        "alpha",
        "0.1",
        "candidate body",
    )
    .await;
    let PromoteDecision::JudgeBlocked { reason } = decision else {
        panic!("expected JudgeBlocked, got {decision:?}");
    };
    assert!(reason.contains("failure"), "reason={reason}");
    assert!(reason.contains("regression"), "reason={reason}");
}

#[tokio::test]
async fn evaluate_promote_gates_judge_transport_error_fails_closed() {
    // Promotion is fail-CLOSED on judge availability — opposite
    // of skill creation. Transport error → JudgeUnavailable
    // (no promote), not silent allow.
    let judge = StubJudge {
        verdict: JudgeVerdict::Success,
        fail_with: Some("network down".into()),
    };
    let decision = evaluate_promote_gates(
        0.95,
        0.85,
        Some(&judge as &dyn Judge),
        SessionId::new(),
        "alpha",
        "0.1",
        "body",
    )
    .await;
    let PromoteDecision::JudgeUnavailable { reason } = decision else {
        panic!("expected JudgeUnavailable, got {decision:?}");
    };
    assert!(reason.contains("transport"), "reason={reason}");
}

#[tokio::test]
async fn evaluate_promote_gates_no_judge_legacy_back_compat() {
    // judge = None is the legacy path used by every existing
    // caller until they thread a judge through. Threshold gate
    // still applies; judge gate is no-op.
    let decision = evaluate_promote_gates(
        0.95,
        0.85,
        None,
        SessionId::new(),
        "alpha",
        "0.1",
        "body",
    )
    .await;
    assert_eq!(decision, PromoteDecision::Allow);
}

/// Snapshot a single env var, unset it for the test body,
/// restore it on drop. Tests that read env vars need this so
/// that the test process's outer env doesn't bleed in.
struct EnvUnsetGuard {
    key: &'static str,
    prev: Option<std::ffi::OsString>,
}

fn scoped_env_unset(key: &'static str) -> EnvUnsetGuard {
    let prev = std::env::var_os(key);
    std::env::remove_var(key);
    EnvUnsetGuard { key, prev }
}

impl Drop for EnvUnsetGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

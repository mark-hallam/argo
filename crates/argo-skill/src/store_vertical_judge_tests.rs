//! Unit tests for the judge-gated `propose_for_review_judged`
//! variant added by `mission-wire-judge-on-skill-creation`.
//!
//! Five cases per the brief acceptance:
//!
//! 1. Judge approves (`Success`) → `PendingReview`
//! 2. Judge rejects (`Failure`) → `Rejected` with stamped reason
//! 3. Judge times out → `PendingReview` (fail-OPEN) + tracing warn
//! 4. Floor configurable via `ARGO_JUDGE_SKILL_FLOOR` —
//!    lenient floor lets `Partial` through
//! 5. Back-compat: `propose_for_review` (no judge) still works

use crate::skill::LifecycleState;
use crate::store::SkillStore;
use argo_judge::{Judge, JudgeEvidence};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel,
    SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;

async fn fresh() -> (SkillStore, SessionId) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    // The skill table's `learned_in` column is BLOB without an FK,
    // so any UUID works here — mirrors the established
    // `propose_for_review_starts_in_pending_review` pattern.
    (SkillStore::new(Arc::new(store)), SessionId::new())
}

fn skill_doc(name: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1\"\ndescription: judge test skill\nlock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}",
    )
}

/// Mock judge whose verdict is fixed at construction.
struct StubJudge {
    verdict: JudgeVerdict,
}

impl StubJudge {
    fn new(verdict: JudgeVerdict) -> Self {
        Self { verdict }
    }
}

#[async_trait]
impl Judge for StubJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict: self.verdict.clone(),
            judge_model: ProviderModel {
                provider: "test-judge".into(),
                model: "test-model".into(),
            },
            rubric_id: "stub".to_string(),
            rubric_version: "0".to_string(),
            evidence_hashes: evidence.evidence_hashes(),
            created_at: Utc::now(),
        })
    }
}

/// Mock judge whose `adjudicate` sleeps past `JUDGE_TIMEOUT` so the
/// outer `tokio::time::timeout` in `propose_for_review_judged`
/// fires.
struct SleepyJudge;

#[async_trait]
impl Judge for SleepyJudge {
    async fn adjudicate(
        &self,
        _session_id: SessionId,
        _subject: JudgeSubject,
        _evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        // 35s > the executor's 30s timeout. Tokio's
        // `timeout` cancels this future cleanly so the test
        // wall-clock is bounded by JUDGE_TIMEOUT, not 35s.
        tokio::time::sleep(Duration::from_secs(35)).await;
        Err(ArgoError::Provider("unreachable".into()))
    }
}

/// Mock judge whose `adjudicate` always errors. Models a transport
/// failure (provider down, JSON parse error, etc.).
struct BrokenJudge;

#[async_trait]
impl Judge for BrokenJudge {
    async fn adjudicate(
        &self,
        _session_id: SessionId,
        _subject: JudgeSubject,
        _evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        Err(ArgoError::Provider("simulated transport error".into()))
    }
}

#[tokio::test]
async fn judge_success_lands_pending_review() {
    let (skills, sid) = fresh().await;
    let judge = StubJudge::new(JudgeVerdict::Success);
    let skill = skills
        .propose_for_review_judged(
            &skill_doc("happy_path", "skill body for the happy path"),
            sid,
            "alpha",
            Some(&judge),
        )
        .await
        .expect("happy path");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

#[tokio::test]
async fn judge_failure_lands_rejected() {
    let (skills, sid) = fresh().await;
    let judge = StubJudge::new(JudgeVerdict::Failure {
        reason: "looks unsafe".into(),
    });
    let skill = skills
        .propose_for_review_judged(
            &skill_doc("vetoed_skill", "this should be rejected"),
            sid,
            "alpha",
            Some(&judge),
        )
        .await
        .expect("rejection still returns Ok with Rejected state");
    assert_eq!(skill.lifecycle_state, LifecycleState::Rejected);
}

#[tokio::test]
async fn judge_partial_lands_rejected_under_default_floor() {
    let (skills, sid) = fresh().await;
    let judge = StubJudge::new(JudgeVerdict::Partial {
        reason: "ambiguous".into(),
    });
    let skill = skills
        .propose_for_review_judged(
            &skill_doc("partial_default_floor", "ambiguous body"),
            sid,
            "alpha",
            Some(&judge),
        )
        .await
        .expect("partial under default floor → rejected");
    assert_eq!(skill.lifecycle_state, LifecycleState::Rejected);
}

#[test]
fn classify_verdict_floor_is_configurable() {
    // The 3-state JudgeVerdict doesn't carry a numeric score, so
    // the floor's role is to flip whether `Partial` counts as
    // "below" or "above". Strict mode (default 0.6) rejects
    // `Partial`; lenient mode (< 0.5) allows it. `Failure` always
    // rejects regardless. `Success` always allows.
    //
    // Tested via the pure `classify_verdict` helper to avoid
    // parallel-unsafe `std::env::set_var` between tests — env
    // mutation in one test races with reads in sibling tests
    // because cargo runs them on multiple threads.
    use crate::store_vertical_judge::{classify_verdict, JudgeOutcome};

    let strict = 0.6;
    let lenient = 0.3;

    assert_eq!(
        classify_verdict(&JudgeVerdict::Success, strict),
        JudgeOutcome::Allow
    );
    assert_eq!(
        classify_verdict(&JudgeVerdict::Success, lenient),
        JudgeOutcome::Allow
    );

    assert!(matches!(
        classify_verdict(
            &JudgeVerdict::Partial {
                reason: "ambiguous".into()
            },
            strict
        ),
        JudgeOutcome::Reject(_)
    ));
    assert_eq!(
        classify_verdict(
            &JudgeVerdict::Partial {
                reason: "ambiguous".into()
            },
            lenient
        ),
        JudgeOutcome::Allow
    );

    assert!(matches!(
        classify_verdict(
            &JudgeVerdict::Failure {
                reason: "bad".into()
            },
            strict
        ),
        JudgeOutcome::Reject(_)
    ));
    assert!(matches!(
        classify_verdict(
            &JudgeVerdict::Failure {
                reason: "bad".into()
            },
            lenient
        ),
        JudgeOutcome::Reject(_)
    ));
}

#[tokio::test]
async fn judge_transport_error_fails_open_to_pending() {
    let (skills, sid) = fresh().await;
    let judge = BrokenJudge;
    let skill = skills
        .propose_for_review_judged(
            &skill_doc("transport_err", "judge will explode"),
            sid,
            "alpha",
            Some(&judge),
        )
        .await
        .expect("transport error fails OPEN");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

#[tokio::test]
async fn judge_timeout_fails_open_to_pending() {
    // The judge timeout in `store_vertical.rs` defaults to 30s but
    // honours `ARGO_JUDGE_SKILL_TIMEOUT_MS` so tests can pin a
    // shorter budget. We set 100ms here; the `SleepyJudge` sleeps
    // 35s, so `tokio::time::timeout` fires in the test's first tick
    // — fail-OPEN must still land the candidate in `PendingReview`
    // because the brief promises a flapping judge can't block
    // skill learning.
    let prev = std::env::var("ARGO_JUDGE_SKILL_TIMEOUT_MS").ok();
    std::env::set_var("ARGO_JUDGE_SKILL_TIMEOUT_MS", "100");
    let (skills, sid) = fresh().await;
    let judge = SleepyJudge;
    let outcome = skills
        .propose_for_review_judged(
            &skill_doc("timeout_skill", "slow judge"),
            sid,
            "alpha",
            Some(&judge),
        )
        .await;
    match prev {
        Some(v) => std::env::set_var("ARGO_JUDGE_SKILL_TIMEOUT_MS", v),
        None => std::env::remove_var("ARGO_JUDGE_SKILL_TIMEOUT_MS"),
    }
    let skill = outcome.expect("timeout fails OPEN");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

#[tokio::test]
async fn back_compat_propose_for_review_unchanged_without_judge() {
    let (skills, sid) = fresh().await;
    let skill = skills
        .propose_for_review(
            &skill_doc("legacy_path", "legacy body still works"),
            sid,
            "alpha",
        )
        .await
        .expect("legacy form still works");
    // Legacy path: no judge, candidate goes straight to PendingReview
    // (matches pre-mission-wire behavior).
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

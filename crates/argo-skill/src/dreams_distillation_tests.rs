//! Unit tests for [`super`]. Lives in a sibling file via
//! `#[path] mod tests;` so the impl module keeps headroom under
//! the 500-LOC invariant — mirrors `argo-memory::dreams_backend_tests`.

use super::*;
use crate::skill::LifecycleState;
use crate::store::SkillStore;
use argo_judge::JudgeEvidence;
use argo_store::SessionStore;
use argo_types::{
    ArgoError, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel,
};
use chrono::Utc;
use std::sync::{Arc, Mutex};

fn input(
    session: &str,
    action: &str,
    vertical: &str,
    steps: &[&str],
    metric: f64,
) -> TrajectoryInput {
    TrajectoryInput {
        session_id: session.into(),
        action_type: action.into(),
        vertical_id: vertical.into(),
        trajectory: steps.iter().map(|s| (*s).to_string()).collect(),
        final_text: Some(format!("done:{session}")),
        success_metric: metric,
    }
}

/// Test stub — captures call count and serves a canned result.
struct StubCaller {
    body: String,
    cost: i64,
    model: String,
    calls: Mutex<usize>,
}

impl StubCaller {
    fn new(body: impl Into<String>, cost: i64) -> Self {
        Self {
            body: body.into(),
            cost,
            model: "claude-dreams-stub".into(),
            calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl SkillDistillationCaller for StubCaller {
    async fn distill(&self, _cohort: &Cohort) -> ArgoResult<DistillationResult> {
        *self.calls.lock().unwrap_or_else(|p| p.into_inner()) += 1;
        Ok(DistillationResult {
            body: self.body.clone(),
            model: self.model.clone(),
            cost_micros: self.cost,
        })
    }
}

#[test]
fn similarity_is_one_for_identical_sequences() {
    let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert!((trajectory_similarity(&a, &a) - 1.0).abs() < 1e-9);
}

#[test]
fn similarity_is_zero_for_disjoint_sequences() {
    let a = vec!["a".to_string(), "b".to_string()];
    let b = vec!["x".to_string(), "y".to_string()];
    assert_eq!(trajectory_similarity(&a, &b), 0.0);
}

#[test]
fn similarity_handles_empty_inputs() {
    let empty: Vec<String> = vec![];
    let some = vec!["a".to_string()];
    assert_eq!(trajectory_similarity(&empty, &some), 0.0);
    assert_eq!(trajectory_similarity(&some, &empty), 0.0);
    assert_eq!(trajectory_similarity(&empty, &empty), 0.0);
}

#[test]
fn similarity_partial_overlap_is_above_threshold_when_close() {
    // Three identical anchors + one different step → similarity
    // is dot=3/(sqrt(4)*sqrt(4))=0.75 which clears the 0.7 floor.
    let anchor = vec![
        "step1".to_string(),
        "step2".to_string(),
        "step3".to_string(),
        "step4".to_string(),
    ];
    let near = vec![
        "step1".to_string(),
        "step2".to_string(),
        "step3".to_string(),
        "different".to_string(),
    ];
    let sim = trajectory_similarity(&anchor, &near);
    assert!(sim >= COHORT_SIMILARITY_THRESHOLD, "expected >=0.7, got {sim}");
}

#[test]
fn group_into_cohorts_skips_undersized_groups() {
    // Two trajectories for one (vertical, action) pair — below
    // MIN_COHORT_SIZE, must be dropped.
    let trajectories = vec![
        input("s1", "post_listing", "marketplace", &["a", "b", "c"], 0.9),
        input("s2", "post_listing", "marketplace", &["a", "b", "c"], 0.85),
    ];
    let cohorts = group_into_cohorts(trajectories);
    assert!(cohorts.is_empty());
}

#[test]
fn group_into_cohorts_clusters_by_vertical_action_pair() {
    let trajectories = vec![
        // 3x post_listing in marketplace (cohort)
        input("s1", "post_listing", "marketplace", &["a", "b", "c"], 0.9),
        input("s2", "post_listing", "marketplace", &["a", "b", "c"], 0.85),
        input("s3", "post_listing", "marketplace", &["a", "b", "c"], 0.95),
        // 1x escalate in support (not a cohort — too small)
        input("s4", "escalate", "support", &["e1", "e2"], 0.9),
    ];
    let cohorts = group_into_cohorts(trajectories);
    assert_eq!(cohorts.len(), 1);
    assert_eq!(cohorts[0].action_type, "post_listing");
    assert_eq!(cohorts[0].vertical_id, "marketplace");
    assert_eq!(cohorts[0].members.len(), 3);
}

#[test]
fn group_into_cohorts_filters_dissimilar_members() {
    // 3 similar + 1 totally dissimilar → cohort is the 3, not 4.
    let trajectories = vec![
        input("s1", "post_listing", "marketplace", &["a", "b", "c", "d"], 0.9),
        input("s2", "post_listing", "marketplace", &["a", "b", "c", "d"], 0.85),
        input("s3", "post_listing", "marketplace", &["a", "b", "c", "d"], 0.95),
        input("s4", "post_listing", "marketplace", &["x", "y", "z"], 0.8),
    ];
    let cohorts = group_into_cohorts(trajectories);
    assert_eq!(cohorts.len(), 1);
    assert_eq!(cohorts[0].members.len(), 3);
    let ids: Vec<&str> = cohorts[0].members.iter().map(|m| m.session_id.as_str()).collect();
    assert!(!ids.contains(&"s4"));
}

#[tokio::test]
async fn distill_cohort_errors_on_undersized_cohort() {
    let caller = StubCaller::new("body", 100);
    let cohort = Cohort {
        action_type: "post_listing".into(),
        vertical_id: "marketplace".into(),
        members: vec![input("s1", "post_listing", "marketplace", &["a"], 0.9)],
    };
    let err = distill_cohort(&cohort, &caller)
        .await
        .expect_err("expected too-small error");
    let msg = format!("{err:?}");
    assert!(msg.contains("too small"), "unexpected err: {msg}");
    assert_eq!(*caller.calls.lock().unwrap(), 0, "no call should be made for tiny cohort");
}

#[tokio::test]
async fn distill_cohort_marks_body_source_dreams_distilled() {
    let caller = StubCaller::new("polished body", 750_000);
    let cohort = Cohort {
        action_type: "post_listing".into(),
        vertical_id: "marketplace".into(),
        members: vec![
            input("session1", "post_listing", "marketplace", &["nav", "fill", "submit"], 0.9),
            input("session2", "post_listing", "marketplace", &["nav", "fill", "submit"], 0.85),
            input("session3", "post_listing", "marketplace", &["nav", "fill", "submit"], 0.95),
        ],
    };
    let (candidate, result) = distill_cohort(&cohort, &caller).await.unwrap();
    assert_eq!(candidate.body_source.as_deref(), Some(BODY_SOURCE_DREAMS));
    assert_eq!(result.body, "polished body");
    assert_eq!(result.cost_micros, 750_000);
    assert_eq!(*caller.calls.lock().unwrap(), 1);
    // Description reflects the cohort size + average metric.
    assert!(candidate.description.contains("cohort of 3"));
    // Name follows convention: <action_type>_<session_short>_dreams.
    assert!(candidate.name.starts_with("post_listing_session1"));
    assert!(candidate.name.ends_with("_dreams"));
    // Average metric is mean of 0.9 / 0.85 / 0.95 = 0.9.
    assert!((candidate.success_metric - 0.9).abs() < 1e-6);
}

/// Mock judge whose verdict is fixed at construction. Mirrors the
/// `StubJudge` pattern in `store_vertical_judge_tests.rs`.
struct StubJudge {
    verdict: JudgeVerdict,
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

/// Mock judge whose `adjudicate` always errors. Models a transport
/// failure (provider down, JSON parse error, etc.) for the
/// fail-OPEN check.
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

async fn fresh_store() -> (SkillStore, SessionId) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    (SkillStore::new(Arc::new(store)), SessionId::new())
}

fn three_member_cohort(action: &str, vertical: &str) -> Cohort {
    Cohort {
        action_type: action.into(),
        vertical_id: vertical.into(),
        members: vec![
            input("session1", action, vertical, &["nav", "fill", "submit"], 0.9),
            input("session2", action, vertical, &["nav", "fill", "submit"], 0.85),
            input("session3", action, vertical, &["nav", "fill", "submit"], 0.95),
        ],
    }
}

#[tokio::test]
async fn distill_and_propose_allow_lands_pending_review() {
    let caller = StubCaller::new("polished body", 750_000);
    let (store, sid) = fresh_store().await;
    let judge = StubJudge {
        verdict: JudgeVerdict::Success,
    };
    let cohort = three_member_cohort("post_listing_allow", "marketplace");
    let (skill, result) =
        distill_and_propose_judged(&cohort, &caller, &store, sid, Some(&judge))
            .await
            .expect("allow path");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
    assert_eq!(skill.vertical_id.as_deref(), Some("marketplace"));
    assert_eq!(result.cost_micros, 750_000);
}

#[tokio::test]
async fn distill_and_propose_reject_lands_rejected() {
    let caller = StubCaller::new("polished body", 750_000);
    let (store, sid) = fresh_store().await;
    let judge = StubJudge {
        verdict: JudgeVerdict::Failure {
            reason: "looks unsafe".into(),
        },
    };
    let cohort = three_member_cohort("post_listing_reject", "marketplace");
    let (skill, _result) =
        distill_and_propose_judged(&cohort, &caller, &store, sid, Some(&judge))
            .await
            .expect("reject still returns Ok with Rejected state");
    assert_eq!(skill.lifecycle_state, LifecycleState::Rejected);
}

#[tokio::test]
async fn distill_and_propose_unavailable_fails_open_to_pending() {
    let caller = StubCaller::new("polished body", 750_000);
    let (store, sid) = fresh_store().await;
    let judge = BrokenJudge;
    let cohort = three_member_cohort("post_listing_unavailable", "marketplace");
    let (skill, _result) =
        distill_and_propose_judged(&cohort, &caller, &store, sid, Some(&judge))
            .await
            .expect("transport error fails OPEN");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

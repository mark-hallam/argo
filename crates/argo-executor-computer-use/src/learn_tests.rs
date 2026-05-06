//! Unit tests for the skill-learning bridge. Loaded via
//! `#[cfg(test)] #[path = ...]` from `learn.rs` to keep that
//! file under the per-file 500-LOC budget.
//!
//! Tests in this file are pure — no `SkillStore`, no DB. The
//! store-backed integration test lives in
//! `tests/skill_learning.rs`.

use super::*;
use crate::budget::BudgetSnapshot;
use crate::executor::Outcome;
use argo_types::SessionId;
use std::path::PathBuf;
use uuid::Uuid;

fn run_request() -> RunRequest {
    RunRequest {
        action_id: "act-abc123".into(),
        job_id: "job-1".into(),
        tenant_id: "tenant-x".into(),
        persona_id: "maya".into(),
        profile_name: "marketplace_browser".into(),
        system_prompt: "do the thing".into(),
        budget_usd: None,
        bundle_dir: None,
        session_id: None,
    }
}

fn run_result() -> RunResult {
    RunResult {
        final_text: Some("https://example.com".into()),
        actions_taken: vec![
            "step 1: navigate".into(),
            "step 2: click".into(),
            "step 3: type".into(),
        ],
        tokens_used: 12_345,
    }
}

fn budget_zero() -> BudgetSnapshot {
    BudgetSnapshot {
        tokens_used: 0,
        tokens_cap: 100,
        steps_used: 0,
        steps_cap: 5,
        wallclock_seconds_used: 0,
        wallclock_seconds_cap: 60,
    }
}

fn completed_outcome() -> Outcome {
    Outcome::Completed {
        result: run_result(),
        bundle_dir: PathBuf::from("/tmp/bundle"),
        budget: budget_zero(),
    }
}

fn handoff_outcome() -> Outcome {
    Outcome::HumanHandoff {
        reason: "budget exhausted".into(),
        bundle_dir: PathBuf::from("/tmp/bundle"),
        budget: budget_zero(),
    }
}

fn session() -> SessionId {
    SessionId(Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap())
}

#[test]
fn success_metric_is_one_for_completed_outcome() {
    let m = success_metric_for_outcome(&completed_outcome());
    assert!((m - 1.0).abs() < f64::EPSILON);
}

#[test]
fn success_metric_is_zero_for_handoff_outcome() {
    let m = success_metric_for_outcome(&handoff_outcome());
    assert!((m - 0.0).abs() < f64::EPSILON);
}

#[test]
fn should_propose_outcome_clears_completed_with_default_threshold() {
    assert!(should_propose_outcome(&completed_outcome(), DEFAULT_THRESHOLD));
    assert!(!should_propose_outcome(&handoff_outcome(), DEFAULT_THRESHOLD));
}

#[test]
fn build_candidate_has_action_type_session_prefix_in_name() {
    let req = run_request();
    let result = run_result();
    let session = session();
    let candidate = build_candidate(&req, &result, session, 1.0);
    // session_short = first 8 chars of "11111111-2222-3333-..."
    assert!(
        candidate.name.starts_with("act_"),
        "name = {}",
        candidate.name
    );
    assert!(
        candidate.name.ends_with("11111111"),
        "name = {}",
        candidate.name
    );
}

#[test]
fn build_candidate_carries_trajectory_and_final_text() {
    let req = run_request();
    let result = run_result();
    let candidate = build_candidate(&req, &result, session(), 1.0);
    assert_eq!(candidate.trajectory.len(), 3);
    assert_eq!(candidate.tokens_used, 12_345);
    assert_eq!(candidate.final_text.as_deref(), Some("https://example.com"));
    assert_eq!(candidate.vertical_id, "marketplace_browser");
    assert!((candidate.success_metric - 1.0).abs() < f64::EPSILON);
}

#[test]
fn build_candidate_falls_back_to_default_vertical_when_profile_empty() {
    let mut req = run_request();
    req.profile_name = String::new();
    let candidate = build_candidate(&req, &run_result(), session(), 1.0);
    assert_eq!(candidate.vertical_id, "default");
}

#[test]
fn session_short_takes_first_eight_chars() {
    let s = session();
    let short = session_short(s);
    assert_eq!(short.len(), 8);
    assert_eq!(short, "11111111");
}

#[tokio::test]
async fn maybe_propose_returns_none_for_handoff_outcome() {
    let req = run_request();
    let outcome = handoff_outcome();
    let result = maybe_propose(None, &req, &outcome, session(), DEFAULT_THRESHOLD).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn maybe_propose_returns_some_candidate_for_completed_outcome() {
    let req = run_request();
    let outcome = completed_outcome();
    let result = maybe_propose(None, &req, &outcome, session(), DEFAULT_THRESHOLD).await;
    let candidate = result.expect("completed outcome must produce a candidate");
    assert!(candidate.name.starts_with("act_"));
    assert!((candidate.success_metric - 1.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn maybe_propose_with_high_threshold_does_not_propose() {
    // Threshold of 1.0 means our completed-outcome metric of 1.0
    // does NOT clear the (strictly > ) check.
    let req = run_request();
    let outcome = completed_outcome();
    let result = maybe_propose(None, &req, &outcome, session(), 1.0).await;
    assert!(result.is_none());
}

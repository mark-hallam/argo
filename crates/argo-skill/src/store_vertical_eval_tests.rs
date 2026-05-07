//! Unit tests for the eval-runner gate added by
//! `claude-skill-test-execution-engine`. Wired through
//! `propose_for_review_judged_with_evals`.
//!
//! Four scenarios per the brief acceptance:
//!
//! 1. Back-compat: legacy `propose_for_review_judged` (no executor)
//!    still flows straight to PendingReview.
//! 2. Executor present + frontmatter declares no evals → still
//!    PendingReview (no work to gate on).
//! 3. Executor present + below-threshold pass-rate → Rejected with
//!    `eval_low_confidence:` reason.
//! 4. Executor present + above-threshold pass-rate → PendingReview.

use crate::eval_runner::{ActualOutcome, EvalExecutor};
use crate::skill::{EvalCase, LifecycleState, Skill};
use crate::store::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoResult, SessionId};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, Mutex};

async fn fresh() -> (SkillStore, SessionId) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    (SkillStore::new(Arc::new(store)), SessionId::new())
}

fn skill_md_with_evals(name: &str, evals_yaml: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1\"\ndescription: eval test\nlock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n{evals_yaml}---\nbody",
    )
}

/// Test executor that returns each queued output in turn.
struct StubExecutor {
    outputs: Mutex<Vec<Value>>,
}

impl StubExecutor {
    fn new(outputs: Vec<Value>) -> Self {
        Self {
            outputs: Mutex::new(outputs),
        }
    }
}

#[async_trait]
impl EvalExecutor for StubExecutor {
    async fn run_one(&self, _skill: &Skill, _case: &EvalCase) -> ArgoResult<ActualOutcome> {
        let mut q = self.outputs.lock().unwrap_or_else(|p| p.into_inner());
        let next = if q.is_empty() {
            Value::Null
        } else {
            q.remove(0)
        };
        Ok(ActualOutcome::new(next))
    }
}

#[tokio::test]
async fn back_compat_no_executor_still_pending_review() {
    let (s, sid) = fresh().await;
    let raw = skill_md_with_evals("eval-back-compat", "");
    let skill = s
        .propose_for_review_judged_with_evals(&raw, sid, "alpha", None, None)
        .await
        .expect("propose");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
    assert!(skill.frontmatter.evals.is_empty());
}

#[tokio::test]
async fn executor_with_empty_evals_falls_through_to_pending_review() {
    let (s, sid) = fresh().await;
    let raw = skill_md_with_evals("eval-empty", "");
    let exec = StubExecutor::new(Vec::new());
    let skill = s
        .propose_for_review_judged_with_evals(&raw, sid, "alpha", None, Some(&exec))
        .await
        .expect("propose");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

#[tokio::test]
async fn below_threshold_pass_rate_rejects_candidate() {
    let (s, sid) = fresh().await;
    // Two cases, Contains "ok" — executor returns "no" both times → 0/2.
    let evals_yaml = "evals:\n  - input: ping\n    expected:\n      kind: contains\n      text: ok\n  - input: ping\n    expected:\n      kind: contains\n      text: ok\n";
    let raw = skill_md_with_evals("eval-fail", evals_yaml);
    let exec = StubExecutor::new(vec![
        Value::String("no".into()),
        Value::String("no".into()),
    ]);
    let skill = s
        .propose_for_review_judged_with_evals(&raw, sid, "alpha", None, Some(&exec))
        .await
        .expect("propose");
    assert_eq!(skill.lifecycle_state, LifecycleState::Rejected);
    assert_eq!(skill.frontmatter.evals.len(), 2);
}

#[tokio::test]
async fn above_threshold_pass_rate_promotes_to_pending_review() {
    let (s, sid) = fresh().await;
    // One case Contains "ok" — executor returns "ok" → 1/1 = 1.0.
    let evals_yaml =
        "evals:\n  - input: ping\n    expected:\n      kind: contains\n      text: ok\n";
    let raw = skill_md_with_evals("eval-pass", evals_yaml);
    let exec = StubExecutor::new(vec![Value::String("ok and good".into())]);
    let skill = s
        .propose_for_review_judged_with_evals(&raw, sid, "alpha", None, Some(&exec))
        .await
        .expect("propose");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
    assert_eq!(skill.frontmatter.evals.len(), 1);
}

#[tokio::test]
async fn legacy_propose_for_review_judged_uses_no_executor() {
    // The 4-arg `propose_for_review_judged` is now a back-compat shim
    // that delegates to the 5-arg form with `evals_executor = None`.
    let (s, sid) = fresh().await;
    let evals_yaml =
        "evals:\n  - input: ping\n    expected:\n      kind: contains\n      text: ok\n";
    let raw = skill_md_with_evals("eval-legacy-shim", evals_yaml);
    // Even with evals declared, no executor means no gate.
    let skill = s
        .propose_for_review_judged(&raw, sid, "alpha", None)
        .await
        .expect("propose");
    assert_eq!(skill.lifecycle_state, LifecycleState::PendingReview);
}

//! Unit tests for [`super::feedback_eval`].
#![cfg(test)]
use super::{
    ingest_operator_feedback, weighted_loss_score, CaseKind, ExpectedOutcome, FeedbackEvalCase,
    ScoredCase,
};
use crate::config::FEEDBACK_CASE_WEIGHT;
use argo_store::feedback::{insert_feedback, Vote};
use argo_store::skill_flag::{set_flag, FLAG_NO_FORGE_FEEDBACK};
use argo_store::SessionStore;
use serde_json::json;
use std::time::Duration;

const FOURTEEN_DAYS: Duration = Duration::from_secs(14 * 24 * 3600);

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Insert a `worker_action` memory row so `runs_for_skill` returns
/// the synthetic action under `skill_name`. Mirrors the
/// production-path body shape `skill_runs_query_decode::parse_run`
/// expects.
async fn insert_worker_action(store: &SessionStore, skill_name: &str, action_id: &str) {
    let (_tenant, workspace) = store
        .ensure_workspace("feedback-eval-test", "/tmp/feedback-eval-test")
        .await
        .unwrap();
    let body = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "state": "completed",
        "payload": {
            "skill_used": skill_name,
            "worker_id": "wkr-test",
        },
        "created_at": chrono::Utc::now().to_rfc3339(),
    })
    .to_string();
    let hash = [7_u8; 32];
    sqlx::query(
        "INSERT INTO memory_entry (id, workspace_id, file, body, learned_in, learned_at, hash) \
         VALUES (?, ?, 'MEMORY.md', ?, NULL, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(&body)
    .bind(chrono::Utc::now().timestamp_micros())
    .bind(hash.as_slice())
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn empty_window_returns_empty() {
    let store = fresh_store().await;
    // No action rows, no feedback rows.
    let cases = ingest_operator_feedback(&store, "ghost-skill", FOURTEEN_DAYS)
        .await
        .unwrap();
    assert!(cases.is_empty());
}

#[tokio::test]
async fn mixed_votes_roundtrip_to_pass_and_fail_cases() {
    let store = fresh_store().await;
    insert_worker_action(&store, "alpha", "act-1").await;
    insert_worker_action(&store, "alpha", "act-2").await;
    insert_feedback(store.pool(), "act-1", Vote::Up, "loved it", None)
        .await
        .unwrap();
    insert_feedback(store.pool(), "act-2", Vote::Down, "wrong tone", None)
        .await
        .unwrap();

    let cases = ingest_operator_feedback(&store, "alpha", FOURTEEN_DAYS)
        .await
        .unwrap();
    assert_eq!(cases.len(), 2);
    let by_id: std::collections::HashMap<&str, &FeedbackEvalCase> =
        cases.iter().map(|c| (c.invocation_id.as_str(), c)).collect();
    let up = by_id["act-1"];
    let down = by_id["act-2"];
    assert_eq!(up.expected, ExpectedOutcome::Pass);
    assert_eq!(down.expected, ExpectedOutcome::Fail);
    assert_eq!(up.action_summary, "loved it");
    assert_eq!(down.action_summary, "wrong tone");
    assert_eq!(up.source, "operator_feedback");
}

#[tokio::test]
async fn window_filters_out_old_votes() {
    let store = fresh_store().await;
    insert_worker_action(&store, "alpha", "act-recent").await;
    insert_worker_action(&store, "alpha", "act-stale").await;

    // Recent vote — within the window.
    insert_feedback(store.pool(), "act-recent", Vote::Up, "fresh", None)
        .await
        .unwrap();
    // Stale vote — explicitly stamp `given_at` 30 days in the past
    // so a 14-day window rejects it.
    let stale_micros = chrono::Utc::now().timestamp_micros() - 30 * 24 * 3600 * 1_000_000;
    sqlx::query(
        "INSERT INTO operator_feedback (action_id, vote, comment, given_at, worker_id) \
         VALUES ('act-stale', 'down', 'old', ?, NULL)",
    )
    .bind(stale_micros)
    .execute(store.pool())
    .await
    .unwrap();

    let cases = ingest_operator_feedback(&store, "alpha", FOURTEEN_DAYS)
        .await
        .unwrap();
    assert_eq!(cases.len(), 1, "stale vote must be filtered out by window");
    assert_eq!(cases[0].invocation_id, "act-recent");
}

#[tokio::test]
async fn skill_flag_disable_skips_ingestion() {
    let store = fresh_store().await;
    insert_worker_action(&store, "alpha", "act-1").await;
    insert_feedback(store.pool(), "act-1", Vote::Up, "good", None)
        .await
        .unwrap();
    set_flag(store.pool(), "alpha", FLAG_NO_FORGE_FEEDBACK, 1)
        .await
        .unwrap();

    let cases = ingest_operator_feedback(&store, "alpha", FOURTEEN_DAYS)
        .await
        .unwrap();
    assert!(
        cases.is_empty(),
        "skill_flag.no-forge-feedback=1 must skip ingestion"
    );
}

#[tokio::test]
async fn ingestion_filters_votes_to_target_skill_actions() {
    let store = fresh_store().await;
    insert_worker_action(&store, "alpha", "act-alpha").await;
    insert_worker_action(&store, "beta", "act-beta").await;
    insert_feedback(store.pool(), "act-alpha", Vote::Up, "", None)
        .await
        .unwrap();
    insert_feedback(store.pool(), "act-beta", Vote::Down, "", None)
        .await
        .unwrap();

    // Pulling for `alpha` must not pick up `beta`'s vote.
    let cases = ingest_operator_feedback(&store, "alpha", FOURTEEN_DAYS)
        .await
        .unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].invocation_id, "act-alpha");
}

#[test]
fn weighted_loss_applies_feedback_case_weight() {
    // One synth (perfect score) + one operator-feedback (zero score).
    // Synth weight = 1.0, operator weight = FEEDBACK_CASE_WEIGHT = 1.5.
    let cases = [
        ScoredCase {
            kind: CaseKind::Synth,
            score: 1.0,
        },
        ScoredCase {
            kind: CaseKind::OperatorFeedback,
            score: 0.0,
        },
    ];
    let (acc, total_weight) = weighted_loss_score(&cases);
    // 1.0 * 1.0 + 1.5 * 0.0 = 1.0
    assert!((acc - 1.0).abs() < 1e-6);
    // 1.0 + 1.5 = 2.5
    assert!((total_weight - (1.0 + FEEDBACK_CASE_WEIGHT)).abs() < 1e-6);
    // Normalized = 1.0 / 2.5 = 0.4 — operator case pulled the score
    // down 1.5× harder than a synth case would have.
    let normalized = acc / total_weight;
    assert!((normalized - 0.4).abs() < 1e-6);
}

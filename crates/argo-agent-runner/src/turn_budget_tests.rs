//! Unit tests for [`super`] (turn-budget hooks). Covers the four
//! cases the brief calls out: pre-turn pass when unbounded, pre-turn
//! pass when remaining > 0, pre-turn fail when remaining <= 0, and
//! post-turn spend round-trip onto the running subtask row.
#![cfg(test)]

use super::*;
use argo_audit::AuditSink;
use argo_store::subtask::{self, NewSubtask, SubtaskState};
use argo_store::subtask_budget;
use argo_store::SessionStore;
use argo_types::{ArgoError, BudgetKind, SessionId};
use std::sync::Arc;

async fn seed_running_subtask(
    store: &Arc<SessionStore>,
    session: SessionId,
    budget_usd_cents: Option<i64>,
    initial_spent_usd_cents: i64,
) -> String {
    let id = subtask::insert(
        store.pool(),
        &NewSubtask {
            parent_goal: "test-goal",
            parent_session_id: Some(&session.0.to_string()),
            worker_id: "wkr-1",
            plan_node_path: "/test/root",
            description: "d",
        },
    )
    .await
    .unwrap();
    subtask::update_state(store.pool(), &id, SubtaskState::Running, None)
        .await
        .unwrap();
    if let Some(b) = budget_usd_cents {
        subtask_budget::set_subtask_budget(store.pool(), &id, b)
            .await
            .unwrap();
    }
    if initial_spent_usd_cents != 0 {
        subtask_budget::record_subtask_spend(store.pool(), &id, initial_spent_usd_cents)
            .await
            .unwrap();
    }
    id
}

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.unwrap();
    s.migrate().await.unwrap();
    Arc::new(s)
}

#[tokio::test]
async fn pre_turn_passes_when_no_running_subtask_for_session() {
    // Unbounded / legacy path — there's nothing in subtask_ledger
    // for this session_id, so the gate should fall through cleanly.
    let store = fresh_store().await;
    let audit = AuditSink::new(store.clone());
    let session_id = SessionId::new();
    check_pre_turn_budget(&store, session_id, &audit)
        .await
        .expect("unbounded session must not be gated");
}

#[tokio::test]
async fn pre_turn_passes_when_remaining_above_zero() {
    let store = fresh_store().await;
    let audit = AuditSink::new(store.clone());
    let session_id = SessionId::new();
    seed_running_subtask(&store, session_id, Some(500), 100).await;
    check_pre_turn_budget(&store, session_id, &audit)
        .await
        .expect("remaining 400 > 0 must not gate");
}

#[tokio::test]
async fn pre_turn_fails_when_remaining_at_or_below_zero() {
    let store = fresh_store().await;
    let audit = AuditSink::new(store.clone());
    let session_id = SessionId::new();
    seed_running_subtask(&store, session_id, Some(100), 100).await;
    let err = check_pre_turn_budget(&store, session_id, &audit)
        .await
        .expect_err("budget exhausted must gate");
    assert!(matches!(
        err,
        ArgoError::BudgetExhausted {
            kind: BudgetKind::SpendUsdMicros,
        }
    ));
}

#[tokio::test]
async fn post_turn_spend_accumulates_onto_running_subtask() {
    let store = fresh_store().await;
    let session_id = SessionId::new();
    let id = seed_running_subtask(&store, session_id, Some(10_000), 0).await;
    let spend = SpendCtx {
        tenant_id: argo_types::TenantId::new(),
        provider: "anthropic",
        model: "claude-sonnet-test",
        // $1 per 1k tokens in, $2 per 1k tokens out.
        pricing: Some((1.0, 2.0)),
    };
    // 1000 tokens in + 500 tokens out → 1.0 + 1.0 = $2.00 → 200 cents.
    record_subtask_turn_spend(&store, session_id, &spend, Some(1000), Some(500))
        .await
        .unwrap();
    let remaining = subtask_budget::get_remaining(store.pool(), &id)
        .await
        .unwrap();
    assert_eq!(remaining, Some(10_000 - 200));
}

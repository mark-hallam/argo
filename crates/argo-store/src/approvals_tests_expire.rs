//! Expire-overdue scenarios extracted from `approvals_tests.rs` so
//! the parent test file stays under the 500-LOC ceiling (HARD RULE
//! 6, brief: `codex-loc-budget-followup-wave3`). The
//! `expire_overdue` helper deserves a dedicated sibling because it
//! drives the most state-machine permutations in the suite (5
//! scenarios, ~160 LOC).

use super::*;
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

#[tokio::test]
async fn expire_overdue_flips_pending_rows_past_deadline() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    let _past = create_request(
        &pool,
        &NewApprovalRequest {
            action_id: vec![],
            tenant_id: tenant,
            requested_by: "op",
            deadline_at: now - 1,
            default_decision: DefaultDecision::Deny,
            draft_content: None,
        },
    )
    .await
    .unwrap();
    let _future = create_request(
        &pool,
        &NewApprovalRequest {
            action_id: vec![],
            tenant_id: tenant,
            requested_by: "op",
            deadline_at: now + 1_000_000,
            default_decision: DefaultDecision::Allow,
            draft_content: None,
        },
    )
    .await
    .unwrap();

    let n = expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();
    assert_eq!(n, 1, "only the past-deadline row should expire");

    let pending = list_pending(&pool, tenant, 100).await.unwrap();
    assert_eq!(pending.len(), 1, "the future row remains pending");

    // Confirm the expired row records default_decision as the decision.
    let all_rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT status, default_decision, decision, decided_by \
         FROM approval_request \
         ORDER BY deadline_at ASC",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(all_rows[0].0, "expired");
    assert_eq!(all_rows[0].1, "deny");
    assert_eq!(all_rows[0].2.as_deref(), Some("deny"));
    assert_eq!(all_rows[0].3.as_deref(), Some("system:timeout-sweep"));
    assert_eq!(all_rows[1].0, "pending");
}

#[tokio::test]
async fn expire_overdue_records_default_decision_per_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    for default in [
        DefaultDecision::Allow,
        DefaultDecision::Ask,
        DefaultDecision::Deny,
    ] {
        create_request(
            &pool,
            &NewApprovalRequest {
                action_id: vec![],
                tenant_id: tenant,
                requested_by: "op",
                deadline_at: now - 1,
                default_decision: default,
                draft_content: None,
            },
        )
        .await
        .unwrap();
    }

    let n = expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();
    assert_eq!(n, 3);

    let labels: Vec<String> = sqlx::query_scalar(
        "SELECT decision FROM approval_request \
         WHERE status = 'expired' \
         ORDER BY default_decision",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    // SQLite ORDER BY string comparison: 'allow' < 'ask' < 'deny'.
    assert_eq!(
        labels,
        vec!["allow".to_string(), "ask".to_string(), "deny".to_string()]
    );
}

#[tokio::test]
async fn expire_overdue_ignores_already_decided_rows() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    let id = create_request(
        &pool,
        &NewApprovalRequest {
            action_id: vec![],
            tenant_id: tenant,
            requested_by: "op",
            deadline_at: now - 1,
            default_decision: DefaultDecision::Deny,
            draft_content: None,
        },
    )
    .await
    .unwrap();

    // Resolve before sweep runs.
    decide_request(&pool, &id, "grantor", ApprovalDecision::Approve)
        .await
        .unwrap();

    let n = expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();
    assert_eq!(n, 0, "already-decided row should not be re-touched");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Granted);
    assert_eq!(row.decided_by.as_deref(), Some("grantor"));
}

#[tokio::test]
async fn expire_overdue_is_idempotent_on_second_call() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    create_request(
        &pool,
        &NewApprovalRequest {
            action_id: vec![],
            tenant_id: tenant,
            requested_by: "op",
            deadline_at: now - 1,
            default_decision: DefaultDecision::Deny,
            draft_content: None,
        },
    )
    .await
    .unwrap();

    let first = expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();
    let second = expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();
    assert_eq!(first, 1);
    assert_eq!(second, 0, "second sweep finds no still-pending rows");
}

#[tokio::test]
async fn state_machine_rejects_grant_after_expire() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_000_000i64;
    let id = create_request(
        &pool,
        &NewApprovalRequest {
            action_id: vec![],
            tenant_id: tenant,
            requested_by: "op",
            deadline_at: now - 1,
            default_decision: DefaultDecision::Deny,
            draft_content: None,
        },
    )
    .await
    .unwrap();

    expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();

    let updated = decide_request(&pool, &id, "grantor", ApprovalDecision::Approve)
        .await
        .unwrap();
    assert!(!updated, "Expired -> Approve is illegal; must be a no-op");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Expired);
}

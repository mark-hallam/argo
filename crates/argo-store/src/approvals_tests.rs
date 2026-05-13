use super::*;
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

fn payload<'a>(tenant: TenantId, deadline_at: i64) -> NewApprovalRequest<'a> {
    NewApprovalRequest {
        action_id: vec![1u8, 2, 3, 4],
        tenant_id: tenant,
        requested_by: "operator-1",
        deadline_at,
        default_decision: DefaultDecision::Deny,
    }
}

#[tokio::test]
async fn migrate_yields_approval_request_table() {
    let pool = fresh_pool().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name = 'approval_request'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn migrate_creates_indexes() {
    let pool = fresh_pool().await;
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master \
         WHERE type = 'index' AND tbl_name = 'approval_request' \
         ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        names.contains(&"idx_approval_request_tenant_status".to_string()),
        "missing idx_approval_request_tenant_status, got {names:?}"
    );
    assert!(
        names.contains(&"idx_approval_request_deadline".to_string()),
        "missing idx_approval_request_deadline, got {names:?}"
    );
}

#[tokio::test]
async fn down_migration_drops_table() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 62);

    store.migrate_down_to(43).await.unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name = 'approval_request'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(count, 0, "approval_request should be dropped at v43");

    // And re-migrating UP recreates it.
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 62);
}

#[tokio::test]
async fn create_request_inserts_pending_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .expect("create");
    let row = get_request(&pool, &id)
        .await
        .expect("get")
        .expect("row exists");
    assert_eq!(row.id, id);
    assert_eq!(row.tenant_id, tenant);
    assert_eq!(row.action_id, vec![1u8, 2, 3, 4]);
    assert_eq!(row.requested_by, "operator-1");
    assert_eq!(row.deadline_at, 1_700_000_000);
    assert_eq!(row.default_decision, DefaultDecision::Deny);
    assert_eq!(row.status, ApprovalStatus::Pending);
    assert!(row.decided_at.is_none());
    assert!(row.decided_by.is_none());
    assert!(row.decision.is_none());
    assert!(row.requested_at > 0);
}

#[tokio::test]
async fn get_request_unknown_id_returns_none() {
    let pool = fresh_pool().await;
    let unknown = Uuid::now_v7().to_string();
    assert!(get_request(&pool, &unknown).await.unwrap().is_none());
}

#[tokio::test]
async fn decide_request_grants_pending_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();

    let updated = decide_request(&pool, &id, "grantor-1", ApprovalDecision::Granted)
        .await
        .expect("decide");
    assert!(updated, "first call should report rows updated");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Granted);
    assert_eq!(row.decided_by.as_deref(), Some("grantor-1"));
    assert_eq!(row.decision.as_deref(), Some("granted"));
    assert!(row.decided_at.is_some());
}

#[tokio::test]
async fn decide_request_denies_pending_row() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();

    decide_request(&pool, &id, "grantor-2", ApprovalDecision::Denied)
        .await
        .unwrap();

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Denied);
    assert_eq!(row.decision.as_deref(), Some("denied"));
}

#[tokio::test]
async fn decide_request_is_idempotent_against_repeat_calls() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();

    let first = decide_request(&pool, &id, "grantor-1", ApprovalDecision::Granted)
        .await
        .unwrap();
    let second = decide_request(&pool, &id, "grantor-2", ApprovalDecision::Denied)
        .await
        .unwrap();
    assert!(first, "first call should have updated the row");
    assert!(!second, "second call should be a no-op");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    // First call wins; second call must not overwrite the decision.
    assert_eq!(row.status, ApprovalStatus::Granted);
    assert_eq!(row.decided_by.as_deref(), Some("grantor-1"));
    assert_eq!(row.decision.as_deref(), Some("granted"));
}

#[tokio::test]
async fn decide_request_unknown_id_returns_false() {
    let pool = fresh_pool().await;
    let unknown = Uuid::now_v7().to_string();
    let updated = decide_request(&pool, &unknown, "grantor", ApprovalDecision::Granted)
        .await
        .unwrap();
    assert!(!updated);
}

#[tokio::test]
async fn list_pending_filters_by_tenant_and_status() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let id_a1 = create_request(&pool, &payload(tenant_a, 1_700_000_000))
        .await
        .unwrap();
    let _id_a2 = create_request(&pool, &payload(tenant_a, 1_700_000_001))
        .await
        .unwrap();
    let id_b = create_request(&pool, &payload(tenant_b, 1_700_000_002))
        .await
        .unwrap();

    // Decide one of tenant A's rows so it leaves the pending list.
    decide_request(&pool, &id_a1, "g", ApprovalDecision::Granted)
        .await
        .unwrap();

    let pending_a = list_pending(&pool, tenant_a, 100).await.unwrap();
    assert_eq!(pending_a.len(), 1);
    assert_ne!(pending_a[0].id, id_a1, "decided row must drop out");

    let pending_b = list_pending(&pool, tenant_b, 100).await.unwrap();
    assert_eq!(pending_b.len(), 1);
    assert_eq!(pending_b[0].id, id_b);
    assert_eq!(pending_b[0].tenant_id, tenant_b);
}

#[tokio::test]
async fn list_pending_orders_oldest_first_and_caps_at_limit() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id1 = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();
    // Sleep micro-pulse so requested_at differs in micros.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let id2 = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let _id3 = create_request(&pool, &payload(tenant, 1_700_000_000))
        .await
        .unwrap();

    let listed = list_pending(&pool, tenant, 2).await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, id1, "oldest first");
    assert_eq!(listed[1].id, id2);
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
        vec![
            "allow".to_string(),
            "ask".to_string(),
            "deny".to_string()
        ]
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
        },
    )
    .await
    .unwrap();

    // Resolve before sweep runs.
    decide_request(&pool, &id, "grantor", ApprovalDecision::Granted)
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
        },
    )
    .await
    .unwrap();

    expire_overdue(&pool, now, "system:timeout-sweep")
        .await
        .unwrap();

    let updated = decide_request(&pool, &id, "grantor", ApprovalDecision::Granted)
        .await
        .unwrap();
    assert!(!updated, "Expired -> Granted is illegal; must be a no-op");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Expired);
}

#[tokio::test]
async fn approval_status_label_round_trips() {
    for s in [
        ApprovalStatus::Pending,
        ApprovalStatus::Granted,
        ApprovalStatus::Denied,
        ApprovalStatus::Expired,
    ] {
        assert_eq!(ApprovalStatus::from_str_lossy(s.as_str()), s);
    }
    assert_eq!(
        ApprovalStatus::from_str_lossy("garbage"),
        ApprovalStatus::Pending
    );
}

#[tokio::test]
async fn default_decision_label_round_trips() {
    for d in [
        DefaultDecision::Allow,
        DefaultDecision::Ask,
        DefaultDecision::Deny,
    ] {
        assert_eq!(DefaultDecision::from_str_lossy(d.as_str()), d);
    }
    assert_eq!(
        DefaultDecision::from_str_lossy("garbage"),
        DefaultDecision::Deny
    );
}

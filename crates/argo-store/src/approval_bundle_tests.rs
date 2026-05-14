use super::*;
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

fn payload<'a>(tenant: TenantId, label: &'a str, members: &[&str]) -> NewBundle<'a> {
    NewBundle {
        tenant_id: tenant,
        label,
        member_approval_ids: members.iter().map(|s| (*s).to_string()).collect(),
    }
}

#[tokio::test]
async fn migrate_yields_approval_bundle_tables() {
    let pool = fresh_pool().await;
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name LIKE 'approval_bundle%' \
         ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        names,
        vec![
            "approval_bundle".to_string(),
            "approval_bundle_member".to_string()
        ]
    );
}

#[tokio::test]
async fn migrate_creates_indexes() {
    let pool = fresh_pool().await;
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master \
         WHERE type = 'index' AND tbl_name LIKE 'approval_bundle%' \
         ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        names.contains(&"idx_approval_bundle_tenant_status".to_string()),
        "missing tenant_status index, got {names:?}"
    );
    assert!(
        names.contains(&"idx_approval_bundle_member_bundle".to_string()),
        "missing member_bundle index, got {names:?}"
    );
}

#[tokio::test]
async fn create_bundle_inserts_pending_row_with_members() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_bundle(&pool, &payload(tenant, "social-mentions", &["a", "b", "c"]))
        .await
        .expect("create");
    let row = get_bundle(&pool, &id).await.unwrap().expect("row exists");
    assert_eq!(row.tenant_id, tenant);
    assert_eq!(row.label, "social-mentions");
    assert_eq!(row.status, BundleStatus::Pending);
    assert!(row.decided_at.is_none());
    assert!(row.decided_by.is_none());
    assert!(row.decision.is_none());
    assert_eq!(row.members, vec!["a", "b", "c"]);
    assert!(row.created_at > 0);
}

#[tokio::test]
async fn create_bundle_rejects_empty_members() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let err = create_bundle(&pool, &payload(tenant, "lonely", &[]))
        .await
        .expect_err("empty must error");
    assert!(err.to_string().contains("empty bundle"));
}

#[tokio::test]
async fn create_bundle_preserves_member_order() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_bundle(
        &pool,
        &payload(tenant, "ordered", &["zzz", "aaa", "mmm", "bbb"]),
    )
    .await
    .unwrap();
    let row = get_bundle(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.members, vec!["zzz", "aaa", "mmm", "bbb"]);
}

#[tokio::test]
async fn get_bundle_unknown_id_returns_none() {
    let pool = fresh_pool().await;
    let unknown = Uuid::now_v7().to_string();
    assert!(get_bundle(&pool, &unknown).await.unwrap().is_none());
}

#[tokio::test]
async fn decide_bundle_flips_pending_row_to_approved() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_bundle(&pool, &payload(tenant, "batch", &["a", "b"]))
        .await
        .unwrap();
    let updated = decide_bundle(&pool, &id, "operator-1", BundleDecision::Approve)
        .await
        .unwrap();
    assert!(updated, "first call should update");
    let row = get_bundle(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BundleStatus::Approved);
    assert_eq!(row.decision.as_deref(), Some("approve"));
    assert_eq!(row.decided_by.as_deref(), Some("operator-1"));
    assert!(row.decided_at.is_some());
}

#[tokio::test]
async fn decide_bundle_refuse_records_refused_state() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_bundle(&pool, &payload(tenant, "batch", &["a"]))
        .await
        .unwrap();
    decide_bundle(&pool, &id, "op", BundleDecision::Refuse)
        .await
        .unwrap();
    let row = get_bundle(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BundleStatus::Refused);
    assert_eq!(row.decision.as_deref(), Some("refuse"));
}

#[tokio::test]
async fn decide_bundle_is_idempotent_against_repeat_calls() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id = create_bundle(&pool, &payload(tenant, "batch", &["a"]))
        .await
        .unwrap();
    let first = decide_bundle(&pool, &id, "op-1", BundleDecision::Approve)
        .await
        .unwrap();
    let second = decide_bundle(&pool, &id, "op-2", BundleDecision::Refuse)
        .await
        .unwrap();
    assert!(first);
    assert!(!second, "second call must be a no-op");
    let row = get_bundle(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BundleStatus::Approved);
    assert_eq!(row.decided_by.as_deref(), Some("op-1"));
    assert_eq!(row.decision.as_deref(), Some("approve"));
}

#[tokio::test]
async fn list_pending_bundles_filters_by_tenant_and_status() {
    let pool = fresh_pool().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let id_a1 = create_bundle(&pool, &payload(tenant_a, "a1", &["x"]))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let _id_a2 = create_bundle(&pool, &payload(tenant_a, "a2", &["y"]))
        .await
        .unwrap();
    let id_b = create_bundle(&pool, &payload(tenant_b, "b", &["z"]))
        .await
        .unwrap();
    decide_bundle(&pool, &id_a1, "op", BundleDecision::Approve)
        .await
        .unwrap();
    let pending_a = list_pending_bundles(&pool, tenant_a, 100).await.unwrap();
    assert_eq!(pending_a.len(), 1);
    assert_ne!(pending_a[0].id, id_a1, "decided row must drop out");
    assert_eq!(pending_a[0].label, "a2");
    let pending_b = list_pending_bundles(&pool, tenant_b, 100).await.unwrap();
    assert_eq!(pending_b.len(), 1);
    assert_eq!(pending_b[0].id, id_b);
    assert_eq!(pending_b[0].members, vec!["z"]);
}

#[tokio::test]
async fn list_pending_bundles_orders_oldest_first_and_caps_at_limit() {
    let pool = fresh_pool().await;
    let tenant = TenantId::new();
    let id1 = create_bundle(&pool, &payload(tenant, "1", &["a"]))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let id2 = create_bundle(&pool, &payload(tenant, "2", &["b"]))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let _id3 = create_bundle(&pool, &payload(tenant, "3", &["c"]))
        .await
        .unwrap();
    let listed = list_pending_bundles(&pool, tenant, 2).await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, id1, "oldest first");
    assert_eq!(listed[1].id, id2);
}

#[tokio::test]
async fn decide_bundle_unknown_id_returns_false() {
    let pool = fresh_pool().await;
    let unknown = Uuid::now_v7().to_string();
    let updated = decide_bundle(&pool, &unknown, "op", BundleDecision::Approve)
        .await
        .unwrap();
    assert!(!updated);
}

#[tokio::test]
async fn bundle_status_label_round_trips() {
    for s in [
        BundleStatus::Pending,
        BundleStatus::Approved,
        BundleStatus::Refused,
    ] {
        assert_eq!(BundleStatus::from_str_lossy(s.as_str()), s);
    }
    assert_eq!(
        BundleStatus::from_str_lossy("garbage"),
        BundleStatus::Pending
    );
}

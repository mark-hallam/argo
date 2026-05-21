//! Tenant A worker's score must NOT include tenant B's audit
//! events. The brief's gotcha #3 calls this out explicitly — the
//! signal queries must filter by `tenant_id AND worker_id`.

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_trust_engine::compute_score;
use argo_types::TenantId;
use serde_json::json;
use std::sync::Arc;

async fn fresh() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn make_worker(store: &Arc<SessionStore>, tenant: TenantId) -> WorkerId {
    let worker = Worker::draft(
        tenant,
        "vert",
        Persona::named("Cross", "op"),
        Role::new("Tenant boundary test."),
    );
    IdentityStore::new(store).create_worker(&worker).await.unwrap();
    worker.id
}

async fn emit(
    sink: &AuditSink,
    tenant: &TenantId,
    kind: AuditKind,
    worker_id: &WorkerId,
) {
    sink.write_with_tenant(
        None,
        kind,
        json!({ "worker_id": worker_id.to_string() }),
        None,
        &tenant.to_string(),
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
}

#[tokio::test]
async fn tenant_a_score_unaffected_by_tenant_b_events() {
    let store = fresh().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = make_worker(&store, tenant_a).await;
    let worker_b = make_worker(&store, tenant_b).await;

    let sink_a = AuditSink::new_with_tenant(Arc::clone(&store), tenant_a.to_string());
    let sink_b = AuditSink::new_with_tenant(Arc::clone(&store), tenant_b.to_string());

    // Tenant A: 2 approvals for worker_a.
    for _ in 0..2 {
        emit(&sink_a, &tenant_a, AuditKind::ApprovalGranted, &worker_a).await;
    }
    // Tenant B: 10 approvals for worker_b — should NOT bleed into A.
    for _ in 0..10 {
        emit(&sink_b, &tenant_b, AuditKind::ApprovalGranted, &worker_b).await;
    }

    let score_a = compute_score(&store, &tenant_a, &worker_a.to_string())
        .await
        .unwrap();
    let score_b = compute_score(&store, &tenant_b, &worker_b.to_string())
        .await
        .unwrap();

    // Tenant A worker sees ONLY its 2 grants — not B's 10.
    assert_eq!(score_a.approval_score, 2.0, "tenant A leaked: {score_a:?}");
    assert!(
        score_b.approval_score >= 10.0,
        "tenant B's own counts dropped: {score_b:?}"
    );
}

#[tokio::test]
async fn worker_id_filter_isolates_within_tenant() {
    // Two workers in the same tenant — counts must NOT mix.
    let store = fresh().await;
    let tenant = TenantId::new();
    let w1 = make_worker(&store, tenant).await;
    let w2 = make_worker(&store, tenant).await;

    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    emit(&sink, &tenant, AuditKind::ApprovalGranted, &w1).await;
    emit(&sink, &tenant, AuditKind::ApprovalGranted, &w2).await;
    emit(&sink, &tenant, AuditKind::ApprovalGranted, &w2).await;

    let s1 = compute_score(&store, &tenant, &w1.to_string()).await.unwrap();
    let s2 = compute_score(&store, &tenant, &w2.to_string()).await.unwrap();
    assert_eq!(s1.approval_score, 1.0, "w1 picked up w2 grants: {s1:?}");
    assert_eq!(s2.approval_score, 2.0, "w2 missing its grants: {s2:?}");
}

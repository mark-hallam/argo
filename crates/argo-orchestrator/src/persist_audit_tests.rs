//! Unit tests for `persist_audit` — the worker→worker handoff
//! audit-row writer. Lives in a sibling file so `persist_audit.rs`
//! stays under the brief's ≤200 LOC budget; mirrors the
//! `subtask_tests.rs` / `tenant_quota_tests.rs` split pattern.

use super::*;
use crate::{Handoff, HandoffStatus, OrchestratorStore, Session};
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let s = Arc::new(SessionStore::open(":memory:").await.unwrap());
    s.migrate().await.unwrap();
    s
}

async fn seeded_handoff(repo: &OrchestratorStore) -> Handoff {
    let tenant = TenantId::new();
    let session = Session::new(tenant, "h-audit");
    repo.insert_session(&session).await.unwrap();
    let h = Handoff::new(
        session.id.clone(),
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "verify the listing",
        serde_json::json!({"sku": "USD-AEON-60"}),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();
    h
}

async fn audit_rows_of_kind(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    sid: &uuid::Uuid,
    kind: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_record WHERE session_id = ? AND kind = ?",
    )
    .bind(sid.as_bytes().as_slice())
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn happy_path_writes_one_row() {
    let store = store().await;
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let h = seeded_handoff(&repo).await;
    audit_worker_handoff(store.pool(), &h).await.unwrap();
    let sid = uuid::Uuid::parse_str(h.session_id.as_str()).unwrap();
    assert_eq!(
        audit_rows_of_kind(store.pool(), &sid, WORKER_HANDOFF_KIND).await,
        1
    );
}

#[tokio::test]
async fn idempotent_reack_no_extra_row() {
    // Caller-side gate: only a Pending→Acknowledged transition
    // calls `audit_worker_handoff`. This test asserts the gate
    // by going through `update_handoff_status` and re-acking an
    // already-acknowledged handoff.
    let store = store().await;
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();
    let session = Session::new(tenant, "h-reack");
    repo.insert_session(&session).await.unwrap();
    let h = Handoff::new(
        session.id.clone(),
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "settle the case",
        serde_json::json!({"case_id": "c-42"}),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();
    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Acknowledged)
        .await
        .unwrap();
    repo.update_handoff_status(tenant, &h.id, HandoffStatus::Acknowledged)
        .await
        .unwrap();
    let sid = uuid::Uuid::parse_str(h.session_id.as_str()).unwrap();
    assert_eq!(
        audit_rows_of_kind(store.pool(), &sid, WORKER_HANDOFF_KIND).await,
        1
    );
}

#[tokio::test]
async fn human_target_recorded_with_kind_human() {
    let store = store().await;
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();
    let session = Session::new(tenant, "h-human");
    repo.insert_session(&session).await.unwrap();
    let h = Handoff::new(
        session.id.clone(),
        WorkerId::new(),
        EscalationStep::Human("oncall@example.com".to_string()),
        "page oncall",
        serde_json::json!({}),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();
    let canonical = canonical_payload(&h).unwrap();
    let v: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    assert_eq!(v["to_kind"], "human");
    assert_eq!(v["to_addr"], "oncall@example.com");
    assert!(v["to_worker_id"].is_null());
}

#[tokio::test]
async fn payload_hash_stable_across_calls() {
    let p = serde_json::json!({"sku": "USD-AEON-60", "qty": 3});
    let a = payload_hash(&p);
    let b = payload_hash(&p);
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
    // Different payload → different hash.
    let c = payload_hash(&serde_json::json!({"sku": "USD-AEON-60", "qty": 4}));
    assert_ne!(a, c);
}

#[tokio::test]
async fn audit_kind_exact_string_worker_handoff() {
    let store = store().await;
    let repo = OrchestratorStore::new(store.clone());
    repo.ensure_schema().await.unwrap();
    let h = seeded_handoff(&repo).await;
    audit_worker_handoff(store.pool(), &h).await.unwrap();
    let kind: String = sqlx::query_scalar(
        "SELECT kind FROM audit_record WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(
        uuid::Uuid::parse_str(h.session_id.as_str())
            .unwrap()
            .as_bytes()
            .as_slice(),
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(kind, "worker_handoff");
}

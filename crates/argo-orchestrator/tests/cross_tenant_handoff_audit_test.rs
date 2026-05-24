//! Cross-tenant handoff audit-emission contract
//! (Stream `mission-orchestrator-cross-tenant-handoff-audit-kind`).
//!
//! Sibling to `cross_tenant_handoff_test.rs`: that file pins the
//! byte-identical-NotFound contract on the public API; this file
//! pins the *operator visibility* half — when a cross-tenant
//! handoff attempt is refused, one
//! [`argo_audit::AuditKind::HandoffCrossTenantAttempt`] row lands
//! on the victim tenant's session chain so the leak attempt shows
//! up in `argo audit show`. Genuine "no such session anywhere"
//! inserts emit no row. The two halves run together so a future
//! refactor cannot silently swap one for the other.
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, OrchestratorStore, Session, SessionId};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = Arc::new(SessionStore::open(":memory:").await.unwrap());
    s.migrate().await.unwrap();
    s
}

async fn count_cross_tenant_attempt_rows(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    session_id: &SessionId,
) -> i64 {
    let sid = uuid::Uuid::parse_str(session_id.as_str()).unwrap();
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_record \
         WHERE session_id = ? AND kind = 'handoff.cross_tenant_attempt'",
    )
    .bind(sid.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn fetch_cross_tenant_payload(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    session_id: &SessionId,
) -> serde_json::Value {
    let sid = uuid::Uuid::parse_str(session_id.as_str()).unwrap();
    let json: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE session_id = ? AND kind = 'handoff.cross_tenant_attempt' \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(sid.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .unwrap();
    serde_json::from_str(&json).unwrap()
}

#[tokio::test]
async fn cross_tenant_insert_handoff_emits_audit_row_on_victim_session() {
    let store = fresh_store().await;
    let pool = store.pool().clone();
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await.unwrap();

    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let session_a = Session::new(tenant_a, "tenant_a session");
    repo.insert_session(&session_a).await.unwrap();

    // tenant_b crafts a handoff targeting tenant_a's session and
    // tries to attach it. The substrate refuses with the byte-
    // identical NotFound (pinned elsewhere), and writes one
    // HandoffCrossTenantAttempt row to tenant_a's session chain.
    let h = Handoff::new(
        session_a.id.clone(),
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "verify",
        serde_json::json!({"sku": "USD-AEON-60"}),
    );
    let err = repo.insert_handoff(tenant_b, &h).await;
    assert!(err.is_err(), "cross-tenant insert must still fail");

    let rows = count_cross_tenant_attempt_rows(&pool, &session_a.id).await;
    assert_eq!(rows, 1, "expected exactly one audit row on victim chain");

    let payload = fetch_cross_tenant_payload(&pool, &session_a.id).await;
    assert_eq!(payload["session_id"], session_a.id.as_str());
    assert_eq!(payload["attempted_tenant_id"], tenant_b.0.to_string());
    assert_eq!(payload["victim_tenant_id"], tenant_a.0.to_string());
    assert_eq!(payload["to_kind"], "worker");
    assert_eq!(payload["reason"], "verify");
    assert!(
        payload["payload_hash"].as_str().is_some_and(|s| s.len() == 64),
        "payload_hash must be 32-byte hex (no payload contents leaked)"
    );
    assert!(payload["attempted_at"].as_str().is_some());
}

#[tokio::test]
async fn nonexistent_session_insert_handoff_emits_no_audit_row() {
    let store = fresh_store().await;
    let pool = store.pool().clone();
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await.unwrap();

    let tenant_b = TenantId::new();
    // A fabricated session id that exists in NO tenant — the
    // genuine nonexistent path must stay silent so the audit
    // chain isn't polluted with rows for honest typos.
    let ghost = Session::new(tenant_b, "ghost");
    let h = Handoff::new(
        ghost.id.clone(),
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "verify",
        serde_json::json!({}),
    );
    let err = repo.insert_handoff(tenant_b, &h).await;
    assert!(err.is_err(), "nonexistent insert must still fail");

    let rows = count_cross_tenant_attempt_rows(&pool, &ghost.id).await;
    assert_eq!(rows, 0, "no audit row should exist for a ghost session");
}

#[tokio::test]
async fn cross_tenant_emission_does_not_change_public_error_shape() {
    // Pins the boundary that matters: even with emission wired up,
    // the cross-tenant insert error MUST be byte-identical to the
    // nonexistent-session insert error — otherwise the existence-
    // leak contract pinned by
    // `cross_tenant_handoff_test::insert_handoff_into_other_tenants_session_is_rejected_identical_to_nonexistent_session`
    // would silently regress.
    let store = fresh_store().await;
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await.unwrap();

    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let session_a = Session::new(tenant_a, "tenant_a session");
    let session_b = Session::new(tenant_b, "tenant_b session");
    repo.insert_session(&session_a).await.unwrap();
    repo.insert_session(&session_b).await.unwrap();

    let cross = Handoff::new(
        session_a.id.clone(),
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "verify",
        serde_json::json!({}),
    );
    let ghost_session = Session::new(tenant_b, "ghost");
    let nonexistent = Handoff::new(
        ghost_session.id,
        WorkerId::new(),
        EscalationStep::Worker(WorkerId::new()),
        "verify",
        serde_json::json!({}),
    );

    let cross_err = repo.insert_handoff(tenant_b, &cross).await;
    let nonexistent_err = repo.insert_handoff(tenant_b, &nonexistent).await;

    assert_eq!(format!("{cross_err:?}"), format!("{nonexistent_err:?}"));
}

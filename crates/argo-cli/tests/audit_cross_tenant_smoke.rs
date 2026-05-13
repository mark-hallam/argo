//! Integration smoke for Stream `mission-enterprise-cross-tenant-audit`.
//!
//! Seeds two tempdir workspaces with distinct `[tenant]` blocks in
//! `argo.toml`, writes audit rows under each tenant id, and asserts
//! end-to-end isolation:
//!
//! 1. `argo_config::workspace_tenant_id` resolves distinct ids from
//!    each workspace's TOML (override path) and the path-hash
//!    fallback (when no TOML is present).
//! 2. `AuditSink::new_with_tenant` stamps the column AND
//!    `payload_json.tenant_id` so the audit-feed handler's denorm
//!    filter can scope reads without parsing every payload twice.
//! 3. Rows written under tenant-A and tenant-B never bleed across
//!    in the underlying `audit_record` table — the SELECT predicate
//!    the feed injects (`WHERE tenant_id = ?`) is sufficient on its
//!    own for isolation.
//! 4. The hash chain stays clean after tenant_id is stamped onto
//!    every new row (no chain-input regression).
//!
//! The HTTP-level cross-tenant probe (`?as_tenant=` + capability
//! header) is covered by the route-handler unit tests in
//! `routes_audit_feed_tests.rs`; this smoke is the write-side
//! complement that proves config-resolution + SQL isolation.

use argo_audit::{verify_session, AuditKind, AuditSink};
use argo_config::workspace_tenant_id;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

fn write_argo_toml(dir: &std::path::Path, body: &str) {
    std::fs::write(dir.join("argo.toml"), body).expect("write argo.toml");
}

async fn fresh_store_session() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'smoke', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    (store, session)
}

#[tokio::test]
async fn two_workspaces_resolve_to_distinct_tenant_ids() {
    let ws_a = tempfile::tempdir().expect("tempdir A");
    let ws_b = tempfile::tempdir().expect("tempdir B");
    write_argo_toml(
        ws_a.path(),
        "[tenant]\ndefault_id = \"acme-prod\"\n",
    );
    write_argo_toml(
        ws_b.path(),
        "[tenant]\ndefault_id = \"globex-prod\"\n",
    );
    let id_a = workspace_tenant_id(ws_a.path());
    let id_b = workspace_tenant_id(ws_b.path());
    assert_eq!(id_a, "acme-prod");
    assert_eq!(id_b, "globex-prod");
    assert_ne!(id_a, id_b);
}

#[tokio::test]
async fn untenanted_workspace_falls_back_to_path_hash() {
    // No argo.toml in either tempdir → both get distinct ws-<hex> ids,
    // never the `__legacy__` sentinel, and are stable across calls.
    let ws_a = tempfile::tempdir().expect("tempdir A");
    let ws_b = tempfile::tempdir().expect("tempdir B");
    let id_a = workspace_tenant_id(ws_a.path());
    let id_b = workspace_tenant_id(ws_b.path());
    assert!(id_a.starts_with("ws-"), "got {id_a}");
    assert!(id_b.starts_with("ws-"), "got {id_b}");
    assert_ne!(id_a, "__legacy__");
    assert_ne!(id_a, id_b);
    // Stability — same path resolves to same id across calls.
    assert_eq!(id_a, workspace_tenant_id(ws_a.path()));
}

#[tokio::test]
async fn audit_rows_are_isolated_per_tenant() {
    let (store, session) = fresh_store_session().await;

    let sink_a =
        AuditSink::new_with_tenant(Arc::clone(&store), "acme-prod".to_string());
    let sink_b =
        AuditSink::new_with_tenant(Arc::clone(&store), "globex-prod".to_string());

    for i in 0..3 {
        sink_a
            .write(Some(session), AuditKind::TurnStart, serde_json::json!({"i": i}), None)
            .await
            .expect("acme write");
    }
    for i in 0..5 {
        sink_b
            .write(Some(session), AuditKind::ToolCall, serde_json::json!({"i": i}), None)
            .await
            .expect("globex write");
    }
    // Drain each sink's batched-writer latency window before reading the
    // table directly. Without this the rows are queued in-memory and the
    // smoke's row-count assertions race the batched commit.
    sink_a.flush_pending().await.expect("flush acme");
    sink_b.flush_pending().await.expect("flush globex");

    let acme_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE tenant_id = 'acme-prod'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let globex_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE tenant_id = 'globex-prod'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(acme_count, 3);
    assert_eq!(globex_count, 5);

    // Per-tenant payload-tenant_id stamping matches the column —
    // verifies the denorm invariant the audit-feed handler relies on.
    let acme_payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE tenant_id = 'acme-prod' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&acme_payload).unwrap();
    assert_eq!(parsed["tenant_id"].as_str(), Some("acme-prod"));

    // Hash chain stays clean even after both tenants intermix.
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
    assert_eq!(report.rows, 8);
}

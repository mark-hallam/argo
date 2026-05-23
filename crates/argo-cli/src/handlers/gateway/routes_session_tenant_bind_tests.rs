//! Handler-direct tests for `mission-gateway-session-routes-
//! tenant-bind` (cluster 2 of `eval-gateway-uuid-bound-routes-
//! tenant-ownership-sweep`).
//!
//! Same-tenant baselines: a tenant_a session loaded with the
//! tenant_a UUID in the `X-Argo-Tenant-Id` header still 200s
//! across the 5 cluster-2 source files. Cross-tenant probes
//! (tenant_a session loaded with the tenant_b UUID header) 404
//! — the SQL-level filter in `SessionStore::get_session_for_
//! tenant` / `session_belongs_to_tenant` refuses the read.
//!
//! Direct handler tests (no boot/HTTP) so they sidestep the
//! `common::boot`-flake the integration-style cluster-2 pin
//! flagged at `crates/argo-cli/tests/gateway_uuid_routes_tenant_
//! ownership.rs` (header note).

use super::routes_audit_session::{audit_session_response, audit_session_verify_response};
use super::routes_session::{
    session_show_response_with_format, session_spend_response, session_spend_total_response,
};
use super::routes_session_stats::{session_stats_response, session_turns_response};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open mem db");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

/// Insert `(tenant_id, workspace_id, session_id)` rows directly so
/// the test controls every UUID. The gateway-style `ensure_
/// workspace` path mints UUIDs we'd then have to round-trip; this
/// keeps the fixture flat.
async fn seed_session(store: &Arc<SessionStore>) -> (TenantId, SessionId) {
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'tbind', '.', strftime('%s','now'))",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    let session = store
        .create_session(tenant, workspace, "m", "p", Some("t"), None)
        .await
        .expect("create session");
    (tenant, session)
}

fn req_with_tenant(method: &str, path: &str, tenant: &TenantId) -> String {
    format!(
        "{method} {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {}\r\n\r\n",
        tenant.0
    )
}

fn req_with_raw_tenant(method: &str, path: &str, tenant_label: &str) -> String {
    format!("{method} {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant_label}\r\n\r\n")
}

#[tokio::test]
async fn session_show_with_matching_tenant_returns_200() {
    let store = mem_store().await;
    let (tenant, session) = seed_session(&store).await;
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}"), &tenant);
    let (status, body) = session_show_response_with_format(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("200"), "status: {status} body: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(parsed["id"], sid, "echoed session id should match");
}

#[tokio::test]
async fn session_spend_total_with_matching_tenant_returns_200() {
    let store = mem_store().await;
    let (tenant, session) = seed_session(&store).await;
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}/spend/total"), &tenant);
    let (status, body) =
        session_spend_total_response(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("200"), "status: {status} body: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(parsed["session_id"], sid);
}

#[tokio::test]
async fn session_show_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new(); // a different tenant UUID
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}"), &other_tenant);
    let (status, _body) =
        session_show_response_with_format(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn session_show_with_non_uuid_tenant_header_returns_404() {
    // Mirrors the sweep-report expected experiment exactly: hit a
    // tenant_a session with `x-argo-tenant-id: tenant_b` (a raw
    // label, not a UUID). Without the bind this returned 200 + the
    // row; with the bind it 404s because `get_session_for_tenant`
    // refuses non-UUID tenant strings.
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let sid = session.0.to_string();
    let req = req_with_raw_tenant("GET", &format!("/sessions/{sid}"), "tenant_b");
    let (status, _body) =
        session_show_response_with_format(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn session_stats_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new();
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}/stats"), &other_tenant);
    let (status, _body) = session_stats_response(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn session_turns_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new();
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}/turns"), &other_tenant);
    let (status, _body) = session_turns_response(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn session_spend_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new();
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/sessions/{sid}/spend"), &other_tenant);
    let (status, _body) = session_spend_response(&sid, &req, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn audit_session_verify_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new();
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/audit/sessions/{sid}/verify"), &other_tenant);
    let (status, _body) =
        audit_session_verify_response(&sid, &req, false, Some(&store), None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

#[tokio::test]
async fn audit_session_rows_cross_tenant_returns_404() {
    let store = mem_store().await;
    let (_tenant_a, session) = seed_session(&store).await;
    let other_tenant = TenantId::new();
    let sid = session.0.to_string();
    let req = req_with_tenant("GET", &format!("/audit/sessions/{sid}"), &other_tenant);
    let (status, _body) =
        audit_session_response(&sid, &req, Some(&store), None, None, None).await;
    assert!(status.starts_with("404"), "expected 404, got {status}");
}

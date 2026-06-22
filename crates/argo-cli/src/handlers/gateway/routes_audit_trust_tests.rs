//! Tests for `routes_audit_trust` — the customer-facing trust surface
//! (`w3-audit-trust-surface`): `GET /api/v1/audit/verify`,
//! `GET /api/v1/audit/export`, `GET /api/v1/audit/cert/<action_id>`.
//!
//! Seeds a `:memory:` store and drives the handlers directly (the
//! bearer-auth gate runs upstream in `routes::dispatch_inner`). Tenant
//! is resolved server-side; in the in-crate test build the
//! `resolve_tenant_for_request` resolver honors the explicit
//! `X-Tenant-Id` override (a parseable UUID) on the no-session path, so
//! these tests set that header to choose the calling tenant. The
//! cross-tenant tests prove a caller scoped to tenant B never sees
//! tenant A's chain.

#![allow(deprecated)]

use super::{cert_response, export_response, verify_response};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Create a session bound to `tenant`, returning its id.
async fn session_for(store: &Arc<SessionStore>, tenant: TenantId) -> SessionId {
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap()
}

/// Write `n` rows under `tenant` in `session`; return the hex id of
/// the last row written.
async fn seed(store: &Arc<SessionStore>, tenant: TenantId, session: SessionId, n: usize) -> String {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    for i in 0..n {
        sink.write(Some(session), AuditKind::ToolCall, json!({"i": i}), None)
            .await
            .expect("write audit row");
    }
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .expect("read back id");
    hex::encode(id)
}

/// GET request whose server-side tenant resolves to `tenant` (test
/// build honors the explicit `X-Tenant-Id` UUID override).
fn get_as(path: &str, tenant: TenantId) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Tenant-Id: {tenant}\r\n\r\n")
}

/// Same, plus an explicit `X-Argo-Role` (honored in the test build).
fn get_as_role(path: &str, tenant: TenantId, role: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Tenant-Id: {tenant}\r\nX-Argo-Role: {role}\r\n\r\n")
}

fn parse(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1).unwrap_or_else(|_| panic!("invalid JSON: {}", resp.1))
}

// ── verify ──────────────────────────────────────────────────────

#[tokio::test]
async fn verify_reports_intact_for_clean_chain() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    seed(&store, tenant, session, 4).await;
    let resp = verify_response(&get_as("/api/v1/audit/verify", tenant), Some(&store)).await;
    assert_eq!(resp.0, "200 OK");
    let body = parse(&resp);
    assert_eq!(body["intact"], true);
    assert_eq!(body["rows_checked"], 4);
    assert_eq!(body["breaks"], 0);
    assert!(body["first_broken_seq"].is_null());
    assert_eq!(body["head_hash"].as_str().unwrap().len(), 64);
    assert!(body["verified_at"].is_string());
}

#[tokio::test]
async fn verify_reports_broken_truthfully_after_tamper() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    seed(&store, tenant, session, 4).await;
    // Tamper with the stored payload so the recompute no longer matches.
    sqlx::query("UPDATE audit_record SET payload_json = '{\"x\": 9}' WHERE seq = 2")
        .execute(store.pool())
        .await
        .unwrap();
    let resp = verify_response(&get_as("/api/v1/audit/verify", tenant), Some(&store)).await;
    assert_eq!(resp.0, "200 OK");
    let body = parse(&resp);
    assert_eq!(body["intact"], false, "must report the REAL broken state");
    assert!(body["breaks"].as_u64().unwrap() >= 1);
    assert!(body["first_broken_seq"].as_i64().is_some());
}

#[tokio::test]
async fn verify_is_tenant_scoped_other_tenants_rows_invisible() {
    let store = mem_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sa = session_for(&store, tenant_a).await;
    seed(&store, tenant_a, sa, 5).await;
    // Caller B has no rows → intact with zero rows_checked, never
    // counting tenant A's 5.
    let resp = verify_response(&get_as("/api/v1/audit/verify", tenant_b), Some(&store)).await;
    let body = parse(&resp);
    assert_eq!(body["intact"], true);
    assert_eq!(body["rows_checked"], 0);
    assert!(body["head_hash"].is_null());
}

#[tokio::test]
async fn verify_503_when_store_missing() {
    let tenant = TenantId::new();
    let resp = verify_response(&get_as("/api/v1/audit/verify", tenant), None).await;
    assert_eq!(resp.0, "503 Service Unavailable");
}

// ── export ──────────────────────────────────────────────────────

#[tokio::test]
async fn export_returns_base64_bundle_for_calling_tenant() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    seed(&store, tenant, session, 3).await;
    let resp = export_response(&get_as("/api/v1/audit/export", tenant), "", Some(&store)).await;
    assert_eq!(resp.0, "200 OK");
    let body = parse(&resp);
    assert_eq!(body["row_count"], 3);
    assert!(body["size_bytes"].as_u64().unwrap() > 0);
    assert_eq!(body["sha256"].as_str().unwrap().len(), 64);
    let bytes = general_purpose::STANDARD
        .decode(body["b64"].as_str().unwrap())
        .expect("valid base64");
    assert!(!bytes.is_empty());
}

#[tokio::test]
async fn export_only_includes_calling_tenant_rows() {
    let store = mem_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sa = session_for(&store, tenant_a).await;
    seed(&store, tenant_a, sa, 7).await;
    // Caller B's export contains zero of A's rows.
    let resp = export_response(&get_as("/api/v1/audit/export", tenant_b), "", Some(&store)).await;
    let body = parse(&resp);
    assert_eq!(body["row_count"], 0);
}

#[tokio::test]
async fn export_rejects_unknown_format() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let resp = export_response(
        &get_as("/api/v1/audit/export?format=xml", tenant),
        "format=xml",
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "400 Bad Request");
}

#[tokio::test]
async fn export_owner_only_guest_is_denied() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let resp = export_response(
        &get_as_role("/api/v1/audit/export", tenant, "guest"),
        "",
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "403 Forbidden");
}

// ── cert ────────────────────────────────────────────────────────

#[tokio::test]
async fn cert_returns_verified_certificate_for_known_action() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    let id = seed(&store, tenant, session, 4).await;
    let resp = cert_response(
        &get_as(&format!("/api/v1/audit/cert/{id}"), tenant),
        &id,
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "200 OK");
    let cert = &parse(&resp)["certificate"];
    assert_eq!(cert["action_id"], id);
    assert_eq!(cert["verified"], true);
    assert_eq!(cert["chain_rows"], 4);
    let proof = cert["proof"].as_array().expect("proof array");
    assert_eq!(proof.len(), 4);
    assert!(proof.iter().any(|s| s["is_target"] == true));
    assert!(proof.iter().all(|s| s["hash_recomputes"] == true));
}

#[tokio::test]
async fn cert_404_for_cross_tenant_action_id_leaks_nothing() {
    let store = mem_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sa = session_for(&store, tenant_a).await;
    let id = seed(&store, tenant_a, sa, 3).await;
    // Caller B asks for A's real action id → 404, body carries no
    // tenant-A data.
    let resp = cert_response(
        &get_as(&format!("/api/v1/audit/cert/{id}"), tenant_b),
        &id,
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "404 Not Found");
    assert!(resp.1.contains("not_found"));
    assert!(!resp.1.contains(&tenant_a.to_string()));
}

#[tokio::test]
async fn cert_404_for_unknown_action_id() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let missing = "00112233445566778899aabbccddeeff";
    let resp = cert_response(
        &get_as(&format!("/api/v1/audit/cert/{missing}"), tenant),
        missing,
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "404 Not Found");
}

#[tokio::test]
async fn cert_404_for_invalid_hex_id() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let resp = cert_response(
        &get_as("/api/v1/audit/cert/not-hex", tenant),
        "not-hex",
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "404 Not Found");
}

#[tokio::test]
async fn cert_reports_unverified_when_chain_tampered() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = session_for(&store, tenant).await;
    let id = seed(&store, tenant, session, 4).await;
    sqlx::query("UPDATE audit_record SET payload_json = '{\"x\": 1}' WHERE seq = 2")
        .execute(store.pool())
        .await
        .unwrap();
    let resp = cert_response(
        &get_as(&format!("/api/v1/audit/cert/{id}"), tenant),
        &id,
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "200 OK");
    let cert = &parse(&resp)["certificate"];
    assert_eq!(cert["verified"], false, "tampered chain must NOT verify");
}

#[tokio::test]
async fn cert_503_when_store_missing() {
    let tenant = TenantId::new();
    let resp = cert_response(&get_as("/api/v1/audit/cert/abc", tenant), "abc", None).await;
    assert_eq!(resp.0, "503 Service Unavailable");
}

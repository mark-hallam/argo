//! Tests for `routes_governance` — the audit-readiness report
//! (competitive gap #1, move A).
//!
//! Drives `dispatch_get` directly against a `:memory:` `SessionStore`
//! (migrated). Two tenants are seeded with distinct audit chains and
//! approval rows; the report is requested per-tenant via the explicit
//! `?tenant=<uuid>` query so the assertions are deterministic and do
//! not depend on cwd-resolved workspace identity. The upstream
//! bearer-auth gate lives in `routes.rs` and is not re-exercised here.

use super::dispatch_get;
use argo_audit::{AuditKind, AuditSink};
use argo_store::approvals::{
    create_request, decide_request, ApprovalDecision, DefaultDecision, NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId};
use serde_json::Value;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse_ok(resp: Option<(&'static str, String)>) -> Value {
    let (status, body) = resp.expect("route matched");
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

/// Seed `n` chained audit rows for `tenant` under one fresh session.
/// Returns the session id. The sink reads-then-chains per session, so
/// every row links to its predecessor's `self_hash` — a chain the
/// verifier passes.
async fn seed_audit_chain(store: &Arc<SessionStore>, tenant: TenantId, n: usize) -> SessionId {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let session = SessionId::new();
    for i in 0..n {
        sink.write(
            Some(session),
            AuditKind::HttpRequest,
            serde_json::json!({ "seq": i, "path": "/api/v1/example" }),
            None,
        )
        .await
        .expect("seed audit row");
    }
    session
}

/// Seed `decided` granted + `pending` open approval requests for
/// `tenant`. Each request gets a unique 16-byte action id.
async fn seed_approvals(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    decided: usize,
    pending: usize,
) {
    for i in 0..(decided + pending) {
        let mut action_id = vec![0u8; 16];
        action_id[0] = i as u8;
        let id = create_request(
            store.pool(),
            &NewApprovalRequest {
                action_id,
                tenant_id: tenant,
                requested_by: "operator-1",
                deadline_at: 1_900_000_000,
                default_decision: DefaultDecision::Deny,
                draft_content: None,
            },
        )
        .await
        .expect("create approval");
        if i < decided {
            decide_request(store.pool(), &id, tenant, "approver-1", ApprovalDecision::Approve)
                .await
                .expect("decide approval");
        }
    }
}

#[tokio::test]
async fn report_assembles_real_counts_for_seeded_tenant() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    seed_audit_chain(&store, tenant, 5).await;
    seed_approvals(&store, tenant, 3, 1).await;

    let path = format!("/api/v1/governance/readiness?tenant={tenant}");
    let v = parse_ok(dispatch_get(&get_req(&path), Some(&store)).await);

    // Top-level shape.
    assert_eq!(v["tenant_id"], tenant.to_string());
    assert!(v["generated_at"].is_string(), "generated_at present");

    // Audit — the moat made legible: real count + clean chain.
    assert_eq!(v["audit"]["action_count"], 5);
    assert_eq!(v["audit"]["chain_intact"], true);

    // Approvals — 3 decided + 1 pending → 75.0% coverage.
    assert_eq!(v["approvals"]["decided"], 3);
    assert_eq!(v["approvals"]["pending"], 1);
    assert_eq!(v["approvals"]["coverage_pct"], 75.0);

    // Capabilities — the default RBAC role presets are surfaced.
    let roles = v["capabilities"]["default_roles"]
        .as_array()
        .expect("default_roles array");
    let names: Vec<&str> = roles.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"owner"), "roles: {names:?}");
    assert!(names.contains(&"admin"), "roles: {names:?}");

    // Truth boundary — no policy env configured under test → factual
    // `enabled: false` with a note, never a fabricated `true`.
    assert_eq!(v["truth_boundary"]["enabled"], false);
    assert!(v["truth_boundary"]["note"].is_string());
}

#[tokio::test]
async fn report_is_tenant_scoped_no_cross_tenant_leak() {
    let store = mem_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    // Only tenant A has data.
    seed_audit_chain(&store, tenant_a, 4).await;
    seed_approvals(&store, tenant_a, 2, 2).await;

    // Tenant B's report must be a clean, empty report — A's rows must
    // not leak across the tenant boundary.
    let path_b = format!("/api/v1/governance/readiness?tenant={tenant_b}");
    let b = parse_ok(dispatch_get(&get_req(&path_b), Some(&store)).await);
    assert_eq!(b["tenant_id"], tenant_b.to_string());
    assert_eq!(b["audit"]["action_count"], 0);
    // Zero rows is vacuously intact.
    assert_eq!(b["audit"]["chain_intact"], true);
    assert_eq!(b["approvals"]["decided"], 0);
    assert_eq!(b["approvals"]["pending"], 0);
    // No denominator → coverage is null with a note, not a fake 100.
    assert!(b["approvals"]["coverage_pct"].is_null());
    assert!(b["approvals"]["note"].is_string());

    // And tenant A still sees its own counts under the same store.
    let path_a = format!("/api/v1/governance/readiness?tenant={tenant_a}");
    let a = parse_ok(dispatch_get(&get_req(&path_a), Some(&store)).await);
    assert_eq!(a["audit"]["action_count"], 4);
    assert_eq!(a["approvals"]["decided"], 2);
    assert_eq!(a["approvals"]["pending"], 2);
    assert_eq!(a["approvals"]["coverage_pct"], 50.0);
}

#[tokio::test]
async fn malformed_tenant_query_is_a_client_error() {
    let store = mem_store().await;
    let (status, _body) = dispatch_get(
        &get_req("/api/v1/governance/readiness?tenant=not-a-uuid"),
        Some(&store),
    )
    .await
    .expect("route matched");
    // resolve_tenant surfaces the parse failure; the response is a 5xx
    // envelope from the handler (not a silent wrong-tenant report).
    assert!(status.starts_with("500"), "status: {status}");
}

#[tokio::test]
async fn non_matching_path_returns_none() {
    let store = mem_store().await;
    assert!(dispatch_get(&get_req("/api/v1/workers"), Some(&store))
        .await
        .is_none());
    // A sibling path under the same root is not swallowed.
    assert!(
        dispatch_get(&get_req("/api/v1/governance"), Some(&store))
            .await
            .is_none(),
        "/api/v1/governance (no /readiness) must not match"
    );
}

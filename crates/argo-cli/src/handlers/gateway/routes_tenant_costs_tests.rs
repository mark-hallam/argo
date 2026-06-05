//! Tests for `routes_tenant_costs` — the tenant-scoped dollar surface
//! (`GET /api/v1/tenant/costs/{total,by-worker}`).
//!
//! The acceptance gate for this brief is cross-tenant isolation: a
//! request resolved to tenant A must never see tenant B's spend. The
//! request carries no session cookie, so the resolver falls back to the
//! cwd workspace tenant — that is "tenant A" here; we seed a second
//! `ensure_workspace` to act as "tenant B".

use super::{by_worker_response, total_response};
use argo_store::{SessionStore, WorkerId};
use argo_types::{SessionId, TenantId};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// The cwd workspace tenant — what the cookie-less resolver returns.
async fn cwd_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ensure cwd ws");
    tenant
}

async fn other_tenant(store: &Arc<SessionStore>) -> TenantId {
    let (tenant, _) = store
        .ensure_workspace("tenant-b", "/tmp/skycot-test-tenant-b")
        .await
        .expect("ensure other ws");
    tenant
}

async fn seed(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker: Option<WorkerId>,
    micros: u64,
) {
    store
        .record_spend_with_worker(
            tenant,
            SessionId(Uuid::now_v7()),
            chrono::Utc::now().timestamp_micros(),
            micros,
            "anthropic",
            "claude-opus-4-8",
            worker,
        )
        .await
        .expect("record spend");
}

fn total_req() -> String {
    "GET /api/v1/tenant/costs/total HTTP/1.1\r\nHost: x\r\n\r\n".to_string()
}

fn by_worker_req() -> String {
    "GET /api/v1/tenant/costs/by-worker HTTP/1.1\r\nHost: x\r\n\r\n".to_string()
}

#[tokio::test]
async fn total_sums_only_the_resolved_tenants_spend() {
    let store = mem_store().await;
    let a = cwd_tenant(&store).await;
    let b = other_tenant(&store).await;
    seed(&store, a, Some(WorkerId::new()), 1_000_000).await;
    seed(&store, a, None, 500_000).await;
    // Tenant B spend must NOT leak into tenant A's total.
    seed(&store, b, Some(WorkerId::new()), 9_000_000).await;

    let (status, body) = total_response(&total_req(), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["spent_usd_micros"], 1_500_000);
}

#[tokio::test]
async fn by_worker_groups_per_teammate_and_excludes_other_tenants() {
    let store = mem_store().await;
    let a = cwd_tenant(&store).await;
    let b = other_tenant(&store).await;
    let w1 = WorkerId::new();
    let w2 = WorkerId::new();
    seed(&store, a, Some(w1), 3_000_000).await;
    seed(&store, a, Some(w1), 1_000_000).await; // same teammate accumulates
    seed(&store, a, Some(w2), 2_000_000).await;
    seed(&store, a, None, 250_000).await; // system / non-teammate bucket
    seed(&store, b, Some(WorkerId::new()), 8_000_000).await; // other tenant

    let (status, body) = by_worker_response(&by_worker_req(), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let rows = v["by_worker"].as_array().expect("array");
    // w1 (4M), w2 (2M), system/null (250k) — three buckets, tenant B absent.
    assert_eq!(rows.len(), 3, "rows: {rows:?}");
    // Ordered by total DESC → w1 first.
    assert_eq!(rows[0]["worker_id"], w1.to_string());
    assert_eq!(rows[0]["spent_usd_micros"], 4_000_000);
    assert_eq!(rows[1]["worker_id"], w2.to_string());
    assert_eq!(rows[1]["spent_usd_micros"], 2_000_000);
    // The NULL worker_id bucket surfaces as JSON null.
    assert!(rows[2]["worker_id"].is_null());
    assert_eq!(rows[2]["spent_usd_micros"], 250_000);
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let (status, _) = total_response(&total_req(), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let (status, _) = by_worker_response(&by_worker_req(), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

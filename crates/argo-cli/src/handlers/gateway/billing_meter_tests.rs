//! Tests for `billing_meter` — `GET /api/v1/billing/meter` and
//! `POST /api/v1/billing/cap` (DEC-053 overage + customer cap).
//!
//! In-crate `cfg!(test)` requests with no `X-Argo-Role` header resolve
//! to Owner (see `role_gate::effective_role`), so they clear both the
//! `BillingView` read gate and the `BillingManage` owner-only cap gate.

use super::{meter_response, set_cap_response};
use argo_store::tenant_query::increment_tenant_usage;
use argo_store::tenant_quota::set_workspace_tier;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

fn meter_req() -> String {
    "GET /api/v1/billing/meter HTTP/1.1\r\nHost: x\r\n\r\n".to_string()
}

fn cap_req(cap_usd: &str) -> String {
    format!("POST /api/v1/billing/cap HTTP/1.1\r\nHost: x\r\n\r\n{{\"cap_usd\":{cap_usd}}}")
}

async fn meter_ok(store: &Arc<SessionStore>) -> Value {
    let (status, body) = meter_response(&meter_req(), Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[tokio::test]
async fn fresh_free_tier_has_no_allowance_or_cap() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let v = meter_ok(&store).await;
    assert_eq!(v["tier"], "free");
    assert_eq!(v["included_actions"], 0);
    assert_eq!(v["used_actions"], 0);
    assert_eq!(v["overage_units"], 0);
    // Free carries no default_monthly_cap_usd → null.
    assert!(v["cap_usd"].is_null());
}

#[tokio::test]
async fn pro_under_allowance_reports_zero_overage() {
    // Acceptance 1a: usage ≤ allowance → 0 overage.
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    set_workspace_tier(&store, tenant, "pro", "Test Pro")
        .await
        .expect("set pro");
    increment_tenant_usage(&store, tenant, 1_000, Utc::now())
        .await
        .expect("increment");
    let v = meter_ok(&store).await;
    assert_eq!(v["tier"], "pro");
    assert_eq!(v["included_actions"], 1_500);
    assert_eq!(v["used_actions"], 1_000);
    assert_eq!(v["overage_units"], 0);
    // Default cap surfaced (Pro = $200) since none explicitly set.
    assert_eq!(v["cap_usd"], 200.0);
    assert_eq!(v["cap_is_default"], true);
}

#[tokio::test]
async fn pro_over_allowance_reports_exact_overage_units() {
    // Acceptance 1b: usage over allowance → (used − allowance) units.
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    set_workspace_tier(&store, tenant, "pro", "Test Pro")
        .await
        .expect("set pro");
    increment_tenant_usage(&store, tenant, 2_000, Utc::now())
        .await
        .expect("increment");
    let v = meter_ok(&store).await;
    assert_eq!(v["used_actions"], 2_000);
    assert_eq!(v["overage_units"], 500);
    assert!((v["overage_multiplier"].as_f64().unwrap() - 1.4).abs() < f64::EPSILON);
}

#[tokio::test]
async fn set_cap_then_meter_reflects_explicit_cap() {
    // Acceptance 1c surface: the customer-visible self-set hard cap.
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    set_workspace_tier(&store, tenant, "pro", "Test Pro")
        .await
        .expect("set pro");
    let (status, body) = set_cap_response(&cap_req("50"), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let set: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(set["cap_usd"], 50.0);

    let v = meter_ok(&store).await;
    assert_eq!(v["cap_usd"], 50.0);
    assert_eq!(v["cap_is_default"], false);
}

#[tokio::test]
async fn set_cap_rejects_negative() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) = set_cap_response(&cap_req("-5"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
}

#[tokio::test]
async fn meter_store_unavailable_returns_503() {
    let (status, body) = meter_response(&meter_req(), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("unavailable"), "body: {body}");
}

#[tokio::test]
async fn set_cap_store_unavailable_returns_503() {
    let (status, body) = set_cap_response(&cap_req("10"), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("unavailable"), "body: {body}");
}

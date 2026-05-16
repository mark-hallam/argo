//! Tests for `usage_endpoint` — `GET /api/v1/tenant/usage`.

use super::show_response;
use argo_store::tenant_query::{increment_tenant_usage, tenant_tier};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ensure workspace");
    tenant
}

fn req() -> String {
    "GET /api/v1/tenant/usage HTTP/1.1\r\nHost: x\r\n\r\n".to_string()
}

async fn parse_ok(store: &Arc<SessionStore>) -> Value {
    let (status, body) = show_response(&req(), Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[tokio::test]
async fn fresh_tenant_defaults_to_free_tier_with_zero_usage() {
    let _g = env_lock().await;
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let v = parse_ok(&store).await;
    assert_eq!(v["tier"], "free");
    assert_eq!(v["display_name"], "Free");
    assert_eq!(v["used"], 0);
    assert_eq!(v["limit"], 100);
    assert_eq!(v["remaining"], 100);
    assert!(v["resets_at"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn usage_increment_is_reflected_in_response() {
    let _g = env_lock().await;
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let now = Utc::now();
    let updated = increment_tenant_usage(&store, tenant, 17, now).await.expect("increment");
    assert_eq!(updated, 17);
    let v = parse_ok(&store).await;
    assert_eq!(v["used"], 17);
    assert_eq!(v["remaining"], 100 - 17);
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let (status, body) = show_response(&req(), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("unavailable"), "body: {body}");
}

#[tokio::test]
async fn tier_defaults_to_free_when_workspace_predates_tier_column() {
    let _g = env_lock().await;
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let tier = tenant_tier(&store, tenant).await.expect("tier");
    assert_eq!(tier, "free");
}

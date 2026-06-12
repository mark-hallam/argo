//! Tests for `routes_provider_keys` (BYOK provider-key SET, DEC-053
//! Enterprise).
//!
//! Load-bearing pins mirror the brief's acceptance:
//! - **owner-only**: a guest is refused 403 and writes nothing.
//! - **tenant-scoped + cross-tenant isolation**: tenant A's key lands in
//!   A's section; tenant B never inherits it.
//! - **secret discipline (NO THEATER)**: the posted key never appears in
//!   the HTTP response nor in any audit payload.
//! - regression-safe shape errors (unknown provider / missing key → 400).
//!
//! Each test holds the shared env lock and chdirs into a tempdir so the
//! credential file (`<cwd>/.argo/credentials.toml`) is isolated.

#![allow(clippy::expect_used)]

use super::dispatch_post;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::OwnedMutexGuard;

const ANTHROPIC_ENV: &str = "ANTHROPIC_API_KEY";

struct Env {
    _guard: OwnedMutexGuard<()>,
    _tmp: TempDir,
    prev_cwd: PathBuf,
    store: Arc<SessionStore>,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev_cwd);
    }
}

async fn setup() -> Env {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    let tmp = TempDir::new().expect("tempdir");
    let prev_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(tmp.path()).expect("chdir");
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Env {
        _guard: guard,
        _tmp: tmp,
        prev_cwd,
        store: Arc::new(store),
    }
}

fn set_req(tenant: &TenantId, role: Option<&str>, body: &str) -> String {
    let mut req = String::from("POST /api/v1/providers/keys HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str(&format!("X-Argo-Tenant-Id: {tenant}\r\n"));
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

/// Read the tenant's stored key back via the same tenant-scoped store
/// the route wrote through (cwd is the tempdir workspace).
fn stored_key(tenant: &TenantId, env_key: &str) -> Option<String> {
    let ws = std::env::current_dir().expect("cwd");
    argo_config::cred::load_tenant(&ws, tenant)
        .unwrap_or_default()
        .get(env_key)
        .cloned()
}

async fn audit_payloads(store: &Arc<SessionStore>) -> Vec<String> {
    sqlx::query_scalar("SELECT payload_json FROM audit_record")
        .fetch_all(store.pool())
        .await
        .expect("audit rows")
}

// ── Owner stores a tenant-scoped key; the secret never leaks.

#[tokio::test]
async fn owner_sets_tenant_scoped_key_no_secret_leak() {
    let env = setup().await;
    let tenant = TenantId::new();
    let secret = "sk-ant-super-secret-AAA-do-not-leak";
    let body = format!(r#"{{"provider":"anthropic","key":"{secret}"}}"#);

    let (status, resp) = dispatch_post(&set_req(&tenant, Some("owner"), &body), Some(&env.store))
        .await
        .expect("provider-keys route matched");
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["provider"], "anthropic");
    assert_eq!(v["configured"], true);
    assert!(v["masked_preview"].is_null(), "never emit a secret-derived value");
    assert!(
        !resp.contains(secret),
        "the secret must never appear in the response"
    );

    // Stored under THIS tenant's section.
    assert_eq!(stored_key(&tenant, ANTHROPIC_ENV).as_deref(), Some(secret));

    // Secret never in any audit payload; a set row is written.
    let payloads = audit_payloads(&env.store).await;
    assert!(
        payloads.iter().any(|p| p.contains("provider_key.set")),
        "a provider_key.set audit row must be written"
    );
    assert!(
        payloads.iter().all(|p| !p.contains(secret)),
        "the secret must never appear in any audit payload"
    );
}

// ── Cross-tenant isolation: tenant B never inherits tenant A's key.

#[tokio::test]
async fn cross_tenant_isolation() {
    let env = setup().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let secret = "sk-ant-tenant-A-only-7c1";
    let body = format!(r#"{{"provider":"anthropic","key":"{secret}"}}"#);

    let (status, _resp) =
        dispatch_post(&set_req(&tenant_a, Some("owner"), &body), Some(&env.store))
            .await
            .expect("route matched");
    assert_eq!(status, "200 OK");

    assert_eq!(stored_key(&tenant_a, ANTHROPIC_ENV).as_deref(), Some(secret));
    assert_eq!(
        stored_key(&tenant_b, ANTHROPIC_ENV),
        None,
        "tenant B must never inherit tenant A's key"
    );
}

// ── Owner-only: a guest is refused 403 and stores nothing.

#[tokio::test]
async fn guest_is_refused() {
    let env = setup().await;
    let tenant = TenantId::new();
    let body = r#"{"provider":"anthropic","key":"sk-ant-x"}"#;

    let (status, resp) = dispatch_post(&set_req(&tenant, Some("guest"), body), Some(&env.store))
        .await
        .expect("route matched");
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(stored_key(&tenant, ANTHROPIC_ENV), None, "guest wrote nothing");
}

// ── Unknown provider → 400.

#[tokio::test]
async fn unknown_provider_is_400() {
    let env = setup().await;
    let tenant = TenantId::new();
    let body = r#"{"provider":"nonsuch","key":"sk-x"}"#;
    let (status, resp) = dispatch_post(&set_req(&tenant, Some("owner"), body), Some(&env.store))
        .await
        .expect("route matched");
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
}

// ── Missing key → 400.

#[tokio::test]
async fn missing_key_is_400() {
    let env = setup().await;
    let tenant = TenantId::new();
    let body = r#"{"provider":"anthropic"}"#;
    let (status, resp) = dispatch_post(&set_req(&tenant, Some("owner"), body), Some(&env.store))
        .await
        .expect("route matched");
    assert_eq!(status, "400 Bad Request", "body: {resp}");
}

// ── store unavailable → 503.

#[tokio::test]
async fn store_unavailable_returns_503() {
    let tenant = TenantId::new();
    let (status, _resp) = dispatch_post(
        &set_req(&tenant, Some("owner"), r#"{"provider":"anthropic","key":"x"}"#),
        None,
    )
    .await
    .expect("route matched");
    assert!(status.starts_with("503"), "got: {status}");
}

// ── A non-matching path falls through (dispatcher returns None).

#[tokio::test]
async fn non_matching_path_returns_none() {
    let env = setup().await;
    let tenant = TenantId::new();
    let req = format!(
        "POST /api/v1/providers/other HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n{{}}"
    );
    assert!(dispatch_post(&req, Some(&env.store)).await.is_none());
}

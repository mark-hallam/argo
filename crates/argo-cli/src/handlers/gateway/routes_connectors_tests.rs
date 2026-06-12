//! Tests for `routes_connectors` (tg2 — tenant-scoped connector authorize
//! over the gateway).
//!
//! The load-bearing pins mirror the brief's acceptance:
//! - **cross-tenant isolation**: tenant A authorizes a connector; tenant B
//!   reads it as `not_connected` (the credential lives in A's section only).
//! - **secret discipline (NO THEATER)**: the posted secret never appears in
//!   the HTTP response nor in any audit payload.
//! - **owner-only**: a guest is refused 403.
//! - **OAuth refused**: an OAuth connector returns 409 (interactive flow only).
//! - regression-safe shape errors (unknown connector → 404, empty body → 400).
//!
//! Each test holds the shared env lock and chdirs into a tempdir so the
//! credential file (`<cwd>/.argo/credentials.toml`) is isolated and restored.

#![allow(clippy::expect_used)]

use super::{dispatch_get, dispatch_post};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::OwnedMutexGuard;

const TOKEN_CONNECTOR: &str = "slack";
const TOKEN_KEY: &str = "SLACK_BOT_TOKEN";
const OAUTH_CONNECTOR: &str = "gmail";

struct Env {
    _guard: OwnedMutexGuard<()>,
    _tmp: TempDir,
    prev_cwd: PathBuf,
    store: Arc<SessionStore>,
}

impl Drop for Env {
    fn drop(&mut self) {
        // Restore the working directory before the env lock releases.
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

fn authorize_req(kind: &str, tenant: &TenantId, role: Option<&str>, body: &str) -> String {
    authorize_req_action(kind, "authorize", tenant, role, body)
}

fn authorize_req_action(
    kind: &str,
    action: &str,
    tenant: &TenantId,
    role: Option<&str>,
    body: &str,
) -> String {
    let mut req = format!("POST /api/v1/connectors/{kind}/{action} HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str(&format!("X-Argo-Tenant-Id: {tenant}\r\n"));
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

fn list_req(tenant: &TenantId) -> String {
    format!(
        "GET /api/v1/connectors HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\
         X-Argo-Principal: human:alice@acme\r\n\r\n"
    )
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

fn connector_status(v: &Value, kind: &str) -> String {
    v["connectors"]
        .as_array()
        .expect("connectors array")
        .iter()
        .find(|row| row["connectorId"] == kind)
        .and_then(|row| row["status"].as_str())
        .unwrap_or("absent")
        .to_string()
}

async fn audit_payloads(store: &Arc<SessionStore>) -> Vec<String> {
    sqlx::query_scalar("SELECT payload_json FROM audit_record")
        .fetch_all(store.pool())
        .await
        .expect("audit rows")
}

// ── Acceptance: authorize is tenant-scoped; a second tenant sees it unauthorized.

#[tokio::test]
async fn authorize_is_tenant_scoped_and_isolates_a_second_tenant() {
    let env = setup().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let secret = "xoxb-super-secret-AAA-do-not-leak";
    let body = format!(r#"{{"{TOKEN_KEY}":"{secret}"}}"#);

    let (status, resp) =
        dispatch_post(&authorize_req(TOKEN_CONNECTOR, &tenant_a, Some("owner"), &body), Some(&env.store))
            .await
            .expect("authorize route matched");
    assert_eq!(status, "200 OK", "body: {resp}");
    assert_eq!(parse(&resp)["status"], "connected");
    assert_eq!(parse(&resp)["credential_key"], TOKEN_KEY);

    // Tenant A sees it connected.
    let (s, listed_a) = dispatch_get(&list_req(&tenant_a), Some(&env.store))
        .await
        .expect("list route matched");
    assert_eq!(s, "200 OK");
    assert_eq!(connector_status(&parse(&listed_a), TOKEN_CONNECTOR), "connected");

    // Tenant B — same workspace, different tenant section — sees it unauthorized.
    let (_, listed_b) = dispatch_get(&list_req(&tenant_b), Some(&env.store))
        .await
        .expect("list route matched");
    assert_eq!(
        connector_status(&parse(&listed_b), TOKEN_CONNECTOR),
        "not_connected",
        "a second tenant must never inherit tenant A's credential"
    );
}

// ── Secret discipline: the token is never echoed in the response nor any audit row.

#[tokio::test]
async fn secret_never_appears_in_response_or_audit() {
    let env = setup().await;
    let tenant = TenantId::new();
    let secret = "xoxb-top-secret-must-not-leak-12345";
    let body = format!(r#"{{"{TOKEN_KEY}":"{secret}"}}"#);

    let (status, resp) =
        dispatch_post(&authorize_req(TOKEN_CONNECTOR, &tenant, Some("owner"), &body), Some(&env.store))
            .await
            .expect("authorize route matched");
    assert_eq!(status, "200 OK", "body: {resp}");
    assert!(
        !resp.contains(secret),
        "the secret must never appear in the authorize response"
    );

    let payloads = audit_payloads(&env.store).await;
    assert!(
        payloads.iter().any(|p| p.contains("connector.authorized")),
        "an authorize audit row must be written"
    );
    assert!(
        payloads.iter().all(|p| !p.contains(secret)),
        "the secret must never appear in any audit payload"
    );
}

// ── Owner-only: a guest is refused 403 and writes no credential.

#[tokio::test]
async fn guest_is_refused() {
    let env = setup().await;
    let tenant = TenantId::new();
    let body = format!(r#"{{"{TOKEN_KEY}":"x"}}"#);

    let (status, resp) =
        dispatch_post(&authorize_req(TOKEN_CONNECTOR, &tenant, Some("guest"), &body), Some(&env.store))
            .await
            .expect("authorize route matched");
    assert_eq!(status, "403 Forbidden", "body: {resp}");

    // Nothing was stored.
    let (_, listed) = dispatch_get(&list_req(&tenant), Some(&env.store))
        .await
        .expect("list route matched");
    assert_eq!(connector_status(&parse(&listed), TOKEN_CONNECTOR), "not_connected");
}

// ── OAuth connectors are refused over the credential POST (interactive only).

#[tokio::test]
async fn oauth_connector_is_refused() {
    let env = setup().await;
    let tenant = TenantId::new();
    let (status, resp) = dispatch_post(
        &authorize_req(OAUTH_CONNECTOR, &tenant, Some("owner"), r#"{"token":"x"}"#),
        Some(&env.store),
    )
    .await
    .expect("authorize route matched");
    assert_eq!(status, "409 Conflict", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "oauth_not_supported_over_http");
}

// ── Unknown connector → 404.

#[tokio::test]
async fn unknown_connector_is_404() {
    let env = setup().await;
    let tenant = TenantId::new();
    let (status, _resp) = dispatch_post(
        &authorize_req("nonsuch", &tenant, Some("owner"), r#"{"token":"x"}"#),
        Some(&env.store),
    )
    .await
    .expect("authorize route matched");
    assert_eq!(status, "404 Not Found");
}

// ── Empty / credential-less body → 400.

#[tokio::test]
async fn missing_credential_is_400() {
    let env = setup().await;
    let tenant = TenantId::new();
    let (status, resp) =
        dispatch_post(&authorize_req(TOKEN_CONNECTOR, &tenant, Some("owner"), "{}"), Some(&env.store))
            .await
            .expect("authorize route matched");
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
}

// ── The SPA's `/credentials` alias authorizes identically to `/authorize`.

#[tokio::test]
async fn credentials_alias_also_authorizes() {
    let env = setup().await;
    let tenant = TenantId::new();
    let body = format!(r#"{{"{TOKEN_KEY}":"xoxb-alias-ok"}}"#);
    let req = authorize_req_action(TOKEN_CONNECTOR, "credentials", &tenant, Some("owner"), &body);
    let (status, resp) = dispatch_post(&req, Some(&env.store))
        .await
        .expect("credentials route matched");
    assert_eq!(status, "200 OK", "body: {resp}");

    let (_, listed) = dispatch_get(&list_req(&tenant), Some(&env.store))
        .await
        .expect("list route matched");
    assert_eq!(connector_status(&parse(&listed), TOKEN_CONNECTOR), "connected");
}

// ── store unavailable → 503.

#[tokio::test]
async fn store_unavailable_returns_503() {
    let tenant = TenantId::new();
    let (status, _resp) = dispatch_post(
        &authorize_req(TOKEN_CONNECTOR, &tenant, Some("owner"), r#"{"token":"x"}"#),
        None,
    )
    .await
    .expect("authorize route matched");
    assert!(status.starts_with("503"), "got: {status}");
}

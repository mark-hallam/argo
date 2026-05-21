//! Tests for the three `web-gateway-endpoints-onboarding-batch` handlers:
//! `/api/v1/onboarding/{provider-key, install-vertical, first-hire}`.
//!
//! Test fixture mirrors `onboarding_provision_tests` — `:memory:` sqlite,
//! a tempdir-scoped `ARGO_HOME`, and the workspace `verticals/` directory
//! exposed via `ARGO_VERTICALS_DIR` so the vertical loader resolves real
//! packs. Each test pre-provisions the caller through `provision_response`
//! so the resolver returns the existing tenant + workspace path.

#![allow(clippy::too_many_lines)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::{first_hire_response, install_vertical_response, provider_key_response};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(path: &str, body: &str, principal: Option<&str>) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        head.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
    let body = json!({ "workspace_name": "Acme", "company_name": "Acme Co" }).to_string();
    let r = req(
        "/api/v1/onboarding/provision",
        &body,
        Some(&format!("human:{email}")),
    );
    let (status, body) = provision_response(&r, Some(store)).await;
    assert!(
        status.starts_with("201") || status.starts_with("200"),
        "pre-provision failed: {status} {body}"
    );
}

async fn audit_payloads_for_kind(store: &SessionStore, kind: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE kind = ? ORDER BY seq ASC")
        .bind(kind)
        .fetch_all(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn provider_key_happy_path_returns_validated() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let plaintext = "sk-ant-XXXXXXXXXXXXXXXXXXXXXXXX1234";
    let body = json!({
        "provider": "anthropic",
        "api_key": plaintext
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/provider-key",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = provider_key_response(&r, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["validated"], true);
    assert_eq!(v["provider"], "anthropic");
    assert!(
        v["masked_key"].as_str().unwrap().contains("..."),
        "masked: {}",
        v["masked_key"]
    );
    assert!(v["test_cost"].as_f64().is_some());
    assert!(v["test_tokens"].as_i64().is_some());
    let rows = audit_payloads_for_kind(&store, "onboarding.provider_key_configured").await;
    assert_eq!(rows.len(), 1, "expected one provider-key audit row");
    assert!(
        !rows[0].contains(plaintext),
        "audit payload leaked key bytes"
    );
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(payload["endpoint"], "onboarding.provider_key");
    assert_eq!(payload["provider"], "anthropic");
    assert_eq!(payload["masked_key"], v["masked_key"]);
}

#[tokio::test]
async fn provider_key_persists_encrypted_value() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let plaintext = "sk-ant-PLAINTEXT_KEY_SHOULD_NOT_LEAK_2026";
    let body = json!({ "provider": "anthropic", "api_key": plaintext }).to_string();
    let r = req(
        "/api/v1/onboarding/provider-key",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, _) = provider_key_response(&r, Some(&store)).await;
    assert_eq!(status, "200 OK");
    let row: (String, String) = sqlx::query_as(
        "SELECT masked_key, ciphertext_b64 FROM workspace_provider_key \
         WHERE provider = 'anthropic' LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("row exists");
    assert!(
        !row.1.contains(plaintext),
        "ciphertext leaked plaintext: {}",
        row.1
    );
    assert!(row.0.contains("..."), "masked: {}", row.0);
}

#[tokio::test]
async fn provider_key_rejects_missing_principal() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let body = json!({ "api_key": "sk-ant-XXXXXXXXXXXXXXXXXXXX" }).to_string();
    let r = req("/api/v1/onboarding/provider-key", &body, None);
    let (status, _) = provider_key_response(&r, Some(&store)).await;
    assert_eq!(status, "401 Unauthorized");
}

#[tokio::test]
async fn provider_key_rejects_short_key() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "provider": "anthropic", "api_key": "sk-ant-tiny" }).to_string();
    let r = req(
        "/api/v1/onboarding/provider-key",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = provider_key_response(&r, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn provider_key_412_when_not_provisioned() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    // Insert the user_account row but skip /provision so the resolver
    // is forced into the "would create" branch.
    let user = argo_identity::UserAccount::new("nobody@acme.com".into());
    argo_identity::user_store::insert_user(&store, &user)
        .await
        .unwrap();
    let body =
        json!({ "provider": "anthropic", "api_key": "sk-ant-XXXXXXXXXXXXXXXXXXXX" }).to_string();
    let r = req(
        "/api/v1/onboarding/provider-key",
        &body,
        Some("human:nobody@acme.com"),
    );
    let (status, resp) = provider_key_response(&r, Some(&store)).await;
    assert_eq!(status, "412 Precondition Failed", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "not_provisioned");
}

#[tokio::test]
async fn install_vertical_happy_path_returns_counts() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "vertical_id": "gtm-engineer" }).to_string();
    let r = req(
        "/api/v1/onboarding/install-vertical",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = install_vertical_response(&r, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["installed"], true);
    assert_eq!(v["vertical_id"], "gtm-engineer");
    let workflows = v["workflows_added"].as_i64().unwrap();
    let skills = v["skills_added"].as_i64().unwrap();
    assert!(workflows >= 1, "expected at least one workflow file");
    assert!(skills >= 1, "expected at least one skill file");
    let rows = audit_payloads_for_kind(&store, "onboarding.vertical_installed").await;
    assert_eq!(rows.len(), 1, "expected one vertical-install audit row");
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(payload["endpoint"], "onboarding.install_vertical");
    assert_eq!(payload["vertical_id"], "gtm-engineer");
    assert_eq!(payload["workflows_added"], workflows);
    assert_eq!(payload["skills_added"], skills);
}

#[tokio::test]
async fn install_vertical_rejects_unknown_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "vertical_id": "no-such-vertical" }).to_string();
    let r = req(
        "/api/v1/onboarding/install-vertical",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = install_vertical_response(&r, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
}

#[tokio::test]
async fn install_vertical_is_idempotent() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "vertical_id": "gtm-engineer" }).to_string();
    let r = req(
        "/api/v1/onboarding/install-vertical",
        &body,
        Some("human:alice@acme.com"),
    );
    for _ in 0..3 {
        let (status, _) = install_vertical_response(&r, Some(&store)).await;
        assert_eq!(status, "200 OK");
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workspace_installed_vertical WHERE vertical_id = 'gtm-engineer'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(count, 1, "install must coalesce to one row");
}

#[tokio::test]
async fn first_hire_happy_path_creates_worker_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls",
        "vertical_id": "gtm-engineer"
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let worker_id_s = v["worker_id"].as_str().expect("worker_id present");
    assert!(!worker_id_s.is_empty());
    assert_eq!(v["name"], "Mariah");
    assert_eq!(v["vertical_id"], "gtm-engineer");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE vertical_id = 'gtm-engineer'")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one worker should land");
}

#[tokio::test]
async fn first_hire_rejects_unknown_vertical() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "description": "Mark", "vertical_id": "no-such" }).to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, _) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn cross_tenant_provider_key_is_isolated() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    pre_provision(&store, "bob@globex.com").await;
    let alice_body = json!({
        "provider": "anthropic",
        "api_key": "sk-ant-ALICE_KEY_XXXXXXXXXXXXX"
    })
    .to_string();
    let bob_body = json!({
        "provider": "openai",
        "api_key": "sk-BOB_KEY_XXXXXXXXXXXXXXXXXX"
    })
    .to_string();
    let _ = provider_key_response(
        &req(
            "/api/v1/onboarding/provider-key",
            &alice_body,
            Some("human:alice@acme.com"),
        ),
        Some(&store),
    )
    .await;
    let _ = provider_key_response(
        &req(
            "/api/v1/onboarding/provider-key",
            &bob_body,
            Some("human:bob@globex.com"),
        ),
        Some(&store),
    )
    .await;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT provider, masked_key FROM workspace_provider_key ORDER BY provider")
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert_eq!(rows.len(), 2, "each tenant has its own row");
    let providers: Vec<&str> = rows.iter().map(|(p, _)| p.as_str()).collect();
    assert!(providers.contains(&"anthropic"));
    assert!(providers.contains(&"openai"));
    let tenant_count: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT hex(tenant_id)) FROM workspace_provider_key")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(tenant_count, 2, "rows must be scoped to distinct tenants");
}

#[tokio::test]
async fn end_to_end_smoke_chain_provider_key_install_first_hire() {
    // e2e smoke: walk a single user through the full 3-endpoint onboarding
    // tail. Asserts the contract every UI step depends on: rows land, the
    // worker row materialises, and the audit trail captures each step.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;

    // 1. Provider key
    let (status, _) = provider_key_response(
        &req(
            "/api/v1/onboarding/provider-key",
            &json!({
                "provider": "anthropic",
                "api_key": "sk-ant-E2E_SMOKE_KEY_XXXXXXXX"
            })
            .to_string(),
            Some("human:alice@acme.com"),
        ),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");

    // 2. Install vertical
    let (status, _) = install_vertical_response(
        &req(
            "/api/v1/onboarding/install-vertical",
            &json!({ "vertical_id": "gtm-engineer" }).to_string(),
            Some("human:alice@acme.com"),
        ),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");

    // 3. First hire
    let (status, resp) = first_hire_response(
        &req(
            "/api/v1/onboarding/first-hire",
            &json!({
                "description": "Mariah, you book outbound demo calls",
                "vertical_id": "gtm-engineer"
            })
            .to_string(),
            Some("human:alice@acme.com"),
        ),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let worker_id = v["worker_id"].as_str().expect("worker_id").to_string();

    // Audit chain has one row per step.
    let audit_rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.%' ORDER BY created_at ASC",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    let joined = audit_rows.join("|");
    assert!(joined.contains("onboarding.provider_key"));
    assert!(joined.contains("onboarding.install_vertical"));
    assert!(joined.contains("onboarding.first_hire"));

    // Worker row is real and queryable by id. `worker.id` is stored as
    // a TEXT UUID by `IdentityStore::create_worker` — same shape the
    // `/workers/<id>` GET path resolves against.
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE id = ?")
        .bind(&worker_id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(exists, 1, "first-hire worker row must be visible");
}

//! Tests for [`super`] — the `/settings/api-keys` + `/settings/providers`
//! credential-metadata read routes. Asserts the bare-array shapes the SPA
//! clients expect, that NO secret value ever appears in a response, and
//! that provider `configured` status tracks credential presence.
//!
//! Env vars are process-global; the provider-status tests serialize on a
//! local lock + reset the keys they touch (mirrors `routes_auth_tests.rs`).

use super::*;
use argo_store::SessionStore;

const PROVIDER_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "AWS_ACCESS_KEY_ID",
];

fn clear_provider_env() {
    for k in PROVIDER_KEYS {
        std::env::remove_var(k);
    }
}

async fn store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("store");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

#[tokio::test]
async fn api_keys_delegates_to_store_empty_on_fresh_tenant() {
    // The api-keys GET now delegates to the first-class `api_key` store
    // (v86) via `routes_api_keys::list_response`. On a fresh tenant the
    // honest answer is still a bare empty array — but it is a real
    // tenant-scoped DB read, not a hard-coded stub.
    let s = store().await;
    let req = "GET /api/v1/api-keys HTTP/1.1\r\nHost: x\r\n\r\n";
    let (status, body) = dispatch_get(req, Some(&s)).await.expect("api-keys dispatch");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let arr = parsed.as_array().expect("bare array");
    assert!(arr.is_empty(), "fresh tenant has no keys: {body}");
}

#[tokio::test]
async fn provider_keys_returns_status_array_with_no_secret_values() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    clear_provider_env();
    let s = store().await;
    // Explicit tenant header so the read is tenant-scoped + deterministic.
    let tenant = uuid::Uuid::now_v7();
    let req = format!(
        "GET /api/v1/providers/keys HTTP/1.1\r\nX-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    );
    let (status, body) = dispatch_get(&req, Some(&s)).await.expect("providers dispatch");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let arr = parsed.as_array().expect("bare array");
    assert_eq!(arr.len(), 5, "one row per SPA provider: {body}");
    for row in arr {
        assert!(row["provider"].is_string(), "provider name present");
        assert!(row["configured"].is_boolean(), "configured is a boolean");
        // Metadata-only contract: never a secret or secret-derived value.
        assert!(row["masked_preview"].is_null(), "masked_preview must be null");
        assert!(
            row.get("key").is_none() && row.get("value").is_none() && row.get("secret").is_none(),
            "no key/value/secret field may appear: {row}"
        );
    }
    // With every provider env cleared, all report unconfigured.
    for row in arr {
        assert_eq!(row["configured"], false, "cleared env → unconfigured: {row}");
    }
    clear_provider_env();
}

#[tokio::test]
async fn provider_keys_marks_configured_when_credential_present_without_leaking_it() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    clear_provider_env();
    let s = store().await;
    // `cred::get` reads the process env first, so this flips anthropic's
    // status to configured. The secret itself must NOT appear in the body.
    let secret = "sk-ant-super-secret-value-do-not-leak";
    std::env::set_var("ANTHROPIC_API_KEY", secret);
    let tenant = uuid::Uuid::now_v7();
    let req = format!(
        "GET /api/v1/providers/keys HTTP/1.1\r\nX-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    );
    let (status, body) = dispatch_get(&req, Some(&s)).await.expect("providers dispatch");
    assert_eq!(status, "200 OK");
    assert!(
        !body.contains(secret),
        "the secret value MUST NOT appear in the response body: {body}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let arr = parsed.as_array().expect("array");
    let anthropic = arr
        .iter()
        .find(|r| r["provider"] == "anthropic")
        .expect("anthropic row present");
    assert_eq!(anthropic["configured"], true, "configured flips on present cred");
    assert!(anthropic["masked_preview"].is_null(), "still no masked secret");
    clear_provider_env();
}

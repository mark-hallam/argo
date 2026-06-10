//! Tests for `routes_workspace_settings` (theatre item `T1`).
//!
//! Drives `dispatch_get` / `dispatch_patch` directly against a
//! `:memory:` `SessionStore` (migrated, so the v85
//! `workspace_settings` table exists). The upstream bearer-auth gate
//! lives in `routes.rs` and is not re-exercised here. A process-wide
//! mutex serialises the cwd-dependent tenant resolution so concurrent
//! tests do not race `ensure_workspace`.

#![allow(clippy::too_many_lines)]

use super::{dispatch_get, dispatch_patch, read_voice_provider, workspace_default_for};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn patch_req(path: &str, body: &Value) -> String {
    let body = body.to_string();
    format!(
        "PATCH {path} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\n\r\n{body}"
    )
}

/// A PATCH request with a truly empty body (no JSON payload).
fn patch_req_empty(path: &str) -> String {
    format!("PATCH {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse_ok(resp: Option<(&'static str, String)>) -> Value {
    let (status, body) = resp.expect("route matched");
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[tokio::test]
async fn get_workspace_returns_neutral_default_when_unseeded() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let resp = dispatch_get(&get_req("/api/v1/workspace"), Some(&store)).await;
    let v = parse_ok(resp);
    // Real-mode walk 2026-06-07: a real tenant with no saved row gets the
    // NEUTRAL default (blank name + billing) — never "SKYCOT Demo". The
    // demo-mode branding is covered by the pure branch test below. (The
    // cargo-test process has no `ARGO_DEMO_MODE`, so this exercises the real
    // path.)
    assert_eq!(v["name"], "");
    assert_eq!(v["billing_contact"], "");
    assert_eq!(v["default_vertical"], "marketing");
    assert_eq!(v["timezone"], "Australia/Melbourne");
    assert!(v.get("description").is_some());
}

#[test]
fn workspace_default_is_neutral_for_real_tenants_branded_for_demo() {
    // Pure branch test — no env-var manipulation, no race. A real tenant
    // never sees "SKYCOT Demo"; the demo showcase keeps its branding.
    let real = workspace_default_for(false);
    assert_eq!(real["name"], "");
    assert_eq!(real["billing_contact"], "");
    assert_ne!(real["name"], "SKYCOT Demo");

    let demo = workspace_default_for(true);
    assert_eq!(demo["name"], "SKYCOT Demo");
    assert_eq!(demo["billing_contact"], "owner@skycot.example");
}

#[tokio::test]
async fn get_providers_and_notifications_return_defaults() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;

    let p = parse_ok(dispatch_get(&get_req("/api/v1/providers/policy"), Some(&store)).await);
    assert_eq!(p["primary_provider"], "anthropic");
    assert_eq!(p["fallback_behavior"], "fail_closed");
    assert_eq!(p["allow_worker_overrides"], true);

    let n = parse_ok(
        dispatch_get(&get_req("/api/v1/notifications/preferences"), Some(&store)).await,
    );
    assert_eq!(n["approval_escalation_minutes"], 60);
    assert_eq!(n["daily_digest_time"], "08:30");
    assert_eq!(n["email_enabled"], true);
    assert_eq!(n["push_enabled"], false);
}

#[tokio::test]
async fn patch_then_get_round_trips_a_change() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;

    // PATCH a single field; response echoes the merged object.
    let patched = parse_ok(
        dispatch_patch(
            &patch_req("/api/v1/workspace", &json!({ "name": "Renamed WS" })),
            Some(&store),
        )
        .await,
    );
    assert_eq!(patched["name"], "Renamed WS");
    // Untouched fields keep their defaults in the merged response.
    assert_eq!(patched["timezone"], "Australia/Melbourne");

    // A fresh GET reads the persisted value back.
    let got = parse_ok(dispatch_get(&get_req("/api/v1/workspace"), Some(&store)).await);
    assert_eq!(got["name"], "Renamed WS");
    // Real-mode walk 2026-06-07: the PATCH only set `name`, so the untouched
    // `billing_contact` keeps the NEUTRAL default (blank) — the cargo-test
    // process has no `ARGO_DEMO_MODE`, so the real (un-branded) default applies,
    // never the demo "owner@skycot.example". (`workspace_default_for(false)`.)
    assert_eq!(got["billing_contact"], "");
}

#[tokio::test]
async fn patch_providers_validates_enum() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;

    // Valid enum value is accepted.
    let ok = parse_ok(
        dispatch_patch(
            &patch_req("/api/v1/providers/policy", &json!({ "primary_provider": "openai" })),
            Some(&store),
        )
        .await,
    );
    assert_eq!(ok["primary_provider"], "openai");

    // Out-of-allow-list value is rejected with 400, and nothing is
    // persisted (the prior value still reads back).
    let (status, _body) = dispatch_patch(
        &patch_req(
            "/api/v1/providers/policy",
            &json!({ "primary_provider": "not-a-provider" }),
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");

    let after = parse_ok(dispatch_get(&get_req("/api/v1/providers/policy"), Some(&store)).await);
    assert_eq!(after["primary_provider"], "openai");
}

#[tokio::test]
async fn patch_rejects_unknown_field() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let (status, body) = dispatch_patch(
        &patch_req("/api/v1/workspace", &json!({ "bogus": "x" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("unknown field"), "body: {body}");
}

#[tokio::test]
async fn patch_rejects_wrong_type() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    // email_enabled must be a bool.
    let (status, _) = dispatch_patch(
        &patch_req(
            "/api/v1/notifications/preferences",
            &json!({ "email_enabled": "yes" }),
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");

    // approval_escalation_minutes must be a non-negative int.
    let (status, _) = dispatch_patch(
        &patch_req(
            "/api/v1/notifications/preferences",
            &json!({ "approval_escalation_minutes": -5 }),
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn empty_patch_body_is_noop_returning_current() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    // Empty body — merge is a no-op, returns the full current
    // (default) object.
    let v = parse_ok(
        dispatch_patch(&patch_req_empty("/api/v1/providers/policy"), Some(&store)).await,
    );
    assert_eq!(v["primary_provider"], "anthropic");
}

#[tokio::test]
async fn non_matching_path_returns_none() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    assert!(dispatch_get(&get_req("/api/v1/workers"), Some(&store))
        .await
        .is_none());
    // The prefix must be matched exactly — a sibling path under the
    // same root does not get swallowed.
    assert!(
        dispatch_get(&get_req("/api/v1/notifications"), Some(&store))
            .await
            .is_none(),
        "/api/v1/notifications (the bell list) must not match the preferences route"
    );
}

// ---- voice group (`/api/v1/voice/settings`) -------------------------

#[tokio::test]
async fn get_voice_returns_mock_default_when_unseeded() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let v = parse_ok(dispatch_get(&get_req("/api/v1/voice/settings"), Some(&store)).await);
    assert_eq!(v["provider"], "mock");
}

#[tokio::test]
async fn patch_voice_to_openai_round_trips() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;

    let patched = parse_ok(
        dispatch_patch(
            &patch_req("/api/v1/voice/settings", &json!({ "provider": "openai" })),
            Some(&store),
        )
        .await,
    );
    assert_eq!(patched["provider"], "openai");

    // Persisted: a fresh GET reads it back.
    let got = parse_ok(dispatch_get(&get_req("/api/v1/voice/settings"), Some(&store)).await);
    assert_eq!(got["provider"], "openai");
}

#[tokio::test]
async fn patch_voice_rejects_unknown_provider() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;
    let (status, _body) = dispatch_patch(
        &patch_req("/api/v1/voice/settings", &json!({ "provider": "elevenlabs" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status, "400 Bad Request");
    // Nothing persisted: still the default.
    let after = parse_ok(dispatch_get(&get_req("/api/v1/voice/settings"), Some(&store)).await);
    assert_eq!(after["provider"], "mock");
}

#[tokio::test]
async fn read_voice_provider_reflects_persisted_choice() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let store = mem_store().await;

    // Unseeded → None (caller falls back to env), since no row is written
    // until a PATCH lands.
    assert_eq!(read_voice_provider(&store).await, None);

    // After a PATCH, the selector helper returns the stored choice.
    let _ = dispatch_patch(
        &patch_req("/api/v1/voice/settings", &json!({ "provider": "openai" })),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(read_voice_provider(&store).await, Some("openai".to_string()));
}

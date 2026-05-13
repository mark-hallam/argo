//! Tests for `routes_scim_users` — the SCIM 2.0 user-provisioning
//! endpoints under `/scim/v2/Users`
//! (`mission-scim-provisioning`, MISSION_BACKLOG Section 7 #231).
//!
//! Drives the dispatcher directly with `:memory:` SQLite stores.
//! `ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits the bearer gate inside
//! `scim_auth` so the test bodies focus on response shapes and
//! storage side-effects. Env vars are process-global, so every test
//! takes `ENV_LOCK` and resets the bypass flag at start AND end so
//! parallel test runs do not interleave.

#![allow(clippy::too_many_lines, non_snake_case)]

use super::dispatch;
use argo_store::scim_user::ScimUserStore;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

// `tokio::sync::Mutex` so guards are held safely across `.await`
// points — env vars are process-global and tests serialize on this.
static ENV_LOCK: Mutex<()> = Mutex::const_new(());
const BYPASS_KEY: &str = "ARGO_GATEWAY_AUTH_BYPASS";
const TOKEN_KEY: &str = "ARGO_SCIM_BEARER_TOKEN";

async fn lock_env() -> tokio::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().await
}

fn enable_bypass() {
    std::env::remove_var(TOKEN_KEY);
    std::env::set_var(BYPASS_KEY, "1");
}

fn reset_env() {
    std::env::remove_var(BYPASS_KEY);
    std::env::remove_var(TOKEN_KEY);
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(verb: &str, path: &str, body: &str) -> String {
    if body.is_empty() {
        format!(
            "{verb} {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:idp@example.com\r\n\r\n"
        )
    } else {
        format!(
            "{verb} {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:idp@example.com\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }
}

async fn provision(
    store: &Arc<SessionStore>,
    user_name: &str,
) -> Value {
    let body = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": user_name,
        "displayName": user_name.split('@').next().unwrap_or(user_name),
        "emails": [{"value": user_name, "primary": true}],
        "active": true,
    })
    .to_string();
    let (status, response) = dispatch(&req("POST", "/scim/v2/Users", &body), Some(store))
        .await
        .expect("scim dispatch");
    assert_eq!(status, "201 Created", "response: {response}");
    serde_json::from_str(&response).expect("parse")
}

#[tokio::test]
async fn list_users_empty_returns_zero_resources() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let (status, body) = dispatch(&req("GET", "/scim/v2/Users", ""), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["totalResults"], 0);
    assert_eq!(v["Resources"].as_array().unwrap().len(), 0);
    assert_eq!(
        v["schemas"][0],
        "urn:ietf:params:scim:api:messages:2.0:ListResponse"
    );
    reset_env();
}

#[tokio::test]
async fn create_user_201_returns_scim_envelope() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "alice@example.com").await;
    assert_eq!(
        created["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
    assert!(created["id"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(created["userName"], "alice@example.com");
    assert_eq!(created["active"], true);
    assert!(created["meta"]["location"]
        .as_str()
        .unwrap()
        .starts_with("/scim/v2/Users/"));
    reset_env();
}

#[tokio::test]
async fn create_user_400_when_userName_missing() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let body = json!({"displayName": "no name"}).to_string();
    let (status, body) = dispatch(&req("POST", "/scim/v2/Users", &body), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "400 Bad Request", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["status"], "400");
    assert_eq!(
        v["schemas"][0],
        "urn:ietf:params:scim:api:messages:2.0:Error"
    );
    reset_env();
}

#[tokio::test]
async fn create_user_409_on_duplicate_userName() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    provision(&store, "dup@example.com").await;
    // Second insert with the same userName.
    let body = json!({"userName": "dup@example.com"}).to_string();
    let (status, body) = dispatch(&req("POST", "/scim/v2/Users", &body), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "409 Conflict", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["scimType"], "uniqueness");
    reset_env();
}

#[tokio::test]
async fn get_user_returns_full_envelope() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "bob@example.com").await;
    let id = created["id"].as_str().unwrap();
    let (status, body) = dispatch(
        &req("GET", &format!("/scim/v2/Users/{id}"), ""),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], id);
    assert_eq!(v["userName"], "bob@example.com");
    reset_env();
}

#[tokio::test]
async fn get_user_404_for_unknown_id() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let missing = "00000000-0000-0000-0000-000000000000";
    let (status, _) = dispatch(
        &req("GET", &format!("/scim/v2/Users/{missing}"), ""),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "404 Not Found");
    reset_env();
}

#[tokio::test]
async fn get_user_400_for_invalid_uuid() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let (status, _) = dispatch(
        &req("GET", "/scim/v2/Users/not-a-uuid", ""),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "400 Bad Request");
    reset_env();
}

#[tokio::test]
async fn patch_user_deactivate_via_plain_body_sets_active_false() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "carol@example.com").await;
    let id = created["id"].as_str().unwrap();
    let body = json!({"active": false}).to_string();
    let (status, body) = dispatch(
        &req("PATCH", &format!("/scim/v2/Users/{id}"), &body),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["active"], false);
    reset_env();
}

#[tokio::test]
async fn patch_user_accepts_patchop_envelope() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "dave@example.com").await;
    let id = created["id"].as_str().unwrap();
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [
            { "op": "replace", "path": "active", "value": false },
            { "op": "replace", "path": "displayName", "value": "Dave 2" },
        ]
    })
    .to_string();
    let (status, body) = dispatch(
        &req("PATCH", &format!("/scim/v2/Users/{id}"), &body),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["active"], false);
    assert_eq!(v["displayName"], "Dave 2");
    reset_env();
}

#[tokio::test]
async fn delete_user_returns_204_and_writes_tombstone() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "erin@example.com").await;
    let id = created["id"].as_str().unwrap();
    let (status, _) = dispatch(
        &req("DELETE", &format!("/scim/v2/Users/{id}"), ""),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "204 No Content");
    // GET after DELETE returns 404 (live rows only).
    let (status, _) = dispatch(
        &req("GET", &format!("/scim/v2/Users/{id}"), ""),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "404 Not Found");
    // Tombstone row inserted with the same id.
    let parsed = uuid::Uuid::parse_str(id).unwrap();
    let scim = ScimUserStore::new(store.pool());
    let tombstone = scim.tombstone(parsed).await.unwrap();
    assert!(tombstone.is_some(), "tombstone row should exist");
    let row = tombstone.unwrap();
    assert_eq!(row.user_name, "erin@example.com");
    assert_eq!(row.reason, "scim_delete");
    reset_env();
}

#[tokio::test]
async fn delete_already_deleted_user_returns_404() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    let created = provision(&store, "frank@example.com").await;
    let id = created["id"].as_str().unwrap();
    let path = format!("/scim/v2/Users/{id}");
    let _ = dispatch(&req("DELETE", &path, ""), Some(&store))
        .await
        .expect("dispatch");
    let (status, _) = dispatch(&req("DELETE", &path, ""), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "404 Not Found");
    reset_env();
}

#[tokio::test]
async fn list_with_user_name_filter_matches_single_row() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    provision(&store, "gina@example.com").await;
    provision(&store, "henry@example.com").await;
    let path = "/scim/v2/Users?filter=userName+eq+%22gina%40example.com%22";
    let (status, body) = dispatch(&req("GET", path, ""), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["totalResults"], 1);
    assert_eq!(v["Resources"][0]["userName"], "gina@example.com");
    reset_env();
}

#[tokio::test]
async fn scim_auth_503_when_token_unset_and_no_bypass() {
    let _g = lock_env().await;
    reset_env();
    let store = mem_store().await;
    let (status, body) = dispatch(&req("GET", "/scim/v2/Users", ""), Some(&store))
        .await
        .expect("dispatch");
    assert_eq!(status, "503 Service Unavailable", "body: {body}");
    reset_env();
}

// ----- INTEGRATION TEST --------------------------------------------

/// End-to-end: provision → deactivate → reactivate
/// (re-activation by PATCH active=true is the lifecycle the brief
/// calls out as the integration scenario). Confirms the row is
/// preserved across the three operations and that `active` flips
/// each time.
#[tokio::test]
async fn integration_provision_deactivate_reactivate_lifecycle() {
    let _g = lock_env().await;
    enable_bypass();
    let store = mem_store().await;
    // 1. provision
    let created = provision(&store, "lifecycle@example.com").await;
    let id = created["id"].as_str().unwrap();
    let path = format!("/scim/v2/Users/{id}");
    // 2. deactivate
    let (status, body) = dispatch(
        &req("PATCH", &path, &json!({"active": false}).to_string()),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "200 OK");
    let after_deact: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(after_deact["active"], false);
    // 3. reactivate
    let (status, body) = dispatch(
        &req("PATCH", &path, &json!({"active": true}).to_string()),
        Some(&store),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, "200 OK");
    let after_react: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(after_react["active"], true);
    assert_eq!(after_react["id"], id);
    reset_env();
}

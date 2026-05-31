//! Tests for `routes_api_keys` — the first-class `/settings/api-keys`
//! surface (Frontier-B SPEC 2): list / create / revoke.
//!
//! Drives the handlers directly against `:memory:` SQLite stores with an
//! explicit `X-Argo-Tenant-Id` header so each test is tenant-deterministic.
//! Pins: the bare-array list shape, create returning the full token
//! EXACTLY ONCE (and the token NEVER reappearing on a list), the token /
//! hash never landing in a stored row read back through the list, revoke
//! flipping the flag, and the load-bearing CROSS-TENANT NEGATIVE — a
//! revoke scoped to another tenant must fail (404) and leave the key live.

#![allow(clippy::too_many_lines)]

use super::{create_response, list_response, revoke_response};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

const T1: &str = "00000000-0000-0000-0000-0000000000a1";
const T2: &str = "00000000-0000-0000-0000-0000000000a2";

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn get_req(tenant: &str) -> String {
    format!(
        "GET /api/v1/api-keys HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n"
    )
}

fn post_req(tenant: &str, body: &str) -> String {
    format!(
        "POST /api/v1/api-keys HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nX-Argo-Tenant-Id: {tenant}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn delete_req(tenant: &str, id: &str) -> String {
    format!(
        "DELETE /api/v1/api-keys/{id} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n"
    )
}

fn create_body(name: &str) -> String {
    json!({
        "name": name,
        "scopes": ["read:workers", "read:audit"],
        "cohort_ids": [],
        "all_cohorts": true,
        "expiry": "never",
    })
    .to_string()
}

async fn create_key(store: &Arc<SessionStore>, tenant: &str, name: &str) -> (String, String) {
    let (status, body) = create_response(&post_req(tenant, &create_body(name)), Some(store)).await;
    assert_eq!(status, "201 Created", "create body: {body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let id = parsed["key"]["id"].as_str().expect("key id").to_string();
    let token = parsed["token"].as_str().expect("token").to_string();
    (id, token)
}

#[tokio::test]
async fn list_is_empty_on_fresh_store() {
    let s = mem_store().await;
    let (status, body) = list_response(&get_req(T1), Some(&s)).await;
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert!(parsed.as_array().expect("array").is_empty(), "{body}");
}

#[tokio::test]
async fn create_returns_full_token_exactly_once_and_persists_metadata() {
    let s = mem_store().await;
    let (status, body) =
        create_response(&post_req(T1, &create_body("Production CI")), Some(&s)).await;
    assert_eq!(status, "201 Created", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");

    // Token is the full bearer, shown here once.
    let token = parsed["token"].as_str().expect("token present");
    assert!(token.starts_with("sk_live_"), "token shape: {token}");

    // The embedded key row carries metadata only — masked prefix, no token.
    let key = &parsed["key"];
    assert_eq!(key["name"], "Production CI");
    assert_eq!(key["all_cohorts"], true);
    assert_eq!(key["revoked"], false);
    assert!(key["prefix"].as_str().unwrap().contains("..."), "masked prefix");
    assert!(key.get("token").is_none(), "key row must not carry the token");
    assert!(key.get("token_hash").is_none(), "key row must not carry the hash");
    // The masked prefix is NOT the full token.
    assert_ne!(key["prefix"].as_str().unwrap(), token);
}

#[tokio::test]
async fn list_after_create_never_contains_the_token() {
    let s = mem_store().await;
    let (_id, token) = create_key(&s, T1, "Staging dashboard").await;
    let (status, body) = list_response(&get_req(T1), Some(&s)).await;
    assert_eq!(status, "200 OK");
    assert!(
        !body.contains(&token),
        "the full token MUST NOT appear in the list response: {body}"
    );
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "Staging dashboard");
}

#[tokio::test]
async fn create_rejects_unknown_scope() {
    let s = mem_store().await;
    let body = json!({
        "name": "Bad", "scopes": ["read:everything"], "cohort_ids": [],
        "all_cohorts": false, "expiry": "never"
    })
    .to_string();
    let (status, _b) = create_response(&post_req(T1, &body), Some(&s)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn create_rejects_blank_name() {
    let s = mem_store().await;
    let body = json!({
        "name": "   ", "scopes": [], "cohort_ids": [], "all_cohorts": true, "expiry": "never"
    })
    .to_string();
    let (status, _b) = create_response(&post_req(T1, &body), Some(&s)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn revoke_flips_the_flag_and_returns_ok_envelope() {
    let s = mem_store().await;
    let (id, _token) = create_key(&s, T1, "To be revoked").await;
    let (status, body) = revoke_response(&id, &delete_req(T1, &id), Some(&s)).await;
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["id"], id);

    // The key now reads revoked in the list.
    let (_st, list_body) = list_response(&get_req(T1), Some(&s)).await;
    let listed: Value = serde_json::from_str(&list_body).expect("json");
    assert_eq!(listed.as_array().unwrap()[0]["revoked"], true);
}

#[tokio::test]
async fn revoke_unknown_id_is_404() {
    let s = mem_store().await;
    let (status, _b) = revoke_response("key_ghost", &delete_req(T1, "key_ghost"), Some(&s)).await;
    assert_eq!(status, "404 Not Found");
}

/// THE CROSS-TENANT NEGATIVE: a key created under tenant T1 must not be
/// revocable by a request scoped to tenant T2. The foreign revoke 404s
/// and the key stays live for its owning tenant.
#[tokio::test]
async fn cross_tenant_revoke_is_forbidden_and_leaves_key_live() {
    let s = mem_store().await;
    let (id, _token) = create_key(&s, T1, "Owned by T1").await;

    // T2 attempts to revoke T1's key by id → 404, no row touched.
    let (status, _b) = revoke_response(&id, &delete_req(T2, &id), Some(&s)).await;
    assert_eq!(status, "404 Not Found", "cross-tenant revoke must 404");

    // T1's key is still live.
    let (_st, list_body) = list_response(&get_req(T1), Some(&s)).await;
    let listed: Value = serde_json::from_str(&list_body).expect("json");
    let arr = listed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], id);
    assert_eq!(arr[0]["revoked"], false, "key must remain live after foreign revoke");

    // And T2 cannot even see it in its own list.
    let (_st2, t2_body) = list_response(&get_req(T2), Some(&s)).await;
    let t2_listed: Value = serde_json::from_str(&t2_body).expect("json");
    assert!(t2_listed.as_array().unwrap().is_empty(), "T2 sees no T1 keys");
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let (status, _b) = list_response(&get_req(T1), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let (status, _b) =
        create_response(&post_req(T1, &create_body("x")), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

#[tokio::test]
async fn dispatch_post_and_delete_route_correctly() {
    let s = mem_store().await;
    // POST routes to create.
    let (status, body) =
        super::dispatch_post(&post_req(T1, &create_body("Routed")), Some(&s))
            .await
            .expect("post dispatched");
    assert_eq!(status, "201 Created", "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["key"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // DELETE routes to revoke.
    let (status, _b) = super::dispatch_delete(&delete_req(T1, &id), Some(&s))
        .await
        .expect("delete dispatched");
    assert_eq!(status, "200 OK");
    // A non-api-keys DELETE returns None (falls through to other arms).
    assert!(
        super::dispatch_delete(
            "DELETE /api/v1/workers/abc HTTP/1.1\r\nHost: x\r\n\r\n",
            Some(&s)
        )
        .await
        .is_none()
    );
}

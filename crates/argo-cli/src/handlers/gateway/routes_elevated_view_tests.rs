//! Tests for `routes_elevated_view` — Stream J16's three handlers
//! (`POST /api/v1/audit/elevated-view-request`,
//! `GET /api/v1/audit/elevated-view-grants`,
//! `DELETE /api/v1/audit/elevated-view-grants/<id>`).
//!
//! Drives the handlers directly with `:memory:` SQLite stores. The
//! bearer-auth gate is exercised at the dispatch layer; these tests
//! focus on the handler body shapes, validation, and the audit-row
//! side-effects.

#![allow(clippy::too_many_lines)]

use super::{list_response, request_response, revoke_response};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn post_req(body: &str) -> String {
    format!(
        "POST /api/v1/audit/elevated-view-request HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn post_req_for_principal(body: &str, principal: &str) -> String {
    format!(
        "POST /api/v1/audit/elevated-view-request HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: {principal}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn get_req(query: &str) -> String {
    let q = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    format!(
        "GET /api/v1/audit/elevated-view-grants{q} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\n\r\n"
    )
}

fn delete_req(grant_id: &str, principal: &str) -> String {
    format!(
        "DELETE /api/v1/audit/elevated-view-grants/{grant_id} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: {principal}\r\n\r\n"
    )
}

fn valid_body() -> String {
    json!({
        "reason": "Customer escalation requires raw email lookup",
        "categories": ["email", "phone"],
        "duration_hours": 2,
    })
    .to_string()
}

#[tokio::test]
async fn post_happy_path_returns_201_with_grant_id_and_expires_at() {
    let store = mem_store().await;
    let (status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["grant_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(v["expires_at"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(v["auto_granted"], true);
    assert!(v["note"].as_str().unwrap().contains("v1 auto-grants"));
}

#[tokio::test]
async fn post_rejects_short_reason_400() {
    let store = mem_store().await;
    let body = json!({
        "reason": "too short",
        "categories": ["email"],
        "duration_hours": 1,
    })
    .to_string();
    let (status, body) = request_response(&post_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
    assert!(v["error"]["message"].as_str().unwrap().contains("reason"));
}

#[tokio::test]
async fn post_rejects_unknown_category_400() {
    let store = mem_store().await;
    let body = json!({
        "reason": "Investigating cross-channel alignment for one customer",
        "categories": ["email", "ssn"],
        "duration_hours": 1,
    })
    .to_string();
    let (status, body) = request_response(&post_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["error"]["message"].as_str().unwrap().contains("ssn"));
}

#[tokio::test]
async fn post_rejects_duration_out_of_range_400() {
    let store = mem_store().await;
    for hours in [0u32, 25, 99] {
        let body = json!({
            "reason": "Investigating cross-channel alignment for one customer",
            "categories": ["email"],
            "duration_hours": hours,
        })
        .to_string();
        let (status, body) = request_response(&post_req(&body), Some(&store)).await;
        assert_eq!(status, "400 Bad Request", "{hours}: {body}");
    }
}

#[tokio::test]
async fn post_accepts_forward_compat_categories() {
    let store = mem_store().await;
    let body = json!({
        "reason": "Investigating cross-channel alignment for one customer",
        "categories": ["classification", "approver", "counterparty"],
        "duration_hours": 1,
    })
    .to_string();
    let (status, _body) = request_response(&post_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");
}

#[tokio::test]
async fn post_persists_row_visible_via_get_active() {
    let store = mem_store().await;
    let (status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    assert_eq!(status, "201 Created");
    let v: Value = serde_json::from_str(&body).unwrap();
    let grant_id = v["grant_id"].as_str().unwrap().to_string();

    let (status, body) = list_response(&get_req("active=true"), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    let grants = v["grants"].as_array().unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0]["grant_id"], grant_id);
    assert_eq!(grants[0]["active"], true);
    assert_eq!(
        grants[0]["categories"].as_array().unwrap().len(),
        2,
        "email + phone"
    );
}

#[tokio::test]
async fn get_default_filters_to_active_only() {
    let store = mem_store().await;
    request_response(&post_req(&valid_body()), Some(&store))
        .await
        .0
        .starts_with("201");
    // Default query (no ?active param) → active-only.
    let (status, body) = list_response(&get_req(""), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["active_only"], true);
    assert_eq!(v["grants"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_active_false_returns_revoked_grant() {
    let store = mem_store().await;
    let (_status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    let grant_id = serde_json::from_str::<Value>(&body).unwrap()["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, _body) = revoke_response(
        &grant_id,
        &delete_req(&grant_id, "human:mark@example.com"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "204 No Content");

    // active=true → empty.
    let (_status, body) = list_response(&get_req("active=true"), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["grants"].as_array().unwrap().len(), 0);
    // active=false → the revoked row is visible.
    let (_status, body) = list_response(&get_req("active=false"), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let grants = v["grants"].as_array().unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0]["grant_id"], grant_id);
    assert_eq!(grants[0]["active"], false);
    assert!(grants[0]["revoked_at"].as_str().is_some());
}

#[tokio::test]
async fn get_isolates_per_principal() {
    let store = mem_store().await;
    request_response(&post_req(&valid_body()), Some(&store)).await; // mark
    request_response(
        &post_req_for_principal(&valid_body(), "human:dana@example.com"),
        Some(&store),
    )
    .await; // dana
    let (_status, body) = list_response(&get_req(""), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let grants = v["grants"].as_array().unwrap();
    assert_eq!(grants.len(), 1, "mark only sees own grant");
    assert_eq!(grants[0]["principal_id"], "human:mark@example.com");
}

#[tokio::test]
async fn delete_self_revoke_returns_204_and_idempotent() {
    let store = mem_store().await;
    let (_status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    let grant_id = serde_json::from_str::<Value>(&body).unwrap()["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    let req = delete_req(&grant_id, "human:mark@example.com");
    let (status1, body1) = revoke_response(&grant_id, &req, Some(&store)).await;
    assert_eq!(status1, "204 No Content");
    assert!(body1.is_empty());
    // Second revoke is idempotent — same 204.
    let (status2, _body2) = revoke_response(&grant_id, &req, Some(&store)).await;
    assert_eq!(status2, "204 No Content");
}

#[tokio::test]
async fn delete_operator_revoke_in_same_tenant_204() {
    let store = mem_store().await;
    // Grant owned by mark.
    let (_status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    let grant_id = serde_json::from_str::<Value>(&body).unwrap()["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    // Dana (same workspace/tenant) revokes it.
    let req = delete_req(&grant_id, "human:dana@example.com");
    let (status, _body) = revoke_response(&grant_id, &req, Some(&store)).await;
    assert_eq!(status, "204 No Content");
}

#[tokio::test]
async fn delete_unknown_grant_id_404() {
    let store = mem_store().await;
    let phony = "018f0000-0000-7000-8000-000000000000";
    let req = delete_req(phony, "human:mark@example.com");
    let (status, body) = revoke_response(phony, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "grant_not_found");
}

#[tokio::test]
async fn delete_invalid_uuid_400() {
    let store = mem_store().await;
    let req = delete_req("not-a-uuid", "human:mark@example.com");
    let (status, body) = revoke_response("not-a-uuid", &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn post_emits_requested_and_granted_audit_rows() {
    let store = mem_store().await;
    let (_status, _body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("query audit");
    let event_types: Vec<String> = rows
        .iter()
        .filter_map(|(payload,)| {
            serde_json::from_str::<Value>(payload)
                .ok()
                .and_then(|v| v["event_type"].as_str().map(str::to_string))
        })
        .collect();
    assert!(
        event_types.contains(&"elevated_view.requested".to_string()),
        "events: {event_types:?}"
    );
    assert!(
        event_types.contains(&"elevated_view.granted".to_string()),
        "events: {event_types:?}"
    );
}

#[tokio::test]
async fn delete_emits_revoked_audit_row() {
    let store = mem_store().await;
    let (_status, body) = request_response(&post_req(&valid_body()), Some(&store)).await;
    let grant_id = serde_json::from_str::<Value>(&body).unwrap()["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    let req = delete_req(&grant_id, "human:mark@example.com");
    revoke_response(&grant_id, &req, Some(&store)).await;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_all(store.pool())
    .await
    .expect("query audit");
    let (payload,) = rows.first().expect("at least one audit row");
    let v: Value = serde_json::from_str(payload).unwrap();
    assert_eq!(v["event_type"], "elevated_view.revoked");
    assert_eq!(v["grant_id"], grant_id);
    assert_eq!(v["self_revoke"], true);
}

#[tokio::test]
async fn post_store_unavailable_503() {
    let (status, body) = request_response(&post_req(&valid_body()), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["error"].as_str().unwrap().contains("unavailable"));
}

#[tokio::test]
async fn post_rejects_malformed_json_400() {
    let store = mem_store().await;
    let (status, body) = request_response(&post_req("not json"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

//! Tests for `routes_support_tickets` — Wave 4 P2 inbox handlers.
//!
//! Drives the handlers directly with an in-memory `SessionStore`. The
//! bearer-auth gate is exercised at the dispatch layer; these tests
//! focus on body shapes, admin gating, and validation.

#![allow(clippy::too_many_lines)]

use super::{
    create_response, detail_response, list_response, reply_response, update_response,
};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn admin_post(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Role: admin\r\nX-Argo-Principal: human:op@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn anon_post(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn admin_get(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Role: admin\r\nX-Argo-Principal: human:op@example.com\r\n\r\n"
    )
}

fn non_admin_get(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Role: auditor\r\nX-Argo-Principal: human:auditor@example.com\r\n\r\n"
    )
}

fn admin_patch(path: &str, body: &str) -> String {
    format!(
        "PATCH {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Role: admin\r\nX-Argo-Principal: human:op@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn seed_ticket(store: &Arc<SessionStore>, subject: &str) -> String {
    let body = json!({
        "user_email": "alice@example.com",
        "subject": subject,
        "body": "the magic link bounces with 500",
    })
    .to_string();
    let (status, resp) =
        create_response(&anon_post("/api/v1/support/tickets", &body), Some(store)).await;
    assert_eq!(status, "201 Created", "{resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    v["ticket_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn create_happy_path_returns_ticket_id() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "cannot log in").await;
    assert_eq!(id.len(), 36);
}

#[tokio::test]
async fn create_rejects_missing_email() {
    let store = mem_store().await;
    let body = json!({ "subject": "x", "body": "x" }).to_string();
    let (status, resp) =
        create_response(&anon_post("/api/v1/support/tickets", &body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("user_email"));
}

#[tokio::test]
async fn list_requires_admin() {
    let store = mem_store().await;
    seed_ticket(&store, "x").await;
    let (status, resp) = list_response(&non_admin_get("/api/v1/support/tickets"), Some(&store))
        .await;
    assert_eq!(status, "403 Forbidden", "{resp}");
}

#[tokio::test]
async fn list_admin_returns_tickets() {
    let store = mem_store().await;
    seed_ticket(&store, "one").await;
    seed_ticket(&store, "two").await;
    let (status, resp) =
        list_response(&admin_get("/api/v1/support/tickets?status=open&limit=10"), Some(&store))
            .await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["tickets"].as_array().unwrap().len(), 2);
    let subjects: Vec<&str> = v["tickets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["subject"].as_str().unwrap())
        .collect();
    assert!(subjects.contains(&"one"));
    assert!(subjects.contains(&"two"));
}

#[tokio::test]
async fn detail_returns_ticket_and_empty_thread() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "subj").await;
    let (status, resp) = detail_response(
        &id,
        &admin_get(&format!("/api/v1/support/tickets/{id}")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "{resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["ticket"]["id"], id);
    assert_eq!(v["replies"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn detail_404_for_unknown_id() {
    let store = mem_store().await;
    let id = uuid::Uuid::now_v7().to_string();
    let (status, _) = detail_response(
        &id,
        &admin_get(&format!("/api/v1/support/tickets/{id}")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn detail_400_for_invalid_uuid() {
    let store = mem_store().await;
    let id = "not-a-uuid";
    let (status, _) = detail_response(
        id,
        &admin_get(&format!("/api/v1/support/tickets/{id}")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn reply_persists_and_shows_in_thread() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "subj").await;
    let body = json!({ "body": "we're investigating" }).to_string();
    let (status, _) = reply_response(
        &id,
        &admin_post(&format!("/api/v1/support/tickets/{id}/reply"), &body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created");

    let (_, detail) = detail_response(
        &id,
        &admin_get(&format!("/api/v1/support/tickets/{id}")),
        Some(&store),
    )
    .await;
    let v: Value = serde_json::from_str(&detail).unwrap();
    let replies = v["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["author_kind"], "operator");
    assert_eq!(replies[0]["body"], "we're investigating");
}

#[tokio::test]
async fn reply_requires_admin() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "subj").await;
    let body = json!({ "body": "x" }).to_string();
    let (status, _) = reply_response(
        &id,
        &format!(
            "POST /api/v1/support/tickets/{id}/reply HTTP/1.1\r\nHost: x\r\nX-Argo-Role: auditor\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
        Some(&store),
    )
    .await;
    assert_eq!(status, "403 Forbidden");
}

#[tokio::test]
async fn update_changes_status() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "subj").await;
    let body = json!({ "status": "resolved" }).to_string();
    let (status, resp) = update_response(
        &id,
        &admin_patch(&format!("/api/v1/support/tickets/{id}"), &body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "{resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["ticket"]["status"], "resolved");
}

#[tokio::test]
async fn update_rejects_invalid_status() {
    let store = mem_store().await;
    let id = seed_ticket(&store, "subj").await;
    let body = json!({ "status": "wat" }).to_string();
    let (status, _) = update_response(
        &id,
        &admin_patch(&format!("/api/v1/support/tickets/{id}"), &body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn update_404_for_missing_ticket() {
    let store = mem_store().await;
    let id = uuid::Uuid::now_v7().to_string();
    let body = json!({ "status": "closed" }).to_string();
    let (status, _) = update_response(
        &id,
        &admin_patch(&format!("/api/v1/support/tickets/{id}"), &body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn create_503_when_store_unavailable() {
    let body = json!({ "user_email": "a@example.com", "subject": "x", "body": "x" }).to_string();
    let (status, _) =
        create_response(&anon_post("/api/v1/support/tickets", &body), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

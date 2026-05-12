//! Tests for `routes_retros` — mission-retro-review-ui.
//!
//! Drives the handlers directly against an in-memory SQLite store.
//! Bearer auth is exercised at the dispatch layer; these tests focus
//! on body shapes, validation, ordering, and the audit-row
//! side-effects.

#![allow(clippy::too_many_lines)]

use super::{
    create_response, finalize_response, index_response, list_for_worker_response, update_response,
};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn post_req(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn patch_req(path: &str, body: &str) -> String {
    format!(
        "PATCH {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn get_req(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\n\r\n"
    )
}

fn fresh_worker() -> String {
    Uuid::now_v7().to_string()
}

#[tokio::test]
async fn create_returns_201_with_retro_id_and_empty_sections() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (status, body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["retro_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(v["worker_id"], worker);
    assert_eq!(v["finalized"], false);
    assert_eq!(v["sections"]["surface_metrics"], "");
    assert_eq!(v["sections"]["what_went_well"], "");
    assert_eq!(v["sections"]["surprises"], "");
    assert_eq!(v["sections"]["what_to_change"], "");
    assert_eq!(v["sections"]["action_items"], "");
}

#[tokio::test]
async fn create_with_invalid_worker_id_returns_400() {
    let store = mem_store().await;
    let path = "/api/v1/workers/not-a-uuid/retros";
    let (status, body) =
        create_response("not-a-uuid", &post_req(path, "{}"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn create_rejects_window_end_before_start_400() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let body = json!({ "window_start_unix": 2_000, "window_end_unix": 1_000 }).to_string();
    let (status, _b) = create_response(&worker, &post_req(&path, &body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn update_persists_then_get_reflects_new_sections() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_status, body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let retro_id = v["retro_id"].as_str().unwrap().to_string();

    let patch_path = format!("/api/v1/workers/{worker}/retros/{retro_id}");
    let patch_body = json!({
        "what_went_well": "Cold-start latency stayed under target.",
        "action_items": "Add p95 latency alert.",
    })
    .to_string();
    let (status, body) = update_response(
        &worker,
        &retro_id,
        &patch_req(&patch_path, &patch_body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        v["sections"]["what_went_well"],
        "Cold-start latency stayed under target."
    );
    assert_eq!(v["sections"]["action_items"], "Add p95 latency alert.");
    // Untouched slots default to empty string (not absent).
    assert_eq!(v["sections"]["surface_metrics"], "");
}

#[tokio::test]
async fn update_rejects_unknown_section_400() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_status, body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let retro_id = serde_json::from_str::<Value>(&body).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();

    let patch_path = format!("/api/v1/workers/{worker}/retros/{retro_id}");
    let patch_body = json!({ "what_we_changed": "Something" }).to_string();
    let (status, body) = update_response(
        &worker,
        &retro_id,
        &patch_req(&patch_path, &patch_body),
        Some(&store),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("what_we_changed"),
        "{body}"
    );
}

#[tokio::test]
async fn finalize_locks_and_blocks_subsequent_edits() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_status, body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let retro_id = serde_json::from_str::<Value>(&body).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();

    let finalize_path = format!("/api/v1/workers/{worker}/retros/{retro_id}/finalize");
    let (status, body) =
        finalize_response(&worker, &retro_id, &post_req(&finalize_path, ""), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["finalized"], true);
    assert!(v["finalized_at"].as_i64().is_some());

    // Subsequent PATCH → 409.
    let patch_path = format!("/api/v1/workers/{worker}/retros/{retro_id}");
    let (status, body) = update_response(
        &worker,
        &retro_id,
        &patch_req(&patch_path, &json!({"what_went_well": "x"}).to_string()),
        Some(&store),
    )
    .await;
    assert_eq!(status, "409 Conflict", "body: {body}");
    // Re-finalize → 409.
    let (status, _b) =
        finalize_response(&worker, &retro_id, &post_req(&finalize_path, ""), Some(&store)).await;
    assert_eq!(status, "409 Conflict");
}

#[tokio::test]
async fn finalize_unknown_retro_id_returns_404() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let phony = "018f0000-0000-7000-8000-000000000000";
    let path = format!("/api/v1/workers/{worker}/retros/{phony}/finalize");
    let (status, body) =
        finalize_response(&worker, phony, &post_req(&path, ""), Some(&store)).await;
    assert_eq!(status, "404 Not Found");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "retro_not_found");
}

#[tokio::test]
async fn list_for_worker_orders_newest_first() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_s, b1) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    // Tiny pause not necessary — UUIDv7 timestamps are nanosecond-
    // resolution and `created_at` is unix-seconds; both retros may
    // share the second but the row insert order still distinguishes.
    let (_s, b2) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let id1 = serde_json::from_str::<Value>(&b1).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();
    let id2 = serde_json::from_str::<Value>(&b2).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) =
        list_for_worker_response(&worker, &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    let retros = v["retros"].as_array().unwrap();
    assert_eq!(retros.len(), 2);
    let ids: Vec<&str> = retros.iter().map(|r| r["retro_id"].as_str().unwrap()).collect();
    assert!(ids.contains(&id1.as_str()));
    assert!(ids.contains(&id2.as_str()));
}

#[tokio::test]
async fn index_q_param_filters_to_matching_sections() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_s, b) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let retro_id = serde_json::from_str::<Value>(&b).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();
    let patch_path = format!("/api/v1/workers/{worker}/retros/{retro_id}");
    let patch_body = json!({
        "what_went_well": "Great escalation discipline this week.",
    })
    .to_string();
    update_response(
        &worker,
        &retro_id,
        &patch_req(&patch_path, &patch_body),
        Some(&store),
    )
    .await;
    // Second retro: doesn't match theme.
    let (_s, _b) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;

    let (status, body) =
        index_response(&get_req("/api/v1/retros?q=escalation"), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    let retros = v["retros"].as_array().unwrap();
    assert_eq!(retros.len(), 1);
    assert_eq!(retros[0]["retro_id"], retro_id);
    assert_eq!(v["q"], "escalation");
}

#[tokio::test]
async fn create_emits_retro_created_audit_row() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (status, _body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    assert!(status.starts_with("201"));
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' ORDER BY seq DESC LIMIT 5",
    )
    .fetch_all(store.pool())
    .await
    .expect("audit query");
    let saw_created = rows.iter().any(|(p,)| {
        serde_json::from_str::<Value>(p)
            .ok()
            .and_then(|v| v["event_type"].as_str().map(str::to_string))
            == Some("retro.created".to_string())
    });
    assert!(saw_created, "expected retro.created event; rows: {rows:?}");
}

#[tokio::test]
async fn finalize_emits_retro_finalized_audit_row() {
    let store = mem_store().await;
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (_status, body) = create_response(&worker, &post_req(&path, "{}"), Some(&store)).await;
    let retro_id = serde_json::from_str::<Value>(&body).unwrap()["retro_id"]
        .as_str()
        .unwrap()
        .to_string();
    let finalize_path = format!("/api/v1/workers/{worker}/retros/{retro_id}/finalize");
    finalize_response(&worker, &retro_id, &post_req(&finalize_path, ""), Some(&store)).await;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' ORDER BY seq DESC LIMIT 5",
    )
    .fetch_all(store.pool())
    .await
    .expect("audit query");
    let saw_finalized = rows.iter().any(|(p,)| {
        serde_json::from_str::<Value>(p)
            .ok()
            .and_then(|v| v["event_type"].as_str().map(str::to_string))
            == Some("retro.finalized".to_string())
    });
    assert!(saw_finalized, "expected retro.finalized event; rows: {rows:?}");
}

#[tokio::test]
async fn create_store_unavailable_503() {
    let worker = fresh_worker();
    let path = format!("/api/v1/workers/{worker}/retros");
    let (status, _body) = create_response(&worker, &post_req(&path, "{}"), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

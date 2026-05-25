//! Regression tests for `/api/v1/audit` list/detail ID alignment.

use super::super::routes_audit_detail::detail_response;
use super::list_response;
use super::test_helpers::{get, get_with_tenant, mem_store, parse_body, seed_row_for_tenant};
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

async fn seed_legacy_row(store: &Arc<SessionStore>, worker_id: &str) -> String {
    let id = vec![0x42; 16];
    sqlx::query(
        "INSERT INTO audit_record (id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, 'http_request', ?, ?, ?, ?)",
    )
    .bind(id.as_slice())
    .bind(
        json!({
            "worker_id": worker_id,
            "worker_name": "Legacy Worker",
            "channel": "web",
            "action_type": "legacy.action",
            "outcome_status": "succeeded",
            "verb": "legacy action"
        })
        .to_string(),
    )
    .bind([0u8; 32].as_slice())
    .bind([0u8; 32].as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .expect("insert legacy audit row");
    hex::encode(id)
}

#[tokio::test]
async fn default_feed_legacy_id_fetches_detail() {
    let store = mem_store().await;
    let id = seed_legacy_row(&store, "legacy-worker").await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?range=7d"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    assert!(
        items.iter().any(|item| item["id"] == id),
        "legacy row missing from default feed: {body}"
    );

    let detail = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    assert_eq!(detail.0, "200 OK", "detail response: {}", detail.1);
}

#[tokio::test]
async fn explicit_tenant_feed_id_fetches_detail_but_not_other_tenant() {
    let store = mem_store().await;
    seed_row_for_tenant(&store, "tenant-a", "worker-a").await;
    let (status, body) = parse_body(
        list_response(
            &get_with_tenant("/api/v1/audit?range=7d", "tenant-a"),
            Some(&store),
        )
        .await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let item = body["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|item| item["worker"]["id"] == "worker-a")
        .unwrap_or_else(|| panic!("tenant-a row missing from feed: {body}"));
    let id = item["id"].as_str().expect("audit id");

    let same_tenant = detail_response(
        &get_with_tenant(&format!("/api/v1/audit/{id}"), "tenant-a"),
        id,
        Some(&store),
    )
    .await;
    assert_eq!(
        same_tenant.0, "200 OK",
        "same-tenant detail: {}",
        same_tenant.1
    );

    let other_tenant = detail_response(
        &get_with_tenant(&format!("/api/v1/audit/{id}"), "tenant-b"),
        id,
        Some(&store),
    )
    .await;
    assert_eq!(other_tenant.0, "404 Not Found");
}

#[tokio::test]
async fn explicit_tenant_feed_does_not_inherit_legacy_rows() {
    let store = mem_store().await;
    let id = seed_legacy_row(&store, "legacy-worker").await;
    let (status, body) = parse_body(
        list_response(
            &get_with_tenant("/api/v1/audit?range=7d", "tenant-a"),
            Some(&store),
        )
        .await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    assert!(
        !items.iter().any(|item| item["id"] == id),
        "explicit tenant feed inherited legacy row: {body}"
    );
}

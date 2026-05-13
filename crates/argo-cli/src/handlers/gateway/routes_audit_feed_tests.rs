//! Tests for `routes_audit_feed` — `GET /api/v1/audit` (Stream J14).
//!
//! Seeds a `:memory:` store with synthesized `audit_record` rows
//! whose `payload_json` carries the 7 filter dimensions, then drives
//! `list_response` directly. The bearer-auth gate lives upstream
//! (`routes.rs::dispatch_inner`); these tests target the handler.

#![allow(clippy::too_many_lines)]

use super::list_response;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed_row(store: &Arc<SessionStore>, payload: Value) {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
        .expect("write audit row");
}

/// Standard row shape carrying every dimension the feed filters on.
#[allow(clippy::too_many_arguments)]
fn row(
    worker_id: &str,
    channel: &str,
    action_type: &str,
    status: &str,
    approver_id: Option<&str>,
    cost_usd: Option<f64>,
    classification: &str,
    verb: &str,
) -> Value {
    let mut v = json!({
        "endpoint": "test.action",
        "worker_id": worker_id,
        "worker_name": format!("Worker-{worker_id}"),
        "channel": channel,
        "action_type": action_type,
        "outcome_status": status,
        "data_classification": classification,
        "verb": verb,
    });
    if let Some(a) = approver_id {
        v["approver_id"] = json!(a);
        v["approver_name"] = json!(format!("Approver-{a}"));
    }
    if let Some(c) = cost_usd {
        v["cost_usd"] = json!(c);
    }
    v
}

fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn parse_body(resp: (&str, String)) -> (&str, Value) {
    let v: Value = serde_json::from_str(&resp.1)
        .unwrap_or_else(|_| panic!("invalid JSON body: {}", resp.1));
    (resp.0, v)
}

async fn seed_minimal_two(store: &Arc<SessionStore>) {
    seed_row(
        store,
        row("w1", "web", "approval.decided", "succeeded", Some("a1"),
            Some(0.5), "internal", "approved transfer"),
    )
    .await;
    seed_row(
        store,
        row("w2", "slack", "message.sent", "failed", None,
            Some(0.001), "public", "sent message to channel"),
    )
    .await;
}

#[tokio::test]
async fn returns_200_with_items_and_no_filter() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) = parse_body(list_response(&get("/api/v1/audit"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    // Two seeded rows + the audit.feed.queried row this call emits.
    assert!(items.len() >= 2);
}

#[tokio::test]
async fn returns_503_when_store_missing() {
    let (status, body) = list_response(&get("/api/v1/audit"), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn filters_by_worker_id() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["worker"]["id"], "w1", "row leaked: {it}");
    }
}

#[tokio::test]
async fn filters_by_channel() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?channel=slack"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["channel"], "slack");
    }
}

#[tokio::test]
async fn filters_by_action_type() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) = parse_body(
        list_response(&get("/api/v1/audit?action_type=message.sent"), Some(&store)).await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["action_type"], "message.sent");
    }
}

#[tokio::test]
async fn filters_by_status() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?status=failed"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["status"], "failed");
    }
}

#[tokio::test]
async fn filters_by_approver() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?approver=a1"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["approver"]["id"], "a1");
    }
}

#[tokio::test]
async fn filters_by_cost_band() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    // w1 had cost 0.5 → from001to10
    let (_, body) = parse_body(
        list_response(&get("/api/v1/audit?cost_band=from001to10"), Some(&store)).await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    assert!(items.iter().any(|it| it["worker"]["id"] == "w1"));
    assert!(items.iter().all(|it| it["worker"]["id"] != "w2"));
}

#[tokio::test]
async fn filters_by_classification() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) = parse_body(
        list_response(&get("/api/v1/audit?classification=public"), Some(&store)).await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["data_classification"], "public");
    }
}

#[tokio::test]
async fn multi_value_dimension_is_or() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) = parse_body(
        list_response(
            &get("/api/v1/audit?status=succeeded,failed&worker=w1,w2"),
            Some(&store),
        )
        .await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|it| it["worker"]["id"].as_str())
        .collect();
    assert!(ids.contains(&"w1"));
    assert!(ids.contains(&"w2"));
}

#[tokio::test]
async fn substring_query_q_matches_verb_case_insensitive() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?q=APPROVED"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty(), "case-insensitive q should match");
    for it in items {
        assert!(it["verb"].as_str().unwrap().to_lowercase().contains("approved"));
    }
}

#[tokio::test]
async fn empty_result_when_no_match() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=nope"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    assert!(body["items"].as_array().unwrap().is_empty());
    assert!(body.get("next_cursor").is_none());
}

#[tokio::test]
async fn pagination_emits_cursor_and_continues() {
    let store = mem_store().await;
    for i in 0..6 {
        seed_row(
            &store,
            row(&format!("w{i}"), "web", "msg", "succeeded", None, None, "internal", "did x"),
        )
        .await;
    }
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?limit=2"), Some(&store)).await).await;
    let first_items = body["items"].as_array().unwrap().clone();
    assert_eq!(first_items.len(), 2);
    let cursor = body["next_cursor"].as_str().expect("cursor present");
    let path = format!("/api/v1/audit?limit=2&cursor={cursor}");
    let (_, body2) = parse_body(list_response(&get(&path), Some(&store)).await).await;
    let second_items = body2["items"].as_array().unwrap().clone();
    assert_eq!(second_items.len(), 2);
    let first_ids: Vec<&str> = first_items.iter().map(|it| it["id"].as_str().unwrap()).collect();
    for it in &second_items {
        assert!(!first_ids.contains(&it["id"].as_str().unwrap()));
    }
}

#[tokio::test]
async fn rejects_limit_out_of_range_400() {
    let store = mem_store().await;
    let (status, body) = list_response(&get("/api/v1/audit?limit=0"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
    let (status, body) = list_response(&get("/api/v1/audit?limit=999"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
}

#[tokio::test]
async fn rejects_invalid_limit_400() {
    let store = mem_store().await;
    let (status, body) = list_response(&get("/api/v1/audit?limit=abc"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
}

#[tokio::test]
async fn limit_caps_default_to_fifty() {
    let store = mem_store().await;
    for i in 0..60 {
        seed_row(
            &store,
            row(&format!("w{i}"), "web", "msg", "succeeded", None, None, "internal", "did x"),
        )
        .await;
    }
    let (_, body) = parse_body(list_response(&get("/api/v1/audit"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 50);
    assert!(body["next_cursor"].as_str().is_some());
}

#[tokio::test]
async fn range_1h_filters_to_recent_only() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?range=1h"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    // Both seeded rows + the audit-write itself are within the 1h window.
    assert!(!body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_invalid_range_400() {
    let store = mem_store().await;
    let (status, body) =
        list_response(&get("/api/v1/audit?range=zzz"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("range"));
}

#[tokio::test]
async fn rejects_invalid_cursor_400() {
    let store = mem_store().await;
    let (status, body) =
        list_response(&get("/api/v1/audit?cursor=not_base64!!!"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("cursor"));
}

#[tokio::test]
async fn redacts_email_in_verb_by_default() {
    let store = mem_store().await;
    seed_row(
        &store,
        row("w1", "web", "msg", "succeeded", None, None, "internal",
            "ping sam@acmeco.com to confirm"),
    )
    .await;
    let (_, body) = parse_body(
        list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    let verbs: Vec<&str> = items.iter().filter_map(|it| it["verb"].as_str()).collect();
    assert!(verbs.iter().any(|v| v.contains("[EMAIL]")));
    assert!(verbs.iter().all(|v| !v.contains("sam@acmeco.com")));
}

#[tokio::test]
async fn emits_audit_row_per_query() {
    let store = mem_store().await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let (status, _) = list_response(&get("/api/v1/audit"), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(after, before + 1, "expected exactly one audit.feed.queried row");
}

// ─────────────────────────────────────────────────────────────────
// Stream `mission-enterprise-cross-tenant-audit` — tenant scoping
// ─────────────────────────────────────────────────────────────────

async fn seed_row_for_tenant(store: &Arc<SessionStore>, tenant_id: &str, worker: &str) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    sink.write(None, AuditKind::HttpRequest, row(worker, "web", "msg", "succeeded",
        None, None, "internal", "did x"), None)
        .await
        .expect("write tenant-scoped row");
    // Drop the batched-writer latency-window before the test reads back. The
    // sink-per-helper-call pattern queues into a per-sink batch that would
    // otherwise be lost when the sink drops at end-of-scope.
    sink.flush_pending().await.expect("flush tenant-scoped row");
}

fn get_with_tenant(path: &str, tenant: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n"
    )
}

fn get_with_tenant_and_cap(path: &str, tenant: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\
         X-Argo-Audit-Cross-Tenant: 1\r\n\r\n"
    )
}

#[tokio::test]
async fn tenant_scope_default_excludes_other_tenants() {
    let store = mem_store().await;
    seed_row_for_tenant(&store, "tenant-a", "wa").await;
    seed_row_for_tenant(&store, "tenant-b", "wb").await;
    let (status, body) = parse_body(
        list_response(&get_with_tenant("/api/v1/audit", "tenant-a"), Some(&store)).await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items");
    // tenant-a rows visible; tenant-b rows hidden. The audit.feed.queried
    // emit-row stamps tenant_id from the sink default ("ws-default") so it
    // does not leak into a tenant-a-scoped feed.
    let workers: Vec<&str> = items
        .iter()
        .filter_map(|it| it["worker"]["id"].as_str())
        .collect();
    assert!(workers.contains(&"wa"), "expected tenant-a row, got {workers:?}");
    assert!(!workers.contains(&"wb"), "tenant-b row leaked: {workers:?}");
}

#[tokio::test]
async fn as_tenant_without_capability_silently_ignored() {
    let store = mem_store().await;
    seed_row_for_tenant(&store, "tenant-a", "wa").await;
    seed_row_for_tenant(&store, "tenant-b", "wb").await;
    // No X-Argo-Audit-Cross-Tenant header → as_tenant param is parsed
    // (for telemetry) but the read scope stays on tenant-a.
    let (status, body) = parse_body(
        list_response(
            &get_with_tenant("/api/v1/audit?as_tenant=tenant-b", "tenant-a"),
            Some(&store),
        )
        .await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items");
    let workers: Vec<&str> = items
        .iter()
        .filter_map(|it| it["worker"]["id"].as_str())
        .collect();
    assert!(!workers.contains(&"wb"), "cross-tenant leak: {workers:?}");
}

#[tokio::test]
async fn as_tenant_with_capability_returns_other_tenant() {
    let store = mem_store().await;
    seed_row_for_tenant(&store, "tenant-a", "wa").await;
    seed_row_for_tenant(&store, "tenant-b", "wb").await;
    let (status, body) = parse_body(
        list_response(
            &get_with_tenant_and_cap("/api/v1/audit?as_tenant=tenant-b", "tenant-a"),
            Some(&store),
        )
        .await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items");
    let workers: Vec<&str> = items
        .iter()
        .filter_map(|it| it["worker"]["id"].as_str())
        .collect();
    assert!(workers.contains(&"wb"), "expected tenant-b row, got {workers:?}");
    assert!(!workers.contains(&"wa"), "tenant-a row leaked: {workers:?}");
}

#[tokio::test]
async fn audit_row_payload_lists_dims_used() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, _) = list_response(
        &get("/api/v1/audit?worker=w1&q=approved&range=7d"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(payload.contains("\"worker\""));
    assert!(payload.contains("\"q\""));
    assert!(payload.contains("\"range\""));
}

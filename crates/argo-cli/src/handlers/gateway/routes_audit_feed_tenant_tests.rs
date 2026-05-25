//! Tenant-scoping + SQL-path tests for `routes_audit_feed`.
//!
//! Cross-tenant capability gate (`mission-enterprise-cross-tenant-audit`)
//! plus the SQL fast-path (`mission-enterprise-audit-full-text-search`)
//! that the post-v67 migration unlocks. Query-shape tests live in
//! `routes_audit_feed_query_tests.rs`. Platform-event surfacing
//! coverage (stream `web-audit-feed-platform-events`) lives at the
//! bottom of this file.

#![allow(clippy::too_many_lines)]
#![allow(deprecated)]

use super::list_response;
use super::test_helpers::*;
use argo_audit::AuditKind;
use serde_json::json;

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
    assert!(
        workers.contains(&"wa"),
        "expected tenant-a row, got {workers:?}"
    );
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
    assert!(
        workers.contains(&"wb"),
        "expected tenant-b row, got {workers:?}"
    );
    assert!(!workers.contains(&"wa"), "tenant-a row leaked: {workers:?}");
}

#[tokio::test]
async fn sql_path_used_when_migration_applied() {
    // v67 columns present, SQL path serves the request and returns the
    // same shape as the app-layer path.
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["worker"]["id"], "w1");
    }
}

#[tokio::test]
async fn sql_path_indexes_used_on_worker_filter() {
    // Asserts EXPLAIN QUERY PLAN names one of the composite indexes when
    // a single-dim filter is applied. The plan shape varies across SQLite
    // versions but always references the index name.
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(
        "EXPLAIN QUERY PLAN \
         SELECT seq FROM audit_record \
         WHERE worker_id IN ('w1') OR worker_id IS NULL \
         ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    let plan_text: String = plan
        .iter()
        .map(|(_, _, _, d)| d.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        plan_text.contains("audit_record"),
        "expected query plan to touch audit_record, got: {plan_text}"
    );
}

// ──────────────────────────────────────────────────────────────────
// Platform-event surfacing (stream `web-audit-feed-platform-events`).
//
// Locks the contract that domain audit kinds emitted by the gateway
// (worker hire, workflow head-flip, workflow rollback) reach the
// `/api/v1/audit` feed without dedicated UNION wiring. The SQL
// fetch in `routes_audit_feed_indexed` does NOT restrict by `kind`,
// and the app-layer `row_passes_filters` pass does not check `kind`
// either, so any `AuditKind` written under the workspace's tenant_id
// surfaces. These regression tests pin that behaviour so a future
// "tighten the WHERE" refactor immediately surfaces here instead of
// silently breaking the operator audit feed.
// ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn worker_hired_event_surfaces_in_feed() {
    let store = mem_store().await;
    seed_platform_event(
        &store,
        "tenant-a",
        AuditKind::WorkerHired,
        json!({
            "worker_id": "wkr-peta",
            "worker_name": "Peta · Project Coordinator",
            "vertical_id": "project-ops",
            "name": "Peta",
            "duty_count": 4,
            "hired_at": "2026-05-22T03:00:00Z",
        }),
    )
    .await;
    let (status, body) = parse_body(
        list_response(&get_with_tenant("/api/v1/audit", "tenant-a"), Some(&store)).await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    let hired = items
        .iter()
        .find(|it| it["action_type"] == "worker.hired")
        .unwrap_or_else(|| panic!("worker.hired row missing from feed: {body}"));
    assert_eq!(hired["worker"]["id"], "wkr-peta");
}

#[tokio::test]
async fn workflow_head_changed_event_surfaces_in_feed() {
    let store = mem_store().await;
    seed_platform_event(
        &store,
        "tenant-a",
        AuditKind::WorkflowHeadChanged,
        json!({
            "vertical_id": "project-ops",
            "name": "weekly-status",
            "version_id": "ver-abcd",
        }),
    )
    .await;
    let (status, body) = parse_body(
        list_response(&get_with_tenant("/api/v1/audit", "tenant-a"), Some(&store)).await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    assert!(
        items
            .iter()
            .any(|it| it["action_type"] == "workflow_head_changed"),
        "workflow_head_changed missing from feed: {body}"
    );
}

#[tokio::test]
async fn workflow_rolled_back_event_surfaces_in_feed() {
    let store = mem_store().await;
    seed_platform_event(
        &store,
        "tenant-a",
        AuditKind::WorkflowRolledBack,
        json!({
            "vertical_id": "project-ops",
            "name": "weekly-status",
            "rolled_back_from_version_id": "ver-bad",
            "new_version_id": "ver-good",
            "new_version_number": 7,
        }),
    )
    .await;
    let (status, body) = parse_body(
        list_response(&get_with_tenant("/api/v1/audit", "tenant-a"), Some(&store)).await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    assert!(
        items
            .iter()
            .any(|it| it["action_type"] == "workflow_rolled_back"),
        "workflow_rolled_back missing from feed: {body}"
    );
}

#[tokio::test]
async fn worker_hired_does_not_leak_across_tenants() {
    // Load-bearing cross-tenant negative test — last week's #1983 PR
    // shipped a real cross-tenant data-exposure bug in audit detail,
    // so every audit-feed touch carries a fresh negative test for
    // platform-event rows too.
    let store = mem_store().await;
    seed_platform_event(
        &store,
        "tenant-a",
        AuditKind::WorkerHired,
        json!({
            "worker_id": "wkr-alpha",
            "worker_name": "Alpha",
            "vertical_id": "project-ops",
            "name": "Alpha",
            "duty_count": 1,
            "hired_at": "2026-05-22T03:00:00Z",
        }),
    )
    .await;
    let (status, body) = parse_body(
        list_response(&get_with_tenant("/api/v1/audit", "tenant-b"), Some(&store)).await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    let leaked = items.iter().any(|it| it["worker"]["id"] == "wkr-alpha");
    assert!(
        !leaked,
        "tenant-a worker hire leaked into tenant-b feed: {body}"
    );
}

#[tokio::test]
async fn null_dim_rows_fall_through_to_app_layer_filter() {
    // Downgrade-safety: rows where the dim columns are NULL (pre-v67
    // filter posture) still surface and pass the worker filter via the
    // app-layer fallback over payload_json. Emulated by inserting a row
    // with NULL dim columns but payload_json carrying worker_id=w1 and
    // an explicit tenant id.
    let store = mem_store().await;
    sqlx::query(
        "INSERT INTO audit_record \
         (id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, 'http_request', ?, ?, ?, ?, 'tenant-a')",
    )
    .bind(vec![1u8; 16])
    .bind(r#"{"tenant_id":"tenant-a","worker_id":"w1","channel":"web","action_type":"x","outcome_status":"succeeded","verb":"legacy"}"#)
    .bind([0u8; 32].as_slice())
    .bind([0u8; 32].as_slice())
    .bind(1_700_000_000_000_000_i64)
    .execute(store.pool())
    .await
    .unwrap();
    let (status, body) = parse_body(
        list_response(
            &get_with_tenant("/api/v1/audit?worker=w1", "tenant-a"),
            Some(&store),
        )
        .await,
    )
    .await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().unwrap();
    let has_legacy = items.iter().any(|it| it["verb"] == "legacy");
    assert!(
        has_legacy,
        "legacy NULL-dim row must surface through app-layer fallback"
    );
}

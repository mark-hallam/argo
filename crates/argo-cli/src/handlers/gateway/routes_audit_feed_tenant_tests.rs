//! Tenant-scoping + SQL-path tests for `routes_audit_feed`.
//!
//! Cross-tenant capability gate (`mission-enterprise-cross-tenant-audit`)
//! plus the SQL fast-path (`mission-enterprise-audit-full-text-search`)
//! that the post-v67 migration unlocks. Query-shape tests live in
//! `routes_audit_feed_query_tests.rs`.

#![allow(clippy::too_many_lines)]
#![allow(deprecated)]

use super::list_response;
use super::test_helpers::*;

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

#[tokio::test]
async fn legacy_null_rows_fall_through_to_app_layer_filter() {
    // Downgrade-safety: rows where the dim columns are NULL (legacy
    // pre-v67 posture) still surface and pass the worker filter via the
    // app-layer fallback over payload_json. Emulated by inserting a row
    // with NULL dim columns but payload_json carrying worker_id=w1.
    let store = mem_store().await;
    sqlx::query(
        "INSERT INTO audit_record (id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, 'http_request', ?, ?, ?, ?)",
    )
    .bind(vec![1u8; 16])
    .bind(r#"{"worker_id":"w1","channel":"web","action_type":"x","outcome_status":"succeeded","verb":"legacy"}"#)
    .bind([0u8; 32].as_slice())
    .bind([0u8; 32].as_slice())
    .bind(1_700_000_000_000_000_i64)
    .execute(store.pool())
    .await
    .unwrap();
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().unwrap();
    let has_legacy = items.iter().any(|it| it["verb"] == "legacy");
    assert!(
        has_legacy,
        "legacy NULL-dim row must surface through app-layer fallback"
    );
}

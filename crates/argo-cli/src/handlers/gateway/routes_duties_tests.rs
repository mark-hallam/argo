//! Tests for `routes_duties` (part 1 — classify / commit / preview) —
//! Stream J13.
//!
//! Drives the entry points directly against an in-memory SQLite store
//! seeded with a `Worker` row. Bearer auth + principal-header parsing live
//! upstream in `routes_dispatch_post`; these tests focus on body shapes,
//! validation, conflict detection, and audit-row side-effects. The
//! patch/delete/list/store-missing/audit tests live in
//! `routes_duties_tests2.rs`; shared fixtures in
//! `routes_duties_test_helpers.rs`.

#![allow(clippy::too_many_lines)]

use super::duty_test_helpers::*;

#[tokio::test]
async fn classify_returns_proposal_with_samples_and_no_conflicts_on_fresh_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({ "input": "send outbound sales emails every morning" }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/classify", worker.id),
        &body,
    );
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["duty_proposal"]["schedule"].as_str().is_some());
    assert!(v["duty_proposal"]["entry_action"].as_str().is_some());
    assert!(v["duty_proposal"]["label"].as_str().is_some());
    let samples = v["sample_outputs"].as_array().expect("array");
    assert_eq!(samples.len(), 3);
    assert!(v["conflicts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn classify_rejects_malformed_cron_hint_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Devon").await;
    let body = json!({ "input": "ops check daily", "cron_hint": "not a cron" }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/classify", worker.id),
        &body,
    );
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn classify_returns_404_for_unknown_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = WorkerId::new();
    let body = json!({ "input": "summarize day" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{ghost}/duties/classify"), &body);
    let (status, resp) = classify_response(&ghost.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
}

/// `mission-cross-tenant-worker-handler-no-existence-leak`: cross-tenant
/// existence is hidden — both branches return byte-identical 404.
#[tokio::test]
async fn cross_tenant_worker_duties_returns_same_404_as_unknown_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let alien = add_worker(&store, other, "Alien").await;
    let body = json!({ "input": "alien duty" }).to_string();
    let req_alien = post_req(
        &format!("/api/v1/workers/{}/duties/classify", alien.id),
        &body,
    );
    let (status_alien, body_alien) =
        classify_response(&alien.id.to_string(), &req_alien, Some(&store)).await;
    let ghost = WorkerId::new();
    let req_ghost = post_req(&format!("/api/v1/workers/{ghost}/duties/classify"), &body);
    let (status_ghost, body_ghost) =
        classify_response(&ghost.to_string(), &req_ghost, Some(&store)).await;
    assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
    assert_eq!(
        status_alien, status_ghost,
        "cross-tenant status must match unknown-worker status"
    );
    assert_eq!(
        body_alien, body_ghost,
        "cross-tenant body must be byte-identical to unknown-worker body"
    );
    assert!(
        !body_alien.contains(&alien.id.to_string()),
        "response must not echo requested worker_id"
    );
}

#[tokio::test]
async fn classify_archived_worker_returns_410() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Sundown").await;
    IdentityStore::new(&store)
        .archive_worker(tenant, worker.id)
        .await
        .expect("archive worker");
    let body = json!({ "input": "x" }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/classify", worker.id),
        &body,
    );
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "410 Gone", "body: {resp}");
}

#[tokio::test]
async fn commit_inserts_duty_and_returns_first_fire() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let row: Option<String> = sqlx::query_scalar("SELECT description FROM role_duty WHERE id = ?")
        .bind(&id)
        .fetch_optional(store.pool())
        .await
        .unwrap();
    assert_eq!(row.as_deref(), Some("Daily summary"));
}

#[tokio::test]
async fn commit_rejects_short_label_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({
        "duty_proposal": { "label": "abc", "schedule": "0 6 * * *", "entry_action": "ping" }
    })
    .to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/commit", worker.id),
        &body,
    );
    let (status, resp) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["error"]["message"].as_str().unwrap().contains("label"));
}

#[tokio::test]
async fn commit_rejects_non_5_field_cron_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({
        "duty_proposal": { "label": "Daily ping", "schedule": "0 0 6 * * *", "entry_action": "ping" }
    }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/commit", worker.id),
        &body,
    );
    let (status, _) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn commit_detects_cron_conflict_and_requires_resolution() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let _first = commit_simple(&store, &worker.id, "0 6 * * *", "First duty").await;
    let body = json!({
        "duty_proposal": { "label": "Second duty", "schedule": "0 6 * * *", "entry_action": "ping" }
    })
    .to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/commit", worker.id),
        &body,
    );
    let (status, resp) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "409 Conflict", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "duty_conflict");
}

#[tokio::test]
async fn commit_with_replace_resolution_deletes_overlap() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let first = commit_simple(&store, &worker.id, "0 6 * * *", "First duty").await;
    let body = json!({
        "duty_proposal": { "label": "Replacement", "schedule": "0 6 * * *", "entry_action": "ping" },
        "accepted_conflict_resolution": "replace",
    }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/commit", worker.id),
        &body,
    );
    let (status, _) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "201 Created");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE id = ?")
        .bind(&first)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0, "old duty should have been deleted");
}

#[tokio::test]
async fn preview_returns_requested_samples() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let body = json!({ "duty_id": duty_id, "sample_size": 2 }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/preview", worker.id),
        &body,
    );
    let (status, resp) = preview_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["samples"].as_array().unwrap().len(), 2);
    assert!(v["samples"][0]["would_send"].as_str().is_some());
}

#[tokio::test]
async fn preview_unknown_duty_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({ "duty_id": "does-not-exist", "sample_size": 3 }).to_string();
    let req = post_req(
        &format!("/api/v1/workers/{}/duties/preview", worker.id),
        &body,
    );
    let (status, _) = preview_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

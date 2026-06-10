//! Tests for `routes_duties` (part 2 — patch / delete / list / store-missing
//! / audit-row side-effects). Split out of `routes_duties_tests.rs` to keep
//! each file under the per-file LOC budget; shared fixtures live in
//! `routes_duties_test_helpers.rs`.

#![allow(clippy::too_many_lines)]

use super::duty_test_helpers::*;

#[tokio::test]
async fn patch_updates_label_and_schedule_and_enabled() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let body = json!({ "label": "Updated summary", "schedule": "0 7 * * *", "enabled": false })
        .to_string();
    let path = format!("/api/v1/workers/{}/duties/{duty_id}", worker.id);
    let req = patch_request(&path, &body);
    let (status, resp) = patch_response(&worker.id.to_string(), &duty_id, &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["duty"]["label"], "Updated summary");
    assert_eq!(v["duty"]["enabled"], false);
    assert!(v["duty"]["cron"].as_str().unwrap().contains("0 7"));
}

#[tokio::test]
async fn patch_empty_body_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let req = patch_request(
        &format!("/api/v1/workers/{}/duties/{duty_id}", worker.id),
        "{}",
    );
    let (status, _) = patch_response(&worker.id.to_string(), &duty_id, &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn delete_removes_row_and_returns_204() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let path = format!("/api/v1/workers/{}/duties/{duty_id}", worker.id);
    let (status, body) = delete_response(
        &worker.id.to_string(),
        &duty_id,
        &delete_req(&path),
        Some(&store),
    )
    .await;
    assert_eq!(status, "204 No Content");
    assert!(body.is_empty(), "204 should have empty body, got: {body}");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE id = ?")
        .bind(&duty_id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn delete_unknown_duty_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let path = format!("/api/v1/workers/{}/duties/missing", worker.id);
    let (status, _) = delete_response(
        &worker.id.to_string(),
        "missing",
        &delete_req(&path),
        Some(&store),
    )
    .await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn list_returns_committed_duties_scoped_to_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let other = add_worker(&store, tenant, "Devon").await;
    // Commit one duty for each worker in the SAME tenant; the list for
    // `worker` must return only `worker`'s row (real role_duty read, not
    // a fixture), proving both persistence and worker-scoping.
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let _other_id = commit_simple(&store, &other.id, "0 7 * * *", "Devon ping").await;
    let req = get_req(&format!("/api/v1/workers/{}/duties", worker.id));
    let (status, resp) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let duties = v["duties"].as_array().expect("duties array");
    assert_eq!(duties.len(), 1, "only this worker's duty, got: {resp}");
    assert_eq!(duties[0]["id"], duty_id);
    assert_eq!(duties[0]["label"], "Daily summary");
    assert!(duties[0]["cron"].as_str().unwrap().contains("0 6"));
}

/// `mission-cross-tenant-worker-handler-no-existence-leak`: listing a
/// tenant-B worker's duties from a tenant-A request returns the SAME
/// byte-identical 404 an unknown worker returns (mirrors the mutation
/// invariant).
#[tokio::test]
async fn list_cross_tenant_returns_same_404_as_unknown_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let alien = add_worker(&store, other, "Alien").await;
    let req_alien = get_req(&format!("/api/v1/workers/{}/duties", alien.id));
    let (status_alien, body_alien) =
        list_response(&alien.id.to_string(), &req_alien, Some(&store)).await;
    let ghost = WorkerId::new();
    let req_ghost = get_req(&format!("/api/v1/workers/{ghost}/duties"));
    let (status_ghost, body_ghost) =
        list_response(&ghost.to_string(), &req_ghost, Some(&store)).await;
    assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
    assert_eq!(status_alien, status_ghost);
    assert_eq!(
        body_alien, body_ghost,
        "cross-tenant body must be byte-identical to unknown-worker body"
    );
    assert!(!body_alien.contains(&alien.id.to_string()));
}

#[tokio::test]
async fn returns_503_when_store_missing() {
    let id = WorkerId::new();
    let body = json!({ "input": "x" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{id}/duties/classify"), &body);
    let (status, resp) = classify_response(&id.to_string(), &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(resp.contains("session store unavailable"));
}

#[tokio::test]
async fn commit_emits_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%duty.commit%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let _ = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%duty.commit%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(after, before + 1);
}

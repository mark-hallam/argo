//! Status-code response tests for `routes_worker_overview` — 400 / 403 /
//! 404 / 410 / 503 paths plus query-string dispatch.

use super::*;

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let env = prep().await;
    let req = req_for("not-a-uuid", None, None);
    let (status, body) = show_response("not-a-uuid", &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unknown_worker_id_returns_404() {
    let env = prep().await;
    let ghost = WorkerId::new().to_string();
    let req = req_for(&ghost, None, None);
    let (status, body) = show_response(&ghost, &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

/// `mission-cross-tenant-worker-handler-no-existence-leak`: cross-tenant
/// existence is hidden — both branches return byte-identical 404.
#[tokio::test]
async fn cross_tenant_worker_overview_returns_same_404_as_unknown_worker() {
    let env = prep().await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&env.store, alien_tenant, "human:mark").await;
    let req_alien = req_for(&alien.id.to_string(), None, None);
    let (status_alien, body_alien) =
        show_response(&alien.id.to_string(), &req_alien, Some(&env.store)).await;
    let ghost = WorkerId::new().to_string();
    let req_ghost = req_for(&ghost, None, None);
    let (status_ghost, body_ghost) = show_response(&ghost, &req_ghost, Some(&env.store)).await;
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
async fn archived_worker_returns_410_with_recoverable_until() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    IdentityStore::new(&env.store)
        .archive_worker(env.tenant, worker.id)
        .await
        .expect("archive");
    let req = req_for(&worker.id.to_string(), None, None);
    let (status, body) = show_response(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "410 Gone", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "gone");
    assert!(v["archived_at"].is_string());
    assert!(v["recoverable_until_at"].is_string());
    assert!(v["retry_after_seconds"].is_number());
}

#[tokio::test]
async fn handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let id = WorkerId::new().to_string();
    let req = req_for(&id, None, None);
    let (status, body) = show_response(&id, &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn dispatch_path_ignores_query_string() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let id_q = format!("{}?ts=1", worker.id);
    let req = format!(
        "GET /api/v1/workers/{id_q}/overview HTTP/1.1\r\nHost: x\r\nX-Argo-Role: operator\r\n\r\n"
    );
    let (status, body) = show_response(&id_q, &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
}

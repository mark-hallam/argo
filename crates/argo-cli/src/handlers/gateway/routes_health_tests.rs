//! Unit tests for the readiness probe handlers in
//! `routes_health.rs`. Six scenarios:
//!
//!   1. `health_ok_all_checks_pass` — happy path, fresh store +
//!      migrated + no providers dir + no credstore → 200 with the
//!      new shape including `schema_version` and `uptime_seconds`.
//!   2. `health_degraded_schema_mismatch` — forge an "ahead"
//!      schema_version row and confirm 503 + reason
//!      `schema_version_mismatch`.
//!   3. `health_degraded_store_unavailable` — pass `None` for the
//!      store; expect 503 + reason `store_unavailable`. Captures
//!      the "store_timeout" failure class without depending on
//!      sqlx timeout plumbing.
//!   4. `livez_always_ok` — `live_response` is sync, takes no
//!      args, returns 200 with `status:"alive"`.
//!   5. `readyz_handler_returns_health_shape` — same fn backs
//!      `/health` and `/readyz`, so calling once and checking the
//!      shape confirms the alias contract at the handler layer.
//!      The dispatch-level wiring is verified by the integration
//!      smoke (`tests/gateway_health_smoke.rs`).
//!   6. `uptime_counter_increments` — call twice with a sleep
//!      between, confirm `uptime_seconds` is non-decreasing.

use super::{health_response_with_cwd, live_response, mark_start};
use argo_store::{EXPECTED_SCHEMA_VERSION, SessionStore};
use std::sync::Arc;

async fn fresh_store(tmp: &tempfile::TempDir) -> Arc<SessionStore> {
    let db = tmp.path().join("sessions.db");
    let store = SessionStore::open(&db).await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

#[tokio::test]
async fn health_ok_all_checks_pass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = fresh_store(&tmp).await;
    mark_start();
    let (status, body) = health_response_with_cwd(Some(&store), Some(tmp.path())).await;
    assert_eq!(status, "200 OK", "body={body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("parse body");
    assert_eq!(v["status"], "ok", "body={body}");
    assert!(
        v["schema_version"].as_u64().is_some(),
        "schema_version missing: {body}"
    );
    assert!(
        v["uptime_seconds"].as_u64().is_some(),
        "uptime_seconds missing: {body}"
    );
}

#[tokio::test]
async fn health_degraded_schema_mismatch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = tmp.path().join("sessions.db");
    let store = SessionStore::open(&db).await.expect("open store");
    // Don't run full migrate() — forge a schema_version row that
    // points past EXPECTED_SCHEMA_VERSION so the readiness check
    // takes the directional "mismatch" branch deterministically.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (\
            id INTEGER PRIMARY KEY CHECK (id = 1), \
            version INTEGER NOT NULL, \
            applied_at INTEGER NOT NULL)",
    )
    .execute(store.pool())
    .await
    .expect("create schema_version");
    let bumped = i64::from(EXPECTED_SCHEMA_VERSION) + 5;
    sqlx::query(
        "INSERT OR REPLACE INTO schema_version (id, version, applied_at) \
         VALUES (1, ?, 0)",
    )
    .bind(bumped)
    .execute(store.pool())
    .await
    .expect("seed");
    let store = Arc::new(store);
    let (status, body) = health_response_with_cwd(Some(&store), Some(tmp.path())).await;
    assert_eq!(status, "503 Service Unavailable", "body={body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("parse body");
    assert_eq!(v["status"], "degraded");
    assert_eq!(v["reason"], "schema_version_mismatch", "body={body}");
}

#[tokio::test]
async fn health_degraded_store_unavailable() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (status, body) = health_response_with_cwd(None, Some(tmp.path())).await;
    assert_eq!(status, "503 Service Unavailable", "body={body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("parse body");
    assert_eq!(v["status"], "degraded");
    assert_eq!(v["reason"], "store_unavailable", "body={body}");
}

#[test]
fn livez_always_ok() {
    let (status, body) = live_response();
    assert_eq!(status, "200 OK");
    let v: serde_json::Value = serde_json::from_str(&body).expect("parse body");
    assert_eq!(v["status"], "alive", "body={body}");
}

#[tokio::test]
async fn readyz_handler_returns_health_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = fresh_store(&tmp).await;
    mark_start();
    // /readyz routes through health_response — calling it directly
    // and asserting the shape confirms the alias contract at the
    // handler layer (dispatch wiring is covered by the smoke).
    let (status, body) = health_response_with_cwd(Some(&store), Some(tmp.path())).await;
    assert_eq!(status, "200 OK");
    assert!(
        body.contains("\"schema_version\":") && body.contains("\"uptime_seconds\":"),
        "body must carry new shape: {body}"
    );
}

#[tokio::test]
async fn uptime_counter_increments() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = fresh_store(&tmp).await;
    mark_start();
    let (_s1, b1) = health_response_with_cwd(Some(&store), Some(tmp.path())).await;
    let v1: serde_json::Value = serde_json::from_str(&b1).expect("parse 1");
    let u1 = v1["uptime_seconds"].as_u64().expect("uptime 1");
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (_s2, b2) = health_response_with_cwd(Some(&store), Some(tmp.path())).await;
    let v2: serde_json::Value = serde_json::from_str(&b2).expect("parse 2");
    let u2 = v2["uptime_seconds"].as_u64().expect("uptime 2");
    assert!(u2 > u1, "uptime did not advance: u1={u1} u2={u2}");
}

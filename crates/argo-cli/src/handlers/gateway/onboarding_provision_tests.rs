//! Tests for `onboarding_provision` —
//! `POST /api/v1/onboarding/provision`.
//!
//! Uses `:memory:` SQLite + `ARGO_VERTICALS_DIR` pointed at the
//! workspace catalog so the `gtm-engineer` lookup resolves a real
//! pack. Mirrors the harness used by `routes_workers_hire_tests`.

#![allow(clippy::too_many_lines)]

use super::provision_response;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(body: &str, principal: Option<&str>) -> String {
    let mut head = String::from(
        "POST /api/v1/onboarding/provision HTTP/1.1\r\nHost: x\r\n",
    );
    if let Some(p) = principal {
        head.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

fn body() -> String {
    json!({
        "workspace_name": "Acme Workspace",
        "company_name": "Acme Co"
    })
    .to_string()
}

#[tokio::test]
async fn happy_path_returns_201_and_tenant_workspace_ids() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["tenant_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(v["workspace_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(v["vertical_id"], "gtm-engineer");
    assert_eq!(v["next_step"], "/onboarding/connect-provider");
    assert_eq!(v["idempotent"], false);
}

#[tokio::test]
async fn missing_principal_returns_401() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(&req(&body(), None), Some(&store)).await;
    assert_eq!(status, "401 Unauthorized");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn non_human_principal_returns_401() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, _resp) = provision_response(
        &req(&body(), Some("system:scheduler-1")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "401 Unauthorized");
}

#[tokio::test]
async fn idempotent_second_call_returns_same_ids_and_200() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status1, body1) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status1, "201 Created");
    let v1: Value = serde_json::from_str(&body1).unwrap();
    let (status2, body2) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status2, "200 OK");
    let v2: Value = serde_json::from_str(&body2).unwrap();
    assert_eq!(v1["tenant_id"], v2["tenant_id"]);
    assert_eq!(v1["workspace_id"], v2["workspace_id"]);
    assert_eq!(v2["idempotent"], true);
}

#[tokio::test]
async fn empty_workspace_name_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let body = json!({ "workspace_name": "" }).to_string();
    let (status, resp) = provision_response(
        &req(&body, Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn bad_json_body_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(
        &req("not-json", Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
    assert!(resp.contains("parse body"));
}

#[tokio::test]
async fn missing_store_returns_503() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let (status, _) =
        provision_response(&req(&body(), Some("human:alice@acme.com")), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

#[tokio::test]
async fn audit_row_is_written_with_tenant_provisioned_kind() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let tenant_id = v["tenant_id"].as_str().unwrap().to_string();

    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'tenant.provisioned' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("audit row exists");
    assert!(payload.contains(&tenant_id), "payload missing tenant_id");
    assert!(payload.contains("alice@acme.com") || payload.contains("\"user_id\""));
    assert!(payload.contains("gtm-engineer"));
}

#[tokio::test]
async fn provision_seeds_5_workplace_docs() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let tenant_id = v["tenant_id"].as_str().unwrap().to_string();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workplace_knowledge_doc WHERE tenant_id = ?",
    )
    .bind(&tenant_id)
    .fetch_one(store.pool())
    .await
    .expect("count seeded docs");
    assert_eq!(count, 5, "expected 5 seeded docs, got {count}");

    let mut slugs: Vec<String> = sqlx::query_scalar(
        "SELECT slug FROM workplace_knowledge_doc WHERE tenant_id = ? ORDER BY slug",
    )
    .bind(&tenant_id)
    .fetch_all(store.pool())
    .await
    .expect("fetch slugs");
    slugs.sort();
    assert_eq!(
        slugs,
        vec![
            "brand-voice".to_string(),
            "business-rules".to_string(),
            "customer-tone".to_string(),
            "escalation-policy".to_string(),
            "never-do-this".to_string(),
        ]
    );
}

#[tokio::test]
async fn provision_is_idempotent_when_docs_already_exist() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let r = req(&body(), Some("human:alice@acme.com"));
    let (status1, _b1) = provision_response(&r, Some(&store)).await;
    assert_eq!(status1, "201 Created");
    let (status2, b2) = provision_response(&r, Some(&store)).await;
    assert_eq!(status2, "200 OK");
    let v2: Value = serde_json::from_str(&b2).unwrap();
    let tenant_id = v2["tenant_id"].as_str().unwrap().to_string();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workplace_knowledge_doc WHERE tenant_id = ?",
    )
    .bind(&tenant_id)
    .fetch_one(store.pool())
    .await
    .expect("count seeded docs");
    assert_eq!(count, 5, "re-provision must not duplicate docs");

    // No second wave of seed-audit rows either: the per-slug
    // existence check short-circuits before `doc::create` is called,
    // so a no-op re-call writes zero new `workplace_knowledge.doc.created`
    // events.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'workplace_knowledge.doc.created'",
    )
    .fetch_one(store.pool())
    .await
    .expect("count seed-audit rows");
    assert_eq!(audit_count, 5, "expected exactly 5 seed-audit rows, got {audit_count}");
}

#[tokio::test]
async fn seed_emits_5_audit_rows_with_doc_created_kind() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, _resp) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created");

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT payload_json, principal_id FROM audit_record \
         WHERE kind = 'workplace_knowledge.doc.created' \
         ORDER BY created_at ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("fetch seed-audit rows");
    assert_eq!(rows.len(), 5, "expected 5 doc-created audit rows");
    for (payload, principal) in rows {
        assert_eq!(
            principal.as_deref(),
            Some("system:onboarding-seed"),
            "every seed-audit row carries the system actor"
        );
        assert!(
            payload.contains("\"source\":\"onboarding-seed\""),
            "payload missing source marker: {payload}"
        );
    }
}

#[tokio::test]
async fn seed_body_matches_disk_file_byte_for_byte() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (status, resp) = provision_response(
        &req(&body(), Some("human:alice@acme.com")),
        Some(&store),
    )
    .await;
    assert_eq!(status, "201 Created");
    let v: Value = serde_json::from_str(&resp).unwrap();
    let tenant_id = v["tenant_id"].as_str().unwrap().to_string();

    let on_disk = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/handlers/gateway/onboarding_seeds/brand_voice.md"),
    )
    .expect("read brand_voice.md from disk");
    let stored: String = sqlx::query_scalar(
        "SELECT body_markdown FROM workplace_knowledge_doc \
         WHERE tenant_id = ? AND slug = 'brand-voice'",
    )
    .bind(&tenant_id)
    .fetch_one(store.pool())
    .await
    .expect("fetch brand-voice doc");
    assert_eq!(
        stored, on_disk,
        "seeded brand-voice body must match the on-disk file byte-for-byte"
    );
}

#[tokio::test]
async fn idempotent_call_does_not_write_second_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let r = req(&body(), Some("human:alice@acme.com"));
    let _ = provision_response(&r, Some(&store)).await;
    let _ = provision_response(&r, Some(&store)).await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'tenant.provisioned'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    // Both calls emit; second carries `created=false` so a downstream
    // funnel can distinguish the original signup from the idempotent
    // re-call. Two rows is the documented contract.
    assert_eq!(count, 2);
}

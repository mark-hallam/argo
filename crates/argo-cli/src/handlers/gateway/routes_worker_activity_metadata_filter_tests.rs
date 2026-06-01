//! Default-filter behavior for `GET /api/v1/workers/<id>/activity`:
//! system-meta rows (SPA self-pings, `worker.kpi_rolled` cron
//! telemetry, audit-page click telemetry) MUST be hidden unless
//! `?include_meta=1` is passed.
//!
//! Lives in a sibling test module instead of being appended to
//! `routes_worker_activity_tests.rs` so that file stays under the
//! 500-LOC per-file ceiling (HARD RULE 6).

#![allow(deprecated)]

use super::list_response;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
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
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn seed_kind(store: &Arc<SessionStore>, kind: AuditKind, payload: Value) {
    AuditSink::new(Arc::clone(store))
        .write(None, kind, payload, None)
        .await
        .expect("write audit row");
}

async fn call(
    worker_id: &WorkerId,
    query: &str,
    store: &Arc<SessionStore>,
) -> (&'static str, Value) {
    let path = if query.is_empty() {
        format!("/api/v1/workers/{worker_id}/activity")
    } else {
        format!("/api/v1/workers/{worker_id}/activity?{query}")
    };
    let req = format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n");
    let (status, body) = list_response(&worker_id.to_string(), &req, Some(store)).await;
    let v: Value = serde_json::from_str(&body).expect("json body");
    (status, v)
}

/// Default render hides the 3 shapes of system-meta noise (real SPA
/// self-ping, `worker.kpi_rolled` cron, audit-page click telemetry) but
/// keeps the actual agent turn. `?include_meta=1` restores all 4 rows
/// so operators can still get the forensic view.
#[tokio::test]
async fn default_hides_system_meta_include_meta_restores_them() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Mariah").await;

    // Real agent turn — explicit non-metadata verb survives the filter.
    seed_kind(
        &store,
        AuditKind::AssistantMessage,
        json!({
            "worker_id": worker.id.to_string(),
            "action_type": "message.sent",
            "verb": "wrote reply",
            "outcome_status": "succeeded",
        }),
    )
    .await;
    // SPA self-ping into the audit page: kind=HttpRequest with the
    // gateway endpoint encoded in payload.endpoint, no payload.verb.
    seed_kind(
        &store,
        AuditKind::HttpRequest,
        json!({
            "worker_id": worker.id.to_string(),
            "endpoint": "audit.detail.viewed",
        }),
    )
    .await;
    // Worker activity self-view → same shape as above, different endpoint.
    seed_kind(
        &store,
        AuditKind::HttpRequest,
        json!({
            "worker_id": worker.id.to_string(),
            "endpoint": "worker.activity.viewed",
        }),
    )
    .await;
    // Periodic KPI roll-up cron — kind alone identifies it.
    seed_kind(
        &store,
        AuditKind::WorkerKpiRolled,
        json!({
            "worker_id": worker.id.to_string(),
        }),
    )
    .await;

    let (status, body) = call(&worker.id, "", &store).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "default feed should drop system-meta rows");
    let verbs: Vec<&str> = items.iter().filter_map(|i| i["verb"].as_str()).collect();
    assert_eq!(verbs, vec!["wrote reply"]);

    // `include_meta=1` must restore every system-meta row the default
    // view hid. We assert PRESENCE of each seeded row rather than an
    // exact count: the activity endpoint self-audits a
    // `worker.activity.viewed` HttpRequest row on every call (see
    // `emit_view_audit`), so the prior default call above injected one
    // such row — a legitimate system-meta row that `include_meta=1`
    // (the forensic view) correctly surfaces. An exact-count assertion
    // would be coupled to that instrumentation; presence checks pin the
    // real contract.
    let (_, body) = call(&worker.id, "include_meta=1", &store).await;
    let items = body["items"].as_array().expect("items");
    // `action_type` falls back to `endpoint` then `kind`, so each
    // seeded row is identifiable by these stable values.
    let action_types: Vec<&str> = items
        .iter()
        .filter_map(|i| i["action_type"].as_str())
        .collect();
    let verbs: Vec<&str> = items.iter().filter_map(|i| i["verb"].as_str()).collect();
    assert!(verbs.contains(&"wrote reply"), "agent turn restored: {items:?}");
    assert!(
        action_types.contains(&"audit.detail.viewed"),
        "audit-page self-ping restored: {items:?}"
    );
    assert!(
        action_types.contains(&"worker.activity.viewed"),
        "activity self-view restored: {items:?}"
    );
    assert!(
        action_types.contains(&"worker.kpi_rolled"),
        "kpi-roll cron restored: {items:?}"
    );
    assert!(
        items.len() >= 4,
        "include_meta=1 must restore every system-meta row (>=4 seeded), got {}",
        items.len()
    );
}

/// Negative pin: a row whose `payload.action_type` happens to be a
/// metadata value is also hidden, even when `kind` is benign. Catches
/// the case where a gateway route encodes its specific surface in
/// `action_type` rather than `endpoint`.
#[tokio::test]
async fn action_type_metadata_value_is_also_hidden_by_default() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Mariah").await;

    seed_kind(
        &store,
        AuditKind::HttpRequest,
        json!({
            "worker_id": worker.id.to_string(),
            "action_type": "audit.list.viewed",
        }),
    )
    .await;
    let (_, body) = call(&worker.id, "", &store).await;
    assert!(
        body["items"].as_array().unwrap().is_empty(),
        "row with action_type=audit.list.viewed should be filtered"
    );
    // `include_meta=1` restores the hidden row. Assert PRESENCE rather
    // than an exact count: the prior default call self-audited a
    // `worker.activity.viewed` HttpRequest row (see `emit_view_audit`),
    // which the forensic view also surfaces. The contract under test is
    // that the `audit.list.viewed` row is no longer filtered.
    let (_, body) = call(&worker.id, "include_meta=1", &store).await;
    let items = body["items"].as_array().unwrap();
    let action_types: Vec<&str> = items
        .iter()
        .filter_map(|i| i["action_type"].as_str())
        .collect();
    assert!(
        action_types.contains(&"audit.list.viewed"),
        "include_meta=1 must restore the action_type=audit.list.viewed row: {items:?}"
    );
}

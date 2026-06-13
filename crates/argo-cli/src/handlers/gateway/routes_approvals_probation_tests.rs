//! Tests for the probation-hold approvals bridge
//! (`routes_approvals_probation`).
//!
//! Two layers:
//!
//! 1. Pure unit tests of the id round-trip (`parse_phold_id`) and the
//!    card adapter (`build_hold_row`) — no DB, no cwd.
//! 2. End-to-end tests that write real `probation.hold` audit rows to the
//!    cwd-bound workspace tenant and assert they surface through the
//!    public `GET /api/v1/approvals` handler the SPA consumes, then clear
//!    once a resolution marker (the approve / refuse decision) lands.

use super::{build_hold_row, parse_phold_id, HoldRow};

fn hold(session: &str, tool: &str) -> HoldRow {
    HoldRow {
        session_id: session.to_string(),
        tool: tool.to_string(),
        created_at: 1_700_000_000,
    }
}

#[test]
fn parse_phold_id_round_trips() {
    let id = "phold:11111111-1111-1111-1111-111111111111:file_read";
    let (session, tool) = parse_phold_id(id).expect("valid phold id");
    assert_eq!(session, "11111111-1111-1111-1111-111111111111");
    assert_eq!(tool, "file_read");
}

#[test]
fn parse_phold_id_rejects_non_phold_and_malformed() {
    assert!(parse_phold_id("act-1234").is_none(), "non-phold id");
    assert!(parse_phold_id("phold:").is_none(), "empty body");
    assert!(parse_phold_id("phold:sessiononly").is_none(), "no tool sep");
    assert!(parse_phold_id("phold::file_read").is_none(), "empty session");
    assert!(parse_phold_id("phold:sess:").is_none(), "empty tool");
}

#[test]
fn build_hold_row_shapes_the_card() {
    let row = build_hold_row(&hold("sess-1", "web_search"), None);
    assert_eq!(row.json["id"], "phold:sess-1:web_search");
    assert_eq!(row.json["verb"], "wants to use web_search");
    assert_eq!(row.json["channel"], "web");
    assert_eq!(row.json["decision"], serde_json::Value::Null);
    // P2-low band per the brief (routine onboarding, not high-stakes).
    assert_eq!(row.json["risk_level"], "low");
    assert_eq!(row.json["severity"], "p2");
    // No worker resolved → placeholder, not a crash or "unknown" name.
    assert_eq!(row.json["worker"]["name"], "New teammate");
    assert_eq!(row.json["worker"]["id"], "unknown");
}

#[test]
fn build_hold_row_uses_resolved_worker_name() {
    let worker = Some((argo_identity::WorkerId::nil(), "Mariah".to_string()));
    let row = build_hold_row(&hold("sess-2", "send_email"), worker);
    assert_eq!(row.json["worker"]["name"], "Mariah");
    assert_ne!(row.json["worker"]["id"], "unknown");
}

// --- End-to-end through GET /api/v1/approvals --------------------------

use argo_audit::{AuditKind, AuditSink};
use argo_types::SessionId;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::handlers::worker_actions_transitions::open_workspace_memory_with_store;

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

fn auth_get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nAuthorization: Bearer test\r\n\r\n")
}

/// Resolve the same cwd-bound workspace tenant + store that `list_response`
/// reads back, and return the tenant text the AuditSink stamps.
async fn store_and_tenant() -> (Arc<argo_store::SessionStore>, String) {
    let (_memory, _ws, store) = open_workspace_memory_with_store()
        .await
        .expect("open workspace store");
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.expect("tenant");
    (store, tenant.to_string())
}

/// Write one `probation.hold` audit row (the honest source the dispatch
/// gate writes), flushed so the read sees it.
async fn write_hold(store: &Arc<argo_store::SessionStore>, tenant: &str, session: Uuid, tool: &str) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        Some(SessionId(session)),
        AuditKind::ProbationHold,
        json!({ "tool": tool, "reason": "probation_window_active" }),
        None,
    )
    .await
    .expect("write hold");
    sink.flush_pending().await.expect("flush hold");
}

async fn list_items(store: &Arc<argo_store::SessionStore>) -> Vec<Value> {
    let (status, body) = super::super::routes_approvals_json::list_response(
        &auth_get("/api/v1/approvals"),
        Some(store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    v["items"].as_array().cloned().unwrap_or_default()
}

fn phold_row<'a>(items: &'a [Value], tool: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|r| r["verb"] == format!("wants to use {tool}"))
}

#[tokio::test]
async fn held_tool_surfaces_in_inbox_and_clears_on_resolution() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());

    let (store, tenant) = store_and_tenant().await;
    let session = Uuid::now_v7();
    write_hold(&store, &tenant, session, "file_read").await;

    // The broken promise: the hold MUST now be a pending inbox row.
    let items = list_items(&store).await;
    let row = phold_row(&items, "file_read").unwrap_or_else(|| {
        panic!("held tool did not surface in /approvals: {items:?}")
    });
    let id = row["id"].as_str().expect("id");
    assert!(id.starts_with("phold:"), "id: {id}");
    assert_eq!(row["decision"], Value::Null, "must be pending");

    // Resolution marker (the approve decision's audit row) clears it.
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.clone());
    sink.write(
        None,
        AuditKind::ApprovalGranted,
        json!({ "probation_hold": true, "session_id": session.to_string(), "tool": "file_read" }),
        None,
    )
    .await
    .expect("write resolution");
    sink.flush_pending().await.expect("flush resolution");

    let after = list_items(&store).await;
    assert!(
        phold_row(&after, "file_read").is_none(),
        "resolved hold must drop out of the inbox: {after:?}"
    );
}

#[tokio::test]
async fn repeated_holds_for_same_tool_collapse_to_one_row() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());

    let (store, tenant) = store_and_tenant().await;
    let session = Uuid::now_v7();
    // The gate writes a hold every dispatch attempt — the inbox must not
    // show one row per attempt for the same (session, tool).
    write_hold(&store, &tenant, session, "web_search").await;
    write_hold(&store, &tenant, session, "web_search").await;
    write_hold(&store, &tenant, session, "web_search").await;

    let items = list_items(&store).await;
    let count = items
        .iter()
        .filter(|r| r["verb"] == "wants to use web_search")
        .count();
    assert_eq!(count, 1, "deduped to one row, got {count}: {items:?}");
}

#[tokio::test]
async fn refuse_records_decline_and_clears_the_row() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());

    let (store, tenant) = store_and_tenant().await;
    let session = Uuid::now_v7();
    write_hold(&store, &tenant, session, "delete_file").await;

    // Refuse through the real decide handler (the same path the dispatcher
    // routes a `phold:` decide to).
    let approval_id = format!("phold:{session}:delete_file");
    let req = format!(
        "POST /api/v1/approvals/{approval_id}/decide HTTP/1.1\r\n\
         Authorization: Bearer test\r\n\r\n{{\"decision\":\"refuse\"}}"
    );
    let (status, body) = super::decide_probation_hold(&approval_id, "refuse", &req).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["decision"], "refuse");

    let after = list_items(&store).await;
    assert!(
        phold_row(&after, "delete_file").is_none(),
        "refused hold must drop out of the inbox: {after:?}"
    );
}

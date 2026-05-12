//! Unit tests for `routes_approvals_json`. Models the webinbox-test
//! shape: tempdir-backed ARGO_HOME, hand-rolled `ActionRecord` memory
//! rows, then driving the public handlers.

use super::{decide_response, list_response, undo_response};
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_transitions::open_workspace_memory;
use argo_memory::MemoryFile;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::{Mutex, MutexGuard};

// The test environment globals (`ARGO_HOME`, `ARGO_VERTICALS_DIR`) are
// process-wide. Serialize the tests on a tokio mutex so the guard can
// be held across `.await` points (clippy::await_holding_lock).
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
    // ARGO_HOME isolates the SQLite store per tempdir; the workspace
    // is keyed off (cwd_basename, cwd) inside ensure_workspace, but
    // since each test gets a fresh DB the workspace row is always
    // freshly created — no cross-test bleed.
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

fn pending_record(id: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: id.into(),
        job_id: "job-1".into(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "post_listing".into(),
        risk_level: "high".into(),
        requires_approval: true,
        state: "pending".into(),
        payload: serde_json::json!({"summary": format!("test action {id}")}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn seed_action(action_id: &str) {
    let (memory, ws) = open_workspace_memory().await.expect("open memory");
    let rec = pending_record(action_id);
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            serde_json::to_string(&rec).expect("ser"),
            None,
            "gtm-engineer",
        )
        .await
        .expect("append memory row");
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).expect("response is JSON")
}

fn auth_post(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nAuthorization: Bearer test\r\nContent-Length: {len}\r\n\r\n{body}",
        len = body.len()
    )
}

fn auth_get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nAuthorization: Bearer test\r\n\r\n")
}

#[tokio::test]
async fn list_returns_pending_items_and_envelope() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-1").await;
    let (status, body) = list_response(&auth_get("/api/v1/approvals")).await;
    assert_eq!(status, "200 OK");
    let v = parse(&body);
    let items = v["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "act-1");
    assert!(items[0]["decision"].is_null());
    assert!(v.get("next_cursor").is_none(), "no cursor on single page");
}

#[tokio::test]
async fn list_cursor_paginates() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    for i in 0..3 {
        seed_action(&format!("act-{i}")).await;
        // 1-ms gap so created_at strings sort deterministically.
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let (_, body) = list_response(&auth_get("/api/v1/approvals?limit=2")).await;
    let v = parse(&body);
    let cursor = v["next_cursor"].as_str().expect("cursor present");
    let (_, body2) = list_response(&auth_get(&format!(
        "/api/v1/approvals?limit=2&cursor={cursor}"
    )))
    .await;
    let v2 = parse(&body2);
    let items2 = v2["items"].as_array().expect("items");
    assert_eq!(items2.len(), 1);
    // Cursor resume returns rows not on the prior page.
    let first_ids: Vec<&str> = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(!first_ids.contains(&items2[0]["id"].as_str().unwrap()));
}

#[tokio::test]
async fn list_limit_max_100_is_accepted() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-1").await;
    let (status, _) = list_response(&auth_get("/api/v1/approvals?limit=100")).await;
    assert_eq!(status, "200 OK");
}

#[tokio::test]
async fn list_limit_above_max_is_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let (status, body) = list_response(&auth_get("/api/v1/approvals?limit=200")).await;
    assert_eq!(status, "400 Bad Request");
    let v = parse(&body);
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn list_only_workspaces_own_rows_returned() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // Only seed one row in this workspace; a second tempdir would
    // be a distinct workspace (different `ensure_workspace` row) and
    // its rows would not appear.
    seed_action("act-mine").await;
    let other = tempfile::tempdir().expect("other tempdir");
    set_test_env(other.path());
    seed_action("act-theirs").await;
    let (_, body) = list_response(&auth_get("/api/v1/approvals")).await;
    let ids: Vec<String> = parse(&body)["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"act-theirs".to_string()));
    assert!(!ids.contains(&"act-mine".to_string()));
}

#[tokio::test]
async fn decide_approve_transitions_state() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-approve").await;
    let (status, body) = decide_response(
        "act-approve",
        &auth_post("/api/v1/approvals/act-approve/decide", r#"{"decision":"approve"}"#),
    )
    .await;
    assert_eq!(status, "200 OK");
    let v = parse(&body);
    assert_eq!(v["ok"], true);
    assert_eq!(v["decision"], "approve");
    // After approve, the action is no longer pending → list is empty.
    let (_, body) = list_response(&auth_get("/api/v1/approvals")).await;
    assert_eq!(parse(&body)["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn decide_refuse_transitions_state() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-refuse").await;
    let (status, body) = decide_response(
        "act-refuse",
        &auth_post("/api/v1/approvals/act-refuse/decide", r#"{"decision":"refuse"}"#),
    )
    .await;
    assert_eq!(status, "200 OK");
    assert_eq!(parse(&body)["decision"], "refuse");
    let (_, body) = list_response(&auth_get("/api/v1/approvals")).await;
    assert_eq!(parse(&body)["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn decide_already_decided_with_different_decision_is_409() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-conflict").await;
    let _ = decide_response(
        "act-conflict",
        &auth_post(
            "/api/v1/approvals/act-conflict/decide",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    let (status, body) = decide_response(
        "act-conflict",
        &auth_post(
            "/api/v1/approvals/act-conflict/decide",
            r#"{"decision":"refuse"}"#,
        ),
    )
    .await;
    assert_eq!(status, "409 Conflict");
    assert_eq!(parse(&body)["error"]["code"], "already_decided");
}

#[tokio::test]
async fn decide_same_decision_twice_is_idempotent_200() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-idem").await;
    let req = auth_post(
        "/api/v1/approvals/act-idem/decide",
        r#"{"decision":"approve"}"#,
    );
    let (s1, _) = decide_response("act-idem", &req).await;
    let (s2, b2) = decide_response("act-idem", &req).await;
    assert_eq!(s1, "200 OK");
    assert_eq!(s2, "200 OK");
    assert_eq!(parse(&b2)["decision"], "approve");
}

#[tokio::test]
async fn undo_within_window_restores_to_pending() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-undo").await;
    let _ = decide_response(
        "act-undo",
        &auth_post(
            "/api/v1/approvals/act-undo/decide",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    let (status, body) = undo_response(
        "act-undo",
        &auth_post("/api/v1/approvals/act-undo/undo", ""),
    )
    .await;
    assert_eq!(status, "200 OK");
    assert_eq!(parse(&body)["ok"], true);
    // After undo the action is pending again.
    let (_, body) = list_response(&auth_get("/api/v1/approvals")).await;
    assert_eq!(parse(&body)["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn undo_outside_window_returns_410() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // Emulate the outside-window case by appending a single decided
    // row whose `created_at` is 60s in the past — `find_action`
    // returns the row with the latest `created_at`, so we omit the
    // prior `pending` seed (its `created_at = now` would shadow the
    // backdated decided row and short-circuit undo to "not decided").
    let (memory, ws) = open_workspace_memory().await.expect("memory");
    let mut decided = pending_record("act-old");
    decided.state = "approved".into();
    decided.approved_by = Some("gateway".into());
    decided.approved_at = Some(chrono::Utc::now().to_rfc3339());
    decided.created_at = (chrono::Utc::now() - chrono::Duration::seconds(60)).to_rfc3339();
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            serde_json::to_string(&decided).unwrap(),
            None,
            "gtm-engineer",
        )
        .await
        .unwrap();
    let (status, body) = undo_response(
        "act-old",
        &auth_post("/api/v1/approvals/act-old/undo", ""),
    )
    .await;
    assert_eq!(status, "410 Gone");
    assert_eq!(parse(&body)["error"]["code"], "undo_window_expired");
}

#[tokio::test]
async fn decide_writes_an_audit_memory_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-audit").await;
    let (memory, ws) = open_workspace_memory().await.expect("memory");
    let before = memory
        .list_for_vertical(ws, MemoryFile::Memory, "gtm-engineer")
        .await
        .unwrap()
        .len();
    let _ = decide_response(
        "act-audit",
        &auth_post(
            "/api/v1/approvals/act-audit/decide",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    let after = memory
        .list_for_vertical(ws, MemoryFile::Memory, "gtm-engineer")
        .await
        .unwrap()
        .len();
    assert!(after > before, "decide must append an audit memory row");
}

#[tokio::test]
async fn decide_bad_json_is_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_action("act-bad").await;
    let (status, body) = decide_response(
        "act-bad",
        &auth_post("/api/v1/approvals/act-bad/decide", "{not json"),
    )
    .await;
    assert_eq!(status, "400 Bad Request");
    assert_eq!(parse(&body)["error"]["code"], "bad_request");
}

#[tokio::test]
async fn decide_unknown_id_is_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let (status, body) = decide_response(
        "act-ghost",
        &auth_post(
            "/api/v1/approvals/act-ghost/decide",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    assert_eq!(status, "404 Not Found");
    assert_eq!(parse(&body)["error"]["code"], "approval_not_found");
}

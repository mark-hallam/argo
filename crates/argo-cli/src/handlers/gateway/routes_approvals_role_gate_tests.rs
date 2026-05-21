//! Negative + positive integration tests for the
//! `mission-rbac-approvals-decide-role-gate` brief.
//!
//! Wave-10 §4 procurement risk #1: the
//! `POST /api/v1/approvals/<id>/decide` and `/undo` endpoints must
//! refuse callers below the `user` role floor, and each successful
//! decision must stamp the decider's resolved role into the audit
//! memory row so SOC2-style "who approved" trails are inspectable.
//!
//! These tests live in a sibling file so the parent
//! `routes_approvals_json_tests.rs` stays under the 500-LOC ceiling
//! while the role-gate fixtures (production-mode env lock + JWT
//! minting) can grow without crowding the existing decide/undo
//! state-machine tests.

use super::super::auth_magic_link::mint_jwt;
use super::{decider_role_from_record, decide_response, undo_response};
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_query::collect_latest_actions;
use crate::handlers::worker_actions_transitions::open_workspace_memory;
use argo_memory::MemoryFile;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::{Mutex, MutexGuard};

const PROD_PROFILE_ENV: &str = "ARGO_GATEWAY_PROFILE";
const JWT_SECRET_ENV: &str = "SKYCOT_AUTH_JWT_SECRET";
const TRUST_HEADER_ENV: &str = "ARGO_GATEWAY_TRUST_ROLE_HEADER";

/// Process-wide serialisation for tests that mutate ARGO_HOME /
/// production-mode env vars. Held across `.await` points (the action
/// memory store is async), so it must be a tokio Mutex.
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

fn lock_into_production_mode() {
    std::env::set_var(JWT_SECRET_ENV, "test-secret-for-decide-role-gate-tests");
    std::env::set_var(PROD_PROFILE_ENV, "production");
    std::env::set_var("ARGO_GATEWAY_BIND_NON_LOOPBACK", "1");
    std::env::set_var("ARGO_GATEWAY_BEARER_TOKEN", "test-bearer");
    std::env::remove_var(TRUST_HEADER_ENV);
}

fn unlock_production_mode() {
    std::env::remove_var(PROD_PROFILE_ENV);
    std::env::remove_var("ARGO_GATEWAY_BIND_NON_LOOPBACK");
    std::env::remove_var("ARGO_GATEWAY_BEARER_TOKEN");
    std::env::remove_var(TRUST_HEADER_ENV);
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
        payload: serde_json::json!({"summary": format!("rbac fixture {id}")}),
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

async fn seed_pending(id: &str) {
    let (memory, ws) = open_workspace_memory().await.expect("open memory");
    let rec = pending_record(id);
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            serde_json::to_string(&rec).expect("serialise"),
            None,
            "gtm-engineer",
        )
        .await
        .expect("append memory row");
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).expect("response is JSON")
}

fn jwt_post(path: &str, role: &str, body: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt(
        "u-test",
        "alice@example.com",
        Some("tenant-a"),
        Some(role),
        now,
    )
    .expect("mint jwt");
    format!(
        "POST {path} HTTP/1.1\r\nAuthorization: Bearer {jwt}\r\nContent-Length: {len}\r\n\r\n{body}",
        len = body.len()
    )
}

async fn latest_action_record(action_id: &str) -> ActionRecord {
    let rows = collect_latest_actions(None).await.expect("collect actions");
    rows.into_iter()
        .find(|r| r.action_id == action_id)
        .unwrap_or_else(|| panic!("no row for {action_id}"))
}

#[tokio::test]
async fn decide_blocks_guest_role_with_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("act-guest").await;
    lock_into_production_mode();
    let (status, body) = decide_response(
        "act-guest",
        &jwt_post(
            "/api/v1/approvals/act-guest/decide",
            "guest",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    unlock_production_mode();
    assert_eq!(status, "403 Forbidden");
    let v = parse(&body);
    assert_eq!(v["error"]["code"], "forbidden");
    assert_eq!(v["rbac"]["action"], "approval.decide");
    assert_eq!(v["rbac"]["required"], "user");
    assert_eq!(v["rbac"]["effective"], "guest");
    // Negative-test: the audit memory row must NOT have a new
    // approval/refuse transition — the gate short-circuited before
    // the transition was written.
    let rec = latest_action_record("act-guest").await;
    assert_eq!(rec.state, "pending", "guest deny must not transition state");
}

#[tokio::test]
async fn decide_admin_role_succeeds_and_stamps_decider_role_into_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("act-admin").await;
    lock_into_production_mode();
    let (status, _body) = decide_response(
        "act-admin",
        &jwt_post(
            "/api/v1/approvals/act-admin/decide",
            "admin",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    unlock_production_mode();
    assert_eq!(status, "200 OK");
    let rec = latest_action_record("act-admin").await;
    assert_eq!(rec.state, "approved");
    assert_eq!(
        decider_role_from_record(&rec).as_deref(),
        Some("admin"),
        "audit row must stamp the decider's role; got result={:?}",
        rec.result
    );
}

#[tokio::test]
async fn decide_user_role_succeeds_at_floor() {
    // `user` is the minimum role permitted to approve — one notch
    // above `guest`. Lock-step verification that the floor is `>=`,
    // not `>`.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("act-user").await;
    lock_into_production_mode();
    let (status, _body) = decide_response(
        "act-user",
        &jwt_post(
            "/api/v1/approvals/act-user/decide",
            "user",
            r#"{"decision":"approve"}"#,
        ),
    )
    .await;
    unlock_production_mode();
    assert_eq!(status, "200 OK");
    let rec = latest_action_record("act-user").await;
    assert_eq!(decider_role_from_record(&rec).as_deref(), Some("user"));
}

#[tokio::test]
async fn decide_refuse_stamps_decider_role_into_cancellation_note() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("act-refuse-admin").await;
    lock_into_production_mode();
    let (status, body) = decide_response(
        "act-refuse-admin",
        &jwt_post(
            "/api/v1/approvals/act-refuse-admin/decide",
            "admin",
            r#"{"decision":"refuse"}"#,
        ),
    )
    .await;
    unlock_production_mode();
    assert_eq!(status, "200 OK");
    assert_eq!(parse(&body)["decision"], "refuse");
    let rec = latest_action_record("act-refuse-admin").await;
    assert_eq!(rec.state, "cancelled");
    assert_eq!(
        decider_role_from_record(&rec).as_deref(),
        Some("admin"),
        "refuse audit row must stamp decider_role; got result={:?}",
        rec.result
    );
}

#[tokio::test]
async fn undo_blocks_guest_role_with_403() {
    // Sister gate to `decide` — undo carries the same risk shape, so
    // guest must be denied with 403.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("act-undo-guest").await;
    lock_into_production_mode();
    let (status, body) = undo_response(
        "act-undo-guest",
        &jwt_post("/api/v1/approvals/act-undo-guest/undo", "guest", ""),
    )
    .await;
    unlock_production_mode();
    assert_eq!(status, "403 Forbidden");
    assert_eq!(parse(&body)["rbac"]["action"], "approval.undo");
}

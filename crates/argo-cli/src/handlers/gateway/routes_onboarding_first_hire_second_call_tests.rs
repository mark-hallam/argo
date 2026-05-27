//! Regression test for
//! `fix-gateway-first-hire-second-call-ghost-worker-not-persisted`.
//!
//! Pre-fix, `POST /api/v1/onboarding/first-hire` wrote the worker row
//! to the user-account-bound tenant returned by `provision_for_user`,
//! while `/api/v1/team` and `/api/v1/workers/{id}/overview` resolved
//! tenant via `ensure_workspace(name, cwd)`. The two paths never
//! converged, so every hire returned `201 { worker_id }` but the row
//! was invisible to the rest of the gateway — the "ghost worker" bug.
//!
//! This module pre-provisions a single user, fires the hire endpoint
//! twice with two different descriptions, then asserts both worker_ids
//! are reachable via both read surfaces in the *same* test process.
//! Pre-fix this fails on the very first overview probe (404); post-fix
//! both calls land in the CWD-derived tenant the reads query.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::first_hire_response;
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
    std::env::set_var(
        argo_credentials::WORKSPACE_ROOT_KEY_ENV,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn post_req(path: &str, body: &str, principal: &str) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {principal}\r\n"));
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

fn get_req(path: &str, principal: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: {principal}\r\n\r\n")
}

async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
    let body = json!({ "workspace_name": "Acme", "company_name": "Acme Co" }).to_string();
    let r = post_req(
        "/api/v1/onboarding/provision",
        &body,
        &format!("human:{email}"),
    );
    let (status, body) = provision_response(&r, Some(store)).await;
    assert!(
        status.starts_with("201") || status.starts_with("200"),
        "pre-provision failed: {status} {body}"
    );
}

async fn hire(store: &Arc<SessionStore>, email: &str, description: &str) -> String {
    let body = json!({
        "description": description,
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = post_req(
        "/api/v1/onboarding/first-hire",
        &body,
        &format!("human:{email}"),
    );
    let (status, resp) = first_hire_response(&r, Some(store)).await;
    assert_eq!(status, "201 Created", "hire body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    v["worker_id"]
        .as_str()
        .expect("worker_id present")
        .to_string()
}

#[tokio::test]
async fn second_first_hire_call_is_visible_via_team_and_overview() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;

    let first_id = hire(
        &store,
        "alice@acme.com",
        "Sara, you draft outbound nurture emails for new signups.",
    )
    .await;
    let second_id = hire(
        &store,
        "alice@acme.com",
        "Bryn, you triage inbound support tickets and escalate hot ones.",
    )
    .await;
    assert_ne!(first_id, second_id, "each hire must mint a distinct id");

    // /api/v1/team must surface both hires.
    let (status, team_body) = super::super::routes_team_list::list_response(
        &get_req("/api/v1/team", "human:alice@acme.com"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "team body: {team_body}");
    assert!(
        team_body.contains(&first_id),
        "team is missing the first hire id {first_id}: {team_body}"
    );
    assert!(
        team_body.contains(&second_id),
        "team is missing the second hire id {second_id}: {team_body}"
    );

    // /api/v1/workers/{id}/overview must resolve both hires (not 404).
    for wid in [&first_id, &second_id] {
        let path = format!("/api/v1/workers/{wid}/overview");
        let (status, body) = super::super::routes_worker_overview::show_response(
            wid,
            &get_req(&path, "human:alice@acme.com"),
            Some(&store),
        )
        .await;
        assert_eq!(
            status, "200 OK",
            "overview for {wid} returned non-200: {body}"
        );
    }
}

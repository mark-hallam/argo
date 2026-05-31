//! Tests for the seeded-approval bridge (`routes_approvals_json_seeded`).
//!
//! Two layers:
//!
//! 1. Pure unit tests of the card adapter (`build_row` / `approval_verb`)
//!    — no DB, no cwd. These pin the `requested_by` → `worker.name`
//!    mapping and the grammatical `verb` default.
//! 2. One end-to-end test that writes a real Marcus-authored
//!    `approval_request` row to the cwd-bound workspace tenant and asserts
//!    it surfaces as `worker.name == "Marcus"` through the public
//!    `GET /api/v1/approvals` JSON (the same handler the SPA consumes).

use super::{approval_verb, build_row, verb_from_action, ApprovalRow};
use serde_json::{json, Value};

fn row(requested_by: &str) -> ApprovalRow {
    ApprovalRow {
        id: "00000000-0000-0000-0000-000000000001".to_string(),
        requested_at_micros: 1_700_000_000_000_000,
        status: "pending".to_string(),
        requested_by: requested_by.to_string(),
        draft_content: Some("draft body".to_string()),
    }
}

#[test]
fn requested_by_becomes_worker_name_when_no_persona_resolved() {
    // No `approval.asked` worker persona (the marketing-draft case):
    // the `requested_by` column ("Marcus") must drive `worker.name`.
    let built = build_row(&row("Marcus"), None, None);
    assert_eq!(built.json["worker"]["name"], "Marcus");
    // No real WorkerId was resolved — id stays the "unknown" sentinel.
    assert_eq!(built.json["worker"]["id"], "unknown");
}

#[test]
fn blank_requested_by_falls_back_to_pending_teammate() {
    // The placeholder is used ONLY when there is no name at all.
    assert_eq!(
        build_row(&row(""), None, None).json["worker"]["name"],
        "Pending teammate"
    );
    assert_eq!(
        build_row(&row("   "), None, None).json["worker"]["name"],
        "Pending teammate"
    );
}

#[test]
fn resolved_persona_wins_over_requested_by() {
    // When the audit payload resolved a real worker persona, that name
    // wins over the free-text `requested_by` column.
    let worker = Some((argo_identity::WorkerId::nil(), "Mariah".to_string()));
    let built = build_row(&row("Marcus"), None, worker);
    assert_eq!(built.json["worker"]["name"], "Mariah");
}

#[test]
fn verb_default_is_grammatical() {
    // The old default was the ungrammatical "wants to an action".
    assert_eq!(approval_verb(None), "wants to take an action");
    let built = build_row(&row("Marcus"), None, None);
    assert_eq!(built.json["verb"], "wants to take an action");
}

#[test]
fn verb_derived_from_marketing_action() {
    let ask = json!({ "action": "marketing.response_drafted" });
    assert_eq!(
        approval_verb(Some(&ask)),
        "wants to draft a marketing response"
    );
}

#[test]
fn verb_explicit_label_takes_precedence_over_action() {
    let ask = json!({ "label": "post the listing", "action": "marketing.response_drafted" });
    assert_eq!(approval_verb(Some(&ask)), "wants to post the listing");
}

#[test]
fn verb_unknown_action_degrades_readably() {
    // An unrecognised slug becomes a readable phrase, not the default.
    assert_eq!(
        verb_from_action("listing.published"),
        "wants to listing published"
    );
}

// --- End-to-end through GET /api/v1/approvals ---------------------------

use crate::handlers::worker_actions_transitions::open_workspace_memory_with_store;
use argo_store::approvals::{create_request, DefaultDecision, NewApprovalRequest};
use std::path::PathBuf;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

// `ARGO_HOME` / `ARGO_VERTICALS_DIR` are process-wide; serialize on a
// tokio mutex held across `.await` (clippy::await_holding_lock), exactly
// like `routes_approvals_json_tests.rs`.
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

fn auth_get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nAuthorization: Bearer test\r\n\r\n")
}

#[tokio::test]
async fn marcus_authored_request_surfaces_as_worker_name_marcus() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());

    // Resolve the same cwd-bound workspace tenant + store that the
    // seeded arm of `list_response` will read back. Both paths open the
    // SQLite DB at `db_path(resolve_home(cwd))`, so they share one file.
    let (_memory, _ws, store) = open_workspace_memory_with_store()
        .await
        .expect("open workspace store");
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.expect("tenant");

    // Marcus files a marketing draft into the first-class approval store
    // — the exact shape `routes_marketing_draft_response.rs` writes.
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: "Marcus",
            deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
            default_decision: DefaultDecision::Deny,
            draft_content: Some("Marcus drafted an x reply post (Allow) — review before you post it."),
        },
    )
    .await
    .expect("create approval_request");

    // Read it back through the public GET JSON handler the SPA consumes.
    let (status, body) = super::super::routes_approvals_json::list_response(
        &auth_get("/api/v1/approvals"),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let items = v["items"].as_array().expect("items array");
    let marcus = items
        .iter()
        .find(|r| r["worker"]["name"] == "Marcus")
        .unwrap_or_else(|| panic!("no Marcus row in {body}"));

    // The fix: name is "Marcus" (not "Pending teammate") and the verb is
    // grammatical (no "wants to an action").
    assert_eq!(marcus["worker"]["name"], "Marcus");
    let verb = marcus["verb"].as_str().expect("verb string");
    assert_ne!(verb, "wants to an action", "ungrammatical default leaked");
    assert!(verb.starts_with("wants to "), "verb: {verb}");
}

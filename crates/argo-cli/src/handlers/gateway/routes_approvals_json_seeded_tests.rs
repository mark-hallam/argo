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

fn row_with_status(status: &str) -> ApprovalRow {
    ApprovalRow {
        status: status.to_string(),
        ..row("Mariah")
    }
}

#[test]
fn granted_status_maps_to_approve_decision() {
    // Regression for the /approvals "Approval rate 0%" bug: the store
    // serialises an APPROVED row as "granted" (ApprovalDecision::
    // as_store_str), but the adapter only matched "approved", so every
    // approved seeded row fell to a null decision and the rate ring read
    // 0% even with resolved rows. "granted" must surface as "approve".
    assert_eq!(
        build_row(&row_with_status("granted"), None, None).json["decision"],
        "approve"
    );
}

#[test]
fn denied_and_expired_statuses_map_to_refuse_decision() {
    for status in ["denied", "refused", "expired"] {
        assert_eq!(
            build_row(&row_with_status(status), None, None).json["decision"],
            "refuse",
            "status {status} should map to refuse"
        );
    }
}

#[test]
fn pending_status_has_null_decision() {
    // A still-pending row must NOT be counted as resolved.
    assert_eq!(
        build_row(&row_with_status("pending"), None, None).json["decision"],
        Value::Null
    );
}

#[test]
fn legacy_approved_token_still_maps_to_approve() {
    // Defence-in-depth: if a writer ever stored the token string
    // "approved" instead of the canonical "granted", it still resolves.
    assert_eq!(
        build_row(&row_with_status("approved"), None, None).json["decision"],
        "approve"
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
use uuid::Uuid;

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
    let _g = crate::handlers::gateway::shared_env_lock().await;
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

    // Read it back through the public GET JSON handler the SPA consumes,
    // passing the SAME already-open store the gateway threads into the
    // route (the race fix: the seeded arm reads through this shared
    // handle instead of opening + re-migrating its own).
    let (status, body) = super::super::routes_approvals_json::list_response(
        &auth_get("/api/v1/approvals"),
        Some(&store),
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

#[tokio::test]
async fn seeded_read_uses_passed_handle_not_a_reopen() {
    // Race-fix regression: the seeded arm must read through the
    // already-open store handle the gateway threads in, NOT open a
    // second `SessionStore` (and re-run `migrate()`) per request. We
    // prove this by writing the row to one store/ARGO_HOME, then
    // repointing ARGO_HOME at a DIFFERENT empty dir before the read but
    // still passing the ORIGINAL handle. If the read read via the passed
    // handle it finds the row; if it (regressed to) re-opening via
    // ARGO_HOME it would open the empty second DB and find nothing.
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let home_a = tempfile::tempdir().expect("home_a");
    set_test_env(home_a.path());

    let (_memory, _ws, store) = open_workspace_memory_with_store()
        .await
        .expect("open store A");
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.expect("tenant A");
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: "Mariah",
            deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
            default_decision: DefaultDecision::Deny,
            draft_content: Some("draft body"),
        },
    )
    .await
    .expect("create approval_request");

    // Repoint ARGO_HOME at a fresh EMPTY home. A re-open would land here
    // (empty), a passed-handle read would not.
    let home_b = tempfile::tempdir().expect("home_b");
    set_test_env(home_b.path());

    let rows = super::fetch_seeded_approval_rows(&auth_get("/api/v1/approvals"), &store).await;
    assert_eq!(
        rows.len(),
        1,
        "seeded read must surface the row via the PASSED store handle, \
         not a re-open of the (now-empty) ARGO_HOME DB"
    );
    assert_eq!(rows[0].json["worker"]["name"], "Mariah");
}

#[tokio::test]
async fn seeded_read_does_not_remigrate_passed_store() {
    // The gateway migrates once at boot; the seeded read must not bump
    // the schema version on the passed handle. Capture the version
    // before/after a read and assert it is unchanged — re-running
    // `migrate()` per request was the pointless work bundled with the
    // race.
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());

    let (_memory, _ws, store) = open_workspace_memory_with_store()
        .await
        .expect("open store");
    let before = store.schema_version().await.expect("schema version before");
    assert!(before > 0, "store should be migrated at this point");

    let _rows = super::fetch_seeded_approval_rows(&auth_get("/api/v1/approvals"), &store).await;

    let after = store.schema_version().await.expect("schema version after");
    assert_eq!(
        before, after,
        "seeded read must not change schema version (no per-request re-migrate)"
    );
}

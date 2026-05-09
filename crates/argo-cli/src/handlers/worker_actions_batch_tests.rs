//! Tests for `worker_actions_batch` — the helper module behind
//! `argo worker actions approve-batch`'s `--deny` / `--confirm` /
//! `--dry-run` flags (Stream `claude-approval-batch-decisions-cli`).

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::WorkerId;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use std::path::PathBuf;
use std::sync::Arc;

const FIXTURE_VERTICAL: &str = "gtm-engineer";

fn set_verticals_dir() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("workspace root");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root.join("verticals"));
}

async fn open_test_memory() -> (MemoryStore, WorkspaceId) {
    set_verticals_dir();
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (MemoryStore::new(store), workspace)
}

fn pending_record(action_id: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        worker_id: WorkerId::nil(),
        item_id: None,
        action_type: "post_listing".into(),
        risk_level: "medium".into(),
        requires_approval: true,
        state: "pending".to_string(),
        payload: serde_json::json!({}),
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

async fn seed(memory: &MemoryStore, ws: WorkspaceId, rec: &ActionRecord) {
    let body = serde_json::to_string(rec).unwrap();
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, FIXTURE_VERTICAL)
        .await
        .unwrap();
}

#[test]
fn batch_decision_round_trips_through_flag() {
    assert_eq!(BatchDecision::from_flag(false), BatchDecision::Approve);
    assert_eq!(BatchDecision::from_flag(true), BatchDecision::Deny);
    assert_eq!(BatchDecision::Approve.label(), "approve");
    assert_eq!(BatchDecision::Deny.label(), "deny");
}

/// Acceptance: `approve-batch a b c` approves all three when
/// every id is a pending approval-gated action. Returns three
/// successes and zero failures.
#[tokio::test]
async fn apply_decisions_approves_every_pending_id() {
    let (memory, ws) = open_test_memory().await;
    for id in ["a-1", "a-2", "a-3"] {
        seed(&memory, ws, &pending_record(id)).await;
    }
    let ids: Vec<String> = vec!["a-1".into(), "a-2".into(), "a-3".into()];
    let (ok, failed) = apply_decisions(
        &memory,
        ws,
        &ids,
        "alice",
        Some("bulk approve"),
        BatchDecision::Approve,
    )
    .await;
    assert_eq!(ok.len(), 3, "every pending id should approve");
    assert!(failed.is_empty(), "no failures expected");
    for r in &ok {
        assert_eq!(r.state, "approved");
        assert_eq!(r.approved_by.as_deref(), Some("alice"));
    }
}

/// Acceptance: missing / already-decided ids surface as failure
/// rows but do NOT abort the rest of the run. Run a 3-id batch
/// where only the middle id is seeded; the other two land in
/// `failed`.
#[tokio::test]
async fn apply_decisions_reports_missing_ids_without_aborting() {
    let (memory, ws) = open_test_memory().await;
    seed(&memory, ws, &pending_record("real")).await;
    let ids: Vec<String> = vec!["ghost-1".into(), "real".into(), "ghost-2".into()];
    let (ok, failed) = apply_decisions(
        &memory,
        ws,
        &ids,
        "ops",
        None,
        BatchDecision::Approve,
    )
    .await;
    assert_eq!(ok.len(), 1, "only the seeded id approves");
    assert_eq!(ok[0].action_id, "real");
    assert_eq!(failed.len(), 2, "the two ghost ids fail");
    let failed_ids: Vec<&str> = failed.iter().map(|(id, _)| id.as_str()).collect();
    assert!(failed_ids.contains(&"ghost-1"));
    assert!(failed_ids.contains(&"ghost-2"));
}

/// Acceptance: `--deny` cancels each pending action instead of
/// approving. Verify the resulting state is `cancelled` and the
/// approver was recorded into the cancellation payload.
#[tokio::test]
async fn apply_decisions_deny_cancels_each_pending_action() {
    let (memory, ws) = open_test_memory().await;
    for id in ["d-1", "d-2"] {
        seed(&memory, ws, &pending_record(id)).await;
    }
    let ids: Vec<String> = vec!["d-1".into(), "d-2".into()];
    let (ok, failed) = apply_decisions(
        &memory,
        ws,
        &ids,
        "ops",
        Some("not relevant"),
        BatchDecision::Deny,
    )
    .await;
    assert_eq!(ok.len(), 2);
    assert!(failed.is_empty());
    for r in &ok {
        assert_eq!(r.state, "cancelled");
        let payload = r.result.as_ref().expect("cancellation payload populated");
        let cancellation = payload.get("cancellation").expect("cancellation key");
        assert_eq!(
            cancellation.get("cancelled_by").and_then(|v| v.as_str()),
            Some("ops")
        );
    }
}

/// Acceptance: `--dry-run` writes nothing to the ledger. We seed
/// two pending ids and call `print_dry_run` directly; afterwards
/// the action records still read back as `pending`, proving no
/// transition was persisted.
#[tokio::test]
async fn dry_run_does_not_advance_action_state() {
    let (memory, ws) = open_test_memory().await;
    seed(&memory, ws, &pending_record("dr-1")).await;
    seed(&memory, ws, &pending_record("dr-2")).await;
    let ids: Vec<String> = vec!["dr-1".into(), "dr-2".into()];

    print_dry_run(&ids, BatchDecision::Approve, false).expect("dry-run prints cleanly");
    print_dry_run(&ids, BatchDecision::Deny, true).expect("json dry-run prints cleanly");

    // Re-read every persisted row and confirm the only state is
    // still `pending` — no `approved` / `cancelled` rows landed.
    let entries = memory
        .list_for_vertical(ws, MemoryFile::Memory, FIXTURE_VERTICAL)
        .await
        .unwrap();
    for e in entries {
        let rec: ActionRecord =
            serde_json::from_str(&e.body).expect("seeded record deserialises");
        assert_eq!(
            rec.state, "pending",
            "dry-run must not append any non-pending state row"
        );
    }
}

/// `prompt_confirms` MUST refuse to silently proceed when stdin
/// isn't a terminal. Tests run with stdin redirected — exercise
/// the guard path so a non-interactive launcher can't acidentally
/// apply a batch decision the operator didn't intend.
#[test]
fn prompt_confirms_refuses_non_interactive_stdin() {
    let err = prompt_confirms(3, BatchDecision::Approve)
        .expect_err("non-interactive stdin must error rather than silently accept");
    let msg = err.to_string();
    assert!(
        msg.contains("non-interactive") && msg.contains("--confirm"),
        "error must steer the operator to --confirm/--dry-run; got: {msg}"
    );
}

/// `print_summary` is the operator-visible output. Smoke-test
/// both render paths (text + json) on a representative mixed
/// result so a regression in formatting surfaces at test time.
#[test]
fn print_summary_renders_text_and_json_paths() {
    let ok = vec![ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: "ok-1".into(),
        job_id: "job-1".into(),
        worker_id: WorkerId::nil(),
        item_id: None,
        action_type: "post_listing".into(),
        risk_level: "medium".into(),
        requires_approval: true,
        state: "approved".into(),
        payload: serde_json::json!({}),
        approved_by: Some("ops".into()),
        approved_at: Some("2026-05-09T00:00:00Z".into()),
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: "2026-05-09T00:00:00Z".into(),
    }];
    let failed = vec![("missing".to_string(), "no action with that id".to_string())];
    print_summary(&ok, &failed, BatchDecision::Approve, false);
    print_summary(&ok, &failed, BatchDecision::Approve, true);
}

/// Help-smoke pin: the clap surface for `argo worker actions
/// approve-batch` accepts every documented flag and at least one
/// id. A regression that drops `--deny`, `--confirm`, or
/// `--dry-run` would surface here.
#[test]
fn approve_batch_clap_surface_accepts_all_documented_flags() {
    use clap::Parser;
    let cli = crate::cli::Cli::try_parse_from([
        "argo",
        "worker",
        "actions",
        "approve-batch",
        "id1",
        "id2",
        "--deny",
        "--confirm",
        "--dry-run",
        "--note",
        "bulk decline",
    ])
    .expect("approve-batch with all flags parses");
    let _ = cli; // structural parse is the assertion.
}

/// `--confirm` has a `-y` short alias per the brief — pin it.
#[test]
fn approve_batch_short_y_aliases_confirm() {
    use clap::Parser;
    let cli = crate::cli::Cli::try_parse_from([
        "argo",
        "worker",
        "actions",
        "approve-batch",
        "id1",
        "-y",
    ])
    .expect("-y short flag must alias --confirm");
    let _ = cli;
}

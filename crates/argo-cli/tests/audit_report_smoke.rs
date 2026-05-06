mod common;

use argo_audit::compute_chain_hash;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::SessionId;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn audit_report_renders_markdown_html_output_and_json() {
    let home = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let root = home.path().to_string_lossy().to_string();
    let (_tenant, workspace) = store.ensure_workspace("audit-report", &root).await.unwrap();
    let session = store
        .create_session(_tenant, workspace, "mock", "mock", Some("audit"), None)
        .await
        .unwrap();
    seed_rows(&store, session).await;
    seed_budget(&store).await;
    seed_pending_action(&store, workspace).await;

    let report_path = out_dir.path().join("report.md");
    let envelope = run_json(
        home.path(),
        &[
            "audit",
            "report",
            "--since",
            "2026-05-06",
            "--until",
            "2026-05-06",
            "--format",
            "markdown",
            "--output",
            report_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(envelope["report"]["format"], "markdown");
    assert_eq!(envelope["report"]["sections"].as_array().unwrap().len(), 6);
    let markdown = std::fs::read_to_string(&report_path).unwrap();
    assert!(markdown.contains("## Summary stats"));
    assert!(markdown.contains("## Approval-decision breakdown by NotifyChannel"));
    assert!(markdown.contains("## Top 10 spending workers"));
    assert!(markdown.contains("## Pending-approval queue snapshot"));
    assert!(markdown.contains("## Hash-chain integrity check verdict"));
    assert!(markdown.contains("## Pre-existing flakes"));
    assert!(markdown.contains("worker-a"));
    assert!(markdown.contains("approve_listing"));

    let html = run_ok(home.path(), &["audit", "report", "--format", "html"]);
    let body = String::from_utf8(html.stdout).unwrap();
    assert!(body.starts_with("<!doctype html><html"));
    assert!(body.contains("Compliance Audit Report"));
    assert!(body.contains("</html>"));
}

async fn seed_rows(store: &SessionStore, session: SessionId) {
    let base = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .timestamp_micros();
    let first = insert_row(
        store,
        session,
        "tool_call",
        json!({"worker_id":"worker-a","tool":"shell","tenant_id":"tenant-a"}),
        base,
        [0_u8; 32],
    )
    .await;
    let second = insert_row(
        store,
        session,
        "approval.granted",
        json!({"worker_id":"worker-a","channel":"slack"}),
        base + Duration::seconds(1).num_microseconds().unwrap(),
        first,
    )
    .await;
    insert_row(
        store,
        session,
        "escalation.required",
        json!({"worker_id":"worker-b","channel":"email"}),
        base + Duration::seconds(2).num_microseconds().unwrap(),
        second,
    )
    .await;
}

async fn seed_budget(store: &SessionStore) {
    let at = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    argo_identity::worker_budget::record_charge(
        store.pool(),
        "act-spend",
        "worker-a",
        1_250_000,
        at,
    )
    .await
    .unwrap();
}

async fn seed_pending_action(store: &Arc<SessionStore>, workspace: argo_types::WorkspaceId) {
    let memory = MemoryStore::new(Arc::clone(store));
    let body = json!({
        "schema_key": "worker_action",
        "action_id": "act-pending",
        "job_id": "job-1",
        "worker_id": Uuid::nil().to_string(),
        "action_type": "approve_listing",
        "risk_level": "high",
        "requires_approval": true,
        "state": "pending",
        "payload": {},
        "created_at": "2026-05-06T12:00:03Z"
    })
    .to_string();
    memory
        .append_for_vertical(workspace, MemoryFile::Memory, body, None, "audit")
        .await
        .unwrap();
}

async fn insert_row(
    store: &SessionStore,
    session_id: SessionId,
    kind: &str,
    payload: Value,
    created_at: i64,
    parent_hash: [u8; 32],
) -> [u8; 32] {
    let id = Uuid::now_v7();
    let canonical = serde_json::to_string(&payload).unwrap();
    let self_hash = compute_chain_hash(&parent_hash, kind, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(session_id.0.as_bytes().to_vec())
    .bind(kind)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
    self_hash
}

fn run_json(home: &Path, args: &[&str]) -> Value {
    let out = run_ok(home, args);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = run(home, args);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}

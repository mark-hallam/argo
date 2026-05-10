mod common;

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use serde_json::Value;
use std::process::{Command, Output};
use std::sync::Arc;

#[tokio::test]
async fn activity_since_filters_and_emits_ndjson() {
    let home = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let root = home.path().to_string_lossy().to_string();
    let (tenant, workspace) = store
        .ensure_workspace("activity-smoke", &root)
        .await
        .unwrap();
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("activity"), None)
        .await
        .unwrap();
    let sink = AuditSink::new(Arc::clone(&store));
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({
            "worker_id": "worker-alpha",
            "action_kind": "send_email",
            "decision": "ask",
            "message": "draft needs approval"
        }),
        None,
    )
    .await
    .unwrap();

    let output = run(
        home.path(),
        &[
            "activity",
            "since",
            "1d",
            "--tenant",
            &tenant.to_string(),
            "--worker",
            "worker-alpha",
            "--json",
        ],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap();
    let row: Value = serde_json::from_str(line).unwrap();
    assert_eq!(row["type"], "activity");
    assert_eq!(row["activity"]["worker_id"], "worker-alpha");
    assert_eq!(row["activity"]["action_kind"], "send_email");
    assert_eq!(row["activity"]["decision"], "ask");
    assert_eq!(row["activity"]["message"], "draft needs approval");
}

fn run(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}

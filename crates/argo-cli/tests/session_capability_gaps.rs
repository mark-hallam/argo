//! Integration tests for `argo session capability-gaps <id>`
//! (#685). Drives the #682 detector through the binary.

mod common;

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use common::argo_bin;
use std::process::Command;
use std::sync::Arc;

/// Create the session db at <home>/sessions.db, insert a
/// workspace + session, write `n` rbac.deny audit rows for the
/// given tool. Returns the session id (string).
async fn seed_session_with_denials(
    home: &std::path::Path,
    tool: &str,
    n: usize,
) -> String {
    let db = home.join("sessions.db");
    let store = SessionStore::open(&db).await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 't', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let sid = store
        .create_session(tenant, ws, "mock", "mock", Some("gap-int"), Some("alpha"))
        .await
        .unwrap();
    let sink = AuditSink::new(Arc::clone(&store));
    for _ in 0..n {
        sink.write(
            Some(sid),
            AuditKind::RbacDeny,
            serde_json::json!({"tool": tool, "denied_by": "vertical_manifest"}),
            None,
        )
        .await
        .unwrap();
    }
    sid.0.to_string()
}

#[test]
fn cli_capability_gaps_empty_session_returns_count_zero() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let sid = rt.block_on(seed_session_with_denials(home.path(), "x", 0));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "capability-gaps", &sid, "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_capability_gaps_below_threshold_returns_no_gaps() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let sid = rt.block_on(seed_session_with_denials(home.path(), "delegate_task", 2));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "capability-gaps",
            &sid,
            "--threshold",
            "3",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_capability_gaps_above_threshold_surfaces_gap() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let sid = rt.block_on(seed_session_with_denials(home.path(), "delegate_task", 5));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "capability-gaps",
            &sid,
            "--threshold",
            "3",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["count"].as_u64().unwrap(), 1);
    assert_eq!(v["gaps"][0]["kind"], "policy_denial");
    assert_eq!(v["gaps"][0]["anchor"], "delegate_task");
    assert_eq!(v["gaps"][0]["count"].as_u64().unwrap(), 5);
}

#[test]
fn cli_capability_gaps_unknown_session_id_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let ghost = uuid::Uuid::now_v7().to_string();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "capability-gaps", &ghost])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no session"), "stderr: {stderr}");
}

#[test]
fn cli_capability_gaps_invalid_uuid_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "capability-gaps", "not-a-uuid"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("invalid session id"), "stderr: {stderr}");
}

#[test]
fn cli_capability_gaps_text_mode_lists_each_gap() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let sid = rt.block_on(seed_session_with_denials(home.path(), "terminal", 4));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "capability-gaps", &sid, "--threshold", "3"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("policy_denial"), "body: {body}");
    assert!(body.contains("terminal"), "body: {body}");
}

//! Integration tests for `argo session approve` /
//! `argo session revoke-approval` (#664).
//!
//! Locks the operator-facing contract:
//!
//! - `approve` writes a grant + an `approval.granted` audit row
//! - `revoke-approval` removes grants + writes `approval.revoked`
//! - `approve` against an unknown session id fails loud
//! - `--json` mode returns the stable `{ok, grant_id, ...}` shape
//!
//! Cross-process behaviour (CLI grant → next agent turn lets
//! the tool through) is covered indirectly: the grant store is
//! a SQLite table, and the agent runner reads it at every
//! TurnStart. Exercising that fully needs an LLM transport, so
//! it lives outside this crate's CI matrix.

mod common;

use common::argo_bin;
use std::process::Command;

fn create_session(home: &std::path::Path) -> String {
    // `argo new` doesn't have a --json mode; parse "created
    // session <uuid>" out of the first stdout line.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .args([
            "new",
            "--provider",
            "mock",
            "--model",
            "mock",
            "--title",
            "approval-test",
        ])
        .output()
        .expect("spawn new");
    assert!(
        out.status.success(),
        "new stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let line = stdout.lines().next().expect("empty stdout from `new`");
    let id = line
        .strip_prefix("created session ")
        .expect("first line should be `created session <uuid>`");
    id.trim().to_string()
}

#[test]
fn cli_approve_persists_grant_and_emits_audit_row() {
    let home = tempfile::tempdir().unwrap();
    let sid = create_session(home.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "approve",
            &sid,
            "--tool",
            "delegate_task",
            "--note",
            "approved by mark",
            "--json",
        ])
        .output()
        .expect("spawn approve");
    assert!(
        out.status.success(),
        "approve stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["session_id"], sid);
    assert_eq!(v["tool"], "delegate_task");
    assert!(v["grant_id"].as_str().is_some());

    // Confirm an approval.granted audit row is in the chain.
    let audit = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "show", "--session", &sid])
        .output()
        .expect("spawn audit");
    assert!(
        audit.status.success(),
        "audit show stderr: {}",
        String::from_utf8_lossy(&audit.stderr)
    );
    let body = String::from_utf8(audit.stdout).unwrap();
    assert!(body.contains("approval.granted"), "body: {body}");
    // Payload (`tool=delegate_task`) lives in the JSON column;
    // assert presence via the export command which streams the
    // full payload as ndjson.
    let export = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "export", "--session", &sid])
        .output()
        .expect("spawn export");
    assert!(export.status.success());
    let exported = String::from_utf8(export.stdout).unwrap();
    assert!(
        exported.contains("delegate_task"),
        "expected `delegate_task` in payload export, got: {exported}"
    );
}

#[test]
fn cli_approve_against_unknown_session_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "approve",
            "00000000-0000-7000-8000-000000000000",
            "--tool",
            "delegate_task",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "must fail on unknown session");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no session"), "stderr: {stderr}");
}

#[test]
fn cli_approve_with_invalid_session_id_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "approve",
            "not-a-uuid",
            "--tool",
            "delegate_task",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid session id"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_approve_then_revoke_round_trips() {
    let home = tempfile::tempdir().unwrap();
    let sid = create_session(home.path());

    // Two approves of the same tool — the runtime
    // de-duplicates, but the audit trail and grant rows survive.
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "approve", &sid, "--tool", "delegate_task"])
        .status()
        .expect("approve 1");
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "approve", &sid, "--tool", "delegate_task"])
        .status()
        .expect("approve 2");

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "revoke-approval",
            &sid,
            "--tool",
            "delegate_task",
            "--json",
        ])
        .output()
        .expect("spawn revoke");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["removed"].as_u64().unwrap(), 2);

    // approval.revoked audit row is in the chain.
    let audit = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "show", "--session", &sid])
        .output()
        .expect("spawn audit");
    assert!(audit.status.success());
    let body = String::from_utf8(audit.stdout).unwrap();
    assert!(body.contains("approval.revoked"), "body: {body}");
}

#[test]
fn cli_list_grants_after_two_approves_shows_both() {
    let home = tempfile::tempdir().unwrap();
    let sid = create_session(home.path());

    // Approve two distinct tools.
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "approve", &sid, "--tool", "delegate_task"])
        .status()
        .unwrap();
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "approve",
            &sid,
            "--tool",
            "terminal",
            "--note",
            "for one-off debug",
        ])
        .status()
        .unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "list-grants", &sid, "--json"])
        .output()
        .expect("spawn list-grants");
    assert!(
        out.status.success(),
        "list-grants stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["session_id"], sid);
    assert_eq!(v["count"].as_u64().unwrap(), 2);
    let grants = v["grants"].as_array().unwrap();
    let tools: Vec<&str> = grants
        .iter()
        .map(|g| g["tool"].as_str().unwrap())
        .collect();
    assert!(tools.contains(&"delegate_task"), "got: {tools:?}");
    assert!(tools.contains(&"terminal"), "got: {tools:?}");
    // One of them carries the note we passed.
    let with_note: Vec<&serde_json::Value> = grants
        .iter()
        .filter(|g| g["note"].as_str().is_some())
        .collect();
    assert_eq!(with_note.len(), 1);
    assert_eq!(with_note[0]["note"].as_str().unwrap(), "for one-off debug");
}

#[test]
fn cli_list_grants_for_session_with_no_grants_returns_count_zero() {
    let home = tempfile::tempdir().unwrap();
    let sid = create_session(home.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "list-grants", &sid, "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["count"].as_u64().unwrap(), 0);
    assert!(v["grants"].as_array().unwrap().is_empty());
}

#[test]
fn cli_list_grants_unknown_session_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "list-grants",
            "00000000-0000-7000-8000-000000000000",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no session"), "stderr: {stderr}");
}

#[test]
fn cli_revoke_with_no_grants_returns_zero_and_succeeds() {
    let home = tempfile::tempdir().unwrap();
    let sid = create_session(home.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session",
            "revoke-approval",
            &sid,
            "--tool",
            "never-granted",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["removed"].as_u64().unwrap(), 0);
}

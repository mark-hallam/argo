//! Integration tests for `argo skill review` (#684) - drive
//! the learned-skill review gate through the binary so the CLI
//! wiring + JSON shape stay locked.
//!
//! Skills can't be created from the CLI in PendingReview state
//! today (no `argo skill propose` command - proposals come from
//! the agent runner via `propose_for_review`). For the tests
//! we install a pending row directly via the library, then
//! exercise the CLI surface against it.

mod common;

use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use common::argo_bin;
use std::process::Command;
use std::sync::Arc;

/// Set up `<home>/sessions.db`, install a workspace + a session
/// bound to `vertical`, propose a skill in pending_review, and
/// return the skill id plus originating session id.
async fn seed_pending_skill_for_vertical(
    home: &std::path::Path,
    name: &str,
    vertical: &str,
) -> (String, SessionId) {
    let db_path = home.join("sessions.db");
    let store = SessionStore::open(&db_path).await.unwrap();
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
        .create_session(
            tenant,
            ws,
            "mock",
            "mock",
            Some("review-int"),
            Some(vertical),
        )
        .await
        .unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    let proposed = skills
        .propose_for_review(&skill_doc(name), sid, vertical)
        .await
        .unwrap();
    (proposed.id.0.to_string(), sid)
}

async fn seed_pending_skill(home: &std::path::Path, name: &str) -> String {
    seed_pending_skill_for_vertical(home, name, "alpha").await.0
}

fn skill_doc(name: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1\"\ndescription: D\n\
         lock: none\nprovenance:\n  source: t\n  hash: sha256-x\n---\n\n# {name}\n\n## Trajectory\n\n1. opened source\n2. generalized pattern\n\n{name}-body"
    )
}

fn parse_json(out: Vec<u8>) -> serde_json::Value {
    serde_json::from_str(&String::from_utf8(out).unwrap()).unwrap()
}

#[test]
fn cli_skill_review_list_returns_empty_for_clean_workspace() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = parse_json(out.stdout);
    assert_eq!(v["count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_skill_review_list_shows_seeded_pending_skill() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _id = rt.block_on(seed_pending_skill(home.path(), "to_review"));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v = parse_json(out.stdout);
    assert_eq!(v["count"].as_u64().unwrap(), 1);
    let pending = v["pending"].as_array().unwrap();
    assert_eq!(pending[0]["name"], "to_review");
    assert_eq!(pending[0]["vertical_id"], "alpha");
}

#[test]
fn cli_skill_review_list_filters_by_vertical_and_limit() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(seed_pending_skill_for_vertical(
        home.path(),
        "alpha_one",
        "alpha",
    ));
    rt.block_on(seed_pending_skill_for_vertical(
        home.path(),
        "beta_one",
        "beta",
    ));
    rt.block_on(seed_pending_skill_for_vertical(
        home.path(),
        "beta_two",
        "beta",
    ));

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "skill",
            "review",
            "list",
            "--vertical",
            "beta",
            "--limit",
            "1",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v = parse_json(out.stdout);
    assert_eq!(v["count"].as_u64().unwrap(), 1);
    let pending = v["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["vertical_id"], "beta");
}

#[test]
fn cli_skill_review_show_returns_body_provenance_and_origin() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (id, sid) = rt.block_on(seed_pending_skill_for_vertical(
        home.path(),
        "show_target",
        "alpha",
    ));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "show", &id, "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "show stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = parse_json(out.stdout);
    assert_eq!(v["ok"], true);
    assert_eq!(v["name"], "show_target");
    assert_eq!(v["provenance"]["source"], "t");
    assert_eq!(v["originating_session"], sid.0.to_string());
    assert!(v["body"].as_str().unwrap().contains("show_target-body"));
    assert_eq!(v["trajectory"].as_array().unwrap()[0], "1. opened source");
}

#[test]
fn cli_skill_review_approve_promotes_to_active_and_records_note() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (id, sid) = rt.block_on(seed_pending_skill_for_vertical(
        home.path(),
        "approve_target",
        "alpha",
    ));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "skill", "review", "approve", &id, "--note", "ship it", "--json",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "approve stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = parse_json(out.stdout);
    assert_eq!(v["ok"], true);
    assert_eq!(v["to"], "active");
    assert_eq!(v["note"], "ship it");

    let rows = rt.block_on(async {
        let store = SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap();
        store.list_audit_for_session(sid).await.unwrap()
    });
    let approved = rows.iter().find(|r| r.kind == "skill.approved").unwrap();
    let payload: serde_json::Value = serde_json::from_str(&approved.payload_json).unwrap();
    assert_eq!(payload["note"], "ship it");
    assert_eq!(payload["reviewed_by"], "cli");

    let list = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .unwrap();
    let lv = parse_json(list.stdout);
    assert_eq!(lv["count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_skill_review_reject_marks_rejected() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let id = rt.block_on(seed_pending_skill(home.path(), "reject_target"));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "skill",
            "review",
            "reject",
            &id,
            "--note",
            "not reusable",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v = parse_json(out.stdout);
    assert_eq!(v["to"], "rejected");
    assert_eq!(v["note"], "not reusable");
}

#[test]
fn cli_skill_review_approve_unknown_id_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let ghost = uuid::Uuid::now_v7().to_string();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "approve", &ghost])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no skill"), "stderr: {stderr}");
}

#[test]
fn cli_skill_review_approve_invalid_uuid_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "approve", "not-a-uuid"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("invalid skill id"), "stderr: {stderr}");
}

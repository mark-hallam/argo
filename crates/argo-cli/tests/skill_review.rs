//! Integration tests for `argo skill review` (#684) — drive
//! the learned-skill review gate through the binary so the CLI
//! wiring + JSON shape stay locked.
//!
//! Skills can't be created from the CLI in PendingReview state
//! today (no `argo skill propose` command — proposals come from
//! the agent runner via `propose_for_review`). For the tests
//! we install a pending row directly via the library, then
//! exercise the CLI surface against it.

mod common;

use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use common::argo_bin;
use std::process::Command;
use std::sync::Arc;

/// Set up `<home>/sessions.db`, install a workspace + a session
/// bound to vertical "alpha", propose a skill in pending_review.
/// Returns the skill's id (string).
async fn seed_pending_skill(home: &std::path::Path, name: &str) -> String {
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
        .create_session(tenant, ws, "mock", "mock", Some("review-int"), Some("alpha"))
        .await
        .unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    let proposed = skills
        .propose_for_review(
            &format!(
                "---\nname: {name}\nversion: \"0.1\"\ndescription: D\n\
                 lock: none\nprovenance:\n  source: t\n  hash: sha256-x\n---\n{name}-body"
            ),
            sid,
            "alpha",
        )
        .await
        .unwrap();
    proposed.id.0.to_string()
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
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
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
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["count"].as_u64().unwrap(), 1);
    let pending = v["pending"].as_array().unwrap();
    assert_eq!(pending[0]["name"], "to_review");
    assert_eq!(pending[0]["vertical_id"], "alpha");
}

#[test]
fn cli_skill_review_approve_promotes_to_active() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let id = rt.block_on(seed_pending_skill(home.path(), "approve_target"));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "approve", &id, "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "approve stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["to"], "active");

    // Pending list now empty.
    let list = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .unwrap();
    let lv: serde_json::Value =
        serde_json::from_str(&String::from_utf8(list.stdout).unwrap()).unwrap();
    assert_eq!(lv["count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_skill_review_reject_marks_rejected() {
    let home = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let id = rt.block_on(seed_pending_skill(home.path(), "reject_target"));
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "review", "reject", &id, "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["to"], "rejected");
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

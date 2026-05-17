//! Unit tests for `argo doctor fix` — split out of `doctor_fix.rs` for LOC budget.

use super::*;
use crate::home::db_path;
use argo_store::SessionStore;
use tempfile::TempDir;

fn tc() -> (TempDir, TempDir, FixContext) {
    let cwd = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let ctx = FixContext {
        cwd: cwd.path().to_path_buf(),
        home: home.path().join("argo-home"),
    };
    (cwd, home, ctx)
}

async fn audits(ctx: &FixContext) -> i64 {
    let store = SessionStore::open(db_path(&ctx.home)).await.unwrap();
    store.migrate().await.unwrap();
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'doctor_fix'")
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn safe_fixes_apply_without_auto_and_write_audit() {
    let (_cwd, _home, ctx) = tc();
    let out = run_with_context(&ctx, false, None).await.unwrap();
    assert!(out.ok);
    assert_eq!(out.fixes.len(), 4);
    assert!(ctx.home.is_dir());
    assert!(ctx.cwd.join(".argo/manifests").is_dir());
    assert!(ctx.cwd.join(POLICY_RELPATH).is_file());
    assert_eq!(audits(&ctx).await, 4);
}

#[tokio::test]
async fn issue_filter_runs_only_one_fix() {
    let (_cwd, _home, ctx) = tc();
    let out = run_with_context(&ctx, false, Some("manifest_dir"))
        .await
        .unwrap();
    assert!(out.ok);
    assert_eq!(out.fixes[0].id, "manifest_dir");
    assert!(!ctx.cwd.join(POLICY_RELPATH).exists());
    assert_eq!(audits(&ctx).await, 1);
}

#[tokio::test]
async fn stale_lock_is_removed() {
    let (_cwd, _home, ctx) = tc();
    let lock = ctx.cwd.join("cron/.tick.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, "old").unwrap();
    assert!(std::process::Command::new("touch")
        .args(["-t", "202001010000"])
        .arg(&lock)
        .status()
        .unwrap()
        .success());
    let out = run_with_context(&ctx, false, Some("stale_lock"))
        .await
        .unwrap();
    assert!(out.ok);
    assert_eq!(out.fixes[0].status, "applied");
    assert!(!lock.exists());
}

#[tokio::test]
async fn confirmation_fix_requires_auto() {
    let (_cwd, _home, ctx) = tc();
    let out = run_with_context(&ctx, false, Some("migrations"))
        .await
        .unwrap();
    assert!(!out.ok);
    assert_eq!(out.fixes[0].status, "blocked");
    assert_eq!(audits(&ctx).await, 1);
}

#[tokio::test]
async fn vertical_required_fields_auto_patches_manifest() {
    let (_cwd, _home, ctx) = tc();
    let root = ctx.cwd.join("verticals/alpha");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("vertical.toml"), "[vertical]\nid = \"alpha\"\n").unwrap();
    assert!(
        !run_with_context(&ctx, false, Some("vertical_required_fields"))
            .await
            .unwrap()
            .ok
    );
    let out = run_with_context(&ctx, true, Some("vertical_required_fields"))
        .await
        .unwrap();
    assert!(out.ok);
    let body = std::fs::read_to_string(root.join("vertical.toml")).unwrap();
    assert!(body.contains("schema_version") && body.contains("version"));
}

#[tokio::test]
async fn unsafe_issue_returns_remediation_and_not_ok() {
    let (_cwd, _home, ctx) = tc();
    let out = run_with_context(&ctx, false, Some("database_corruption"))
        .await
        .unwrap();
    assert!(!out.ok);
    assert_eq!(out.fixes[0].status, "unsafe");
    assert!(out.fixes[0]
        .remediation
        .as_deref()
        .unwrap()
        .contains("restore"));
}

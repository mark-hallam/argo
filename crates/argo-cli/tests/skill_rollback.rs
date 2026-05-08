use argo_skill::{PrincipalOverride, SkillStore};
use argo_store::SessionStore;
use serde_json::Value;
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

struct World {
    home: TempDir,
    work: TempDir,
    store: Arc<SessionStore>,
}

impl World {
    fn skills(&self) -> SkillStore {
        SkillStore::new(Arc::clone(&self.store))
    }
}

fn skill_doc(name: &str, version: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}"
    )
}

async fn seeded() -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    skills
        .install(&skill_doc("alpha", "0.1", "v1"), None)
        .await
        .unwrap();
    skills
        .supersede(
            &skill_doc("alpha", "0.2", "v2"),
            None,
            PrincipalOverride::Force,
        )
        .await
        .unwrap();
    World { home, work, store }
}

fn run(world: &World, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(world.work.path())
        .env("ARGO_HOME", world.home.path())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &Output, needle: &str) {
    assert!(!output.status.success(), "expected failure");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains(needle),
        "missing {needle:?} in {combined}"
    );
}

async fn active_version(world: &World) -> String {
    world
        .skills()
        .find_active_by_name("alpha")
        .await
        .unwrap()
        .unwrap()
        .frontmatter
        .version
}

async fn set_lifecycle(world: &World, version: &str, state: &str) {
    sqlx::query("UPDATE skill SET lifecycle_state = ? WHERE name = 'alpha' AND version = ?")
        .bind(state)
        .bind(version)
        .execute(world.store.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn rollback_promotes_prior_version_and_prints_json() {
    let world = seeded().await;
    let out = run(
        &world,
        &["skill", "rollback", "alpha", "--to", "0.1", "--json"],
    );
    assert_success(&out);
    let payload: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["dry_run"], false);
    assert_eq!(payload["name"], "alpha");
    assert_eq!(payload["from_version"], "0.2");
    assert_eq!(payload["to_version"], "0.1");
    assert_eq!(active_version(&world).await, "0.1");
}

#[tokio::test]
async fn rollback_unknown_name_fails_clearly() {
    let world = seeded().await;
    let out = run(&world, &["skill", "rollback", "missing", "--to", "0.1"]);
    assert_failure_contains(&out, "unknown skill 'missing'");
}

#[tokio::test]
async fn rollback_missing_version_fails_clearly() {
    let world = seeded().await;
    let out = run(&world, &["skill", "rollback", "alpha", "--to", "9.9"]);
    assert_failure_contains(&out, "has no version '9.9'");
}

#[tokio::test]
async fn rollback_rejects_archived_target() {
    let world = seeded().await;
    set_lifecycle(&world, "0.1", "archived").await;
    let out = run(&world, &["skill", "rollback", "alpha", "--to", "0.1"]);
    assert_failure_contains(&out, "cannot rollback to archived version '0.1'");
}

#[tokio::test]
async fn rollback_rejects_rejected_target() {
    let world = seeded().await;
    set_lifecycle(&world, "0.1", "rejected").await;
    let out = run(&world, &["skill", "rollback", "alpha", "--to", "0.1"]);
    assert_failure_contains(&out, "cannot rollback to rejected version '0.1'");
}

#[tokio::test]
async fn rollback_dry_run_does_not_write() {
    let world = seeded().await;
    let out = run(
        &world,
        &[
            "skill",
            "rollback",
            "alpha",
            "--to",
            "0.1",
            "--dry-run",
            "--json",
        ],
    );
    assert_success(&out);
    let payload: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["dry_run"], true);
    assert_eq!(active_version(&world).await, "0.2");
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'skill.rollback'")
            .fetch_one(world.store.pool())
            .await
            .unwrap();
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn rollback_reason_is_written_to_audit_payload() {
    let world = seeded().await;
    let out = run(
        &world,
        &[
            "skill",
            "rollback",
            "alpha",
            "--to",
            "0.1",
            "--reason",
            "bad deploy",
        ],
    );
    assert_success(&out);
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'skill.rollback' LIMIT 1",
    )
    .fetch_one(world.store.pool())
    .await
    .unwrap();
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["name"], "alpha");
    assert_eq!(payload["from_version"], "0.2");
    assert_eq!(payload["to_version"], "0.1");
    assert_eq!(payload["reason"], "bad deploy");
}

mod common;

use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};
use uuid::Uuid;

#[test]
fn budget_by_skill_json_and_history_smoke() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let expected_day = rt.block_on(seed(home.path()));

    let top = run(
        &home,
        &work,
        &["budget", "by-skill", "--since", "90d", "--json"],
    );
    assert_success(&top);
    let top: Value = serde_json::from_slice(&top.stdout).unwrap();
    assert_eq!(top["ok"], true);
    assert_eq!(top["mode"], "top");
    assert_eq!(top["since"], "90d");
    assert_eq!(top["rows"][0]["skill_name"], "expensive-skill");
    assert_eq!(top["rows"][0]["spend_usd_micros"], 4_000_000);

    let history = run(
        &home,
        &work,
        &[
            "budget",
            "by-skill",
            "expensive-skill",
            "--history",
            "--json",
        ],
    );
    assert_success(&history);
    let history: Value = serde_json::from_slice(&history.stdout).unwrap();
    assert_eq!(history["ok"], true);
    assert_eq!(history["mode"], "history");
    assert_eq!(history["rows"][0]["day"], expected_day);
}

async fn seed(home: &std::path::Path) -> String {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'test', '.', 1)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let expensive = insert_skill(&store, "expensive-skill", "ops").await;
    let cheap = insert_skill(&store, "cheap-skill", "ops").await;
    let s1 = store
        .create_session(
            tenant,
            workspace,
            "mock",
            "mock",
            Some("expensive"),
            Some("ops"),
        )
        .await
        .unwrap();
    let s2 = store
        .create_session(
            tenant,
            workspace,
            "mock",
            "mock",
            Some("cheap"),
            Some("ops"),
        )
        .await
        .unwrap();
    attach(&store, s1, expensive).await;
    attach(&store, s2, cheap).await;
    let at = chrono::Utc::now();
    let expected_day = at.format("%Y-%m-%d").to_string();
    let at = at.timestamp_micros();
    store
        .record_spend(tenant, s1, at, 4_000_000, "mock", "mock")
        .await
        .unwrap();
    store
        .record_spend(tenant, s2, at, 1_000_000, "mock", "mock")
        .await
        .unwrap();
    expected_day
}

async fn insert_skill(store: &SessionStore, name: &str, vertical: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, vertical_id) \
         VALUES (?, ?, '1.0.0', 'body', '{}', 'test', ?, 1, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(name)
    .bind(vec![7_u8; 32])
    .bind(vertical)
    .execute(store.pool())
    .await
    .unwrap();
    id
}

async fn attach(store: &SessionStore, session: argo_types::SessionId, skill: Uuid) {
    sqlx::query("INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, 1)")
        .bind(session.0.as_bytes().as_slice())
        .bind(skill.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
}

fn run(home: &tempfile::TempDir, work: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

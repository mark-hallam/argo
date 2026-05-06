mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::{MessageRole, TenantId, WorkspaceId};
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;
use uuid::Uuid;

struct World {
    home: TempDir,
    work: TempDir,
}

#[tokio::test]
async fn tenant_create_list_and_info_round_trip() {
    let world = World {
        home: tempfile::tempdir().unwrap(),
        work: tempfile::tempdir().unwrap(),
    };

    let created = run(&world, &["tenant", "create", "Acme Ops", "--json"]);
    assert_success(&created);
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    let tenant = created["id"].as_str().unwrap();
    let workspace = created["workspace_id"].as_str().unwrap();
    seed_activity(&world, tenant, workspace).await;

    let listed = run(&world, &["tenant", "list", "--json"]);
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == tenant)
        .expect("created tenant listed");
    assert_eq!(row["name"], "Acme Ops");
    assert_eq!(row["worker_count"], 1);
    assert_eq!(row["session_count"], 1);

    let info = run(&world, &["tenant", "info", tenant, "--json"]);
    assert_success(&info);
    let info: Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(info["tenant"]["worker_count"], 1);
    assert_eq!(info["tenant"]["session_count"], 1);
    assert!(info["tenant"]["last_action_at"].as_i64().is_some());
    assert_eq!(info["workers"][0]["persona"], "Maya");
    assert_eq!(info["sessions"][0]["title"], "tenant-demo");

    let text = run(&world, &["tenant", "info", tenant]);
    assert_success(&text);
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("workers:"));
    assert!(text.contains("sessions:"));
    assert!(text.contains("last_action:"));
}

async fn seed_activity(world: &World, tenant: &str, workspace: &str) {
    let store = SessionStore::open(world.home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId(Uuid::parse_str(tenant).unwrap());
    let workspace = WorkspaceId(Uuid::parse_str(workspace).unwrap());
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "marketplace",
        Persona::named("Maya", "operator@example.test"),
        Role::new("Own tenant follow-up."),
    );
    identities.create_worker(&worker).await.unwrap();
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("tenant-demo"), None)
        .await
        .unwrap();
    store
        .append_message(session, MessageRole::User, r#"{"text":"hello"}"#)
        .await
        .unwrap();
}

fn run(world: &World, args: &[&str]) -> Output {
    run_in(&world.home, world.work.path(), args)
}

fn run_in(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
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

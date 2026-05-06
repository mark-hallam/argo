mod common;

use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[tokio::test]
async fn tenant_lifecycle_commands_work_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let created = run(
        home.path(),
        work.path(),
        &["tenant", "create", "Acme Ops", "--json"],
    );
    assert_success(&created);
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    let tenant = created["id"].as_str().unwrap();

    let renamed = run(
        home.path(),
        work.path(),
        &["tenant", "rename", tenant, "--to", "Acme Renamed", "--json"],
    );
    assert_success(&renamed);
    let renamed: Value = serde_json::from_slice(&renamed.stdout).unwrap();
    assert_eq!(renamed["tenant_rename"]["name"], "Acme Renamed");

    let transferred = run(
        home.path(),
        work.path(),
        &[
            "tenant",
            "transfer",
            tenant,
            "--owner",
            "owner@example.test",
            "--json",
        ],
    );
    assert_success(&transferred);
    let transferred: Value = serde_json::from_slice(&transferred.stdout).unwrap();
    assert_eq!(
        transferred["tenant_transfer"]["new_owner_email"],
        "owner@example.test"
    );

    let archive = run(
        home.path(),
        work.path(),
        &["tenant", "archive", tenant, "--json"],
    );
    assert_success(&archive);
    let archive: Value = serde_json::from_slice(&archive.stdout).unwrap();
    assert_eq!(archive["tenant_archive"]["tenant_id"], tenant);

    let visible = run(home.path(), work.path(), &["tenant", "list", "--json"]);
    assert_success(&visible);
    let visible: Value = serde_json::from_slice(&visible.stdout).unwrap();
    assert!(!visible
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == tenant));

    let archived = run(
        home.path(),
        work.path(),
        &["tenant", "list", "--include-archived", "--json"],
    );
    assert_success(&archived);
    let archived: Value = serde_json::from_slice(&archived.stdout).unwrap();
    let row = archived
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == tenant)
        .unwrap();
    assert_eq!(row["name"], "Acme Renamed");
    assert_eq!(row["owner_email"], "owner@example.test");
    assert!(row["archived_at"].as_i64().is_some());

    assert_transfer_audit(home.path(), tenant).await;
}

async fn assert_transfer_audit(home: &Path, tenant: &str) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'tenant_transfer' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["tenant_id"], tenant);
    assert_eq!(payload["old_owner_email"], Value::Null);
    assert_eq!(payload["new_owner_email"], "owner@example.test");
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
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

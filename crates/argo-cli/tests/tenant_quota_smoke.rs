mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use common::argo_bin;
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn tenant_quota_cli_and_auto_approval_gate_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let verticals = write_vertical_fixture();

    let created = run(
        home.path(),
        work.path(),
        &verticals,
        &["tenant", "create", "Acme", "--json"],
    );
    assert_success(&created);
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    let tenant = created["id"].as_str().unwrap();
    let workspace = created["workspace_id"].as_str().unwrap();

    let set = run(
        home.path(),
        work.path(),
        &verticals,
        &[
            "tenant",
            "quota",
            "set",
            tenant,
            "--max-usd",
            "100",
            "--max-actions",
            "10",
            "--period",
            "monthly",
            "--json",
        ],
    );
    assert_success(&set);
    let set: Value = serde_json::from_slice(&set.stdout).unwrap();
    assert_eq!(set["ok"], true);
    assert_eq!(set["tenant_quota"]["quota"]["max_usd_micros"], 100_000_000);

    let show = run(
        home.path(),
        work.path(),
        &verticals,
        &["tenant", "quota", "show", tenant, "--json"],
    );
    assert_success(&show);
    let show: Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(show["tenant_quota"]["remaining"]["actions"], 10);

    let list = run(
        home.path(),
        work.path(),
        &verticals,
        &["tenant", "quota", "list", "--json"],
    );
    assert_success(&list);
    let list: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(list["tenant_quotas"].as_array().unwrap().len(), 1);

    let clear_field = run(
        home.path(),
        work.path(),
        &verticals,
        &[
            "tenant",
            "quota",
            "clear",
            tenant,
            "--field",
            "max-actions",
            "--json",
        ],
    );
    assert_success(&clear_field);
    let cleared: Value = serde_json::from_slice(&clear_field.stdout).unwrap();
    assert_eq!(
        cleared["tenant_quota_clear"]["summary"]["quota"]["max_actions"],
        Value::Null
    );

    let worker = seed_gate_action(home.path(), tenant, workspace).await;
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    std::fs::write(
        work.path().join(".skycot/auto_approval.toml"),
        "enabled = true\ndaily_budget_usd_micros = 100000000\n",
    )
    .unwrap();
    let zero_cap = run(
        home.path(),
        work.path(),
        &verticals,
        &[
            "tenant",
            "quota",
            "set",
            tenant,
            "--max-usd",
            "0",
            "--period",
            "daily",
            "--json",
        ],
    );
    assert_success(&zero_cap);

    let blocked = run(
        home.path(),
        work.path(),
        &verticals,
        &["worker", "actions", "execute", "act-quota", "--json"],
    );
    assert!(
        !blocked.status.success(),
        "worker {worker} should be blocked by tenant quota\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("tenant_quota:max_usd"));
}

async fn seed_gate_action(home: &Path, tenant: &str, workspace: &str) -> String {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId(Uuid::parse_str(tenant).unwrap());
    let workspace = WorkspaceId(Uuid::parse_str(workspace).unwrap());
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "quota-fixture",
        Persona::named("Maya", "builder@example.test"),
        Role::new("Test quota gate."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();
    let memory = MemoryStore::new(Arc::new(store));
    let body = json!({
        "schema_key": "worker_action",
        "action_id": "act-quota",
        "job_id": "job-quota",
        "worker_id": worker_id,
        "action_type": "post_listing",
        "risk_level": "low",
        "requires_approval": true,
        "state": "pending",
        "payload": {"worker_id": worker_id},
        "created_at": "2026-05-06T12:00:00Z"
    });
    memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            body.to_string(),
            None,
            "quota-fixture",
        )
        .await
        .unwrap();
    worker_id
}

fn write_vertical_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("quota-fixture");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("vertical.toml"),
        r#"
schema_version = "1"

[vertical]
id = "quota-fixture"
name = "Quota Fixture"
version = "0.0.1"

[actions.post_listing]
risk = "low"
requires_approval = true
description = "Post listing."
executor = { kind = "computer_use", profile = "test", budget_usd = 0.50 }
"#,
    )
    .unwrap();
    dir
}

fn run(home: &Path, cwd: &Path, verticals: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals.path())
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

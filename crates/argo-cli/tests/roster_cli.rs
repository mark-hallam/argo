//! Integration tests for `argo roster ...` and roster lineage in
//! `argo worker show`. Stream `claude-worker-roster`.

mod common;

use argo_identity::{AutoApprovalPolicy, IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

struct World {
    home: TempDir,
    work: TempDir,
    tenant: TenantId,
    worker_id: String,
}

#[tokio::test]
async fn roster_create_then_show_round_trip() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();

    let created = run(
        &world,
        &[
            "roster",
            "create",
            "--tenant",
            &tenant,
            "--budget-cap-usd",
            "75.5",
            "--json",
            "alpha-team",
        ],
    );
    assert_success(&created);
    let payload: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(payload["name"], "alpha-team");
    assert_eq!(payload["tenant_id"], tenant);

    let listed = run(
        &world,
        &["roster", "list", "--tenant", &tenant, "--json"],
    );
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let names: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"alpha-team"));

    let shown = run(
        &world,
        &[
            "roster",
            "show",
            "--tenant",
            &tenant,
            "alpha-team",
            "--json",
        ],
    );
    assert_success(&shown);
    let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(detail["name"], "alpha-team");
    let cap = detail["budget_cap_usd"].as_f64().unwrap();
    assert!((cap - 75.5).abs() < 1e-6);
    assert!(detail["member_worker_ids"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn worker_show_includes_roster_lineage() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();

    let created = run(
        &world,
        &[
            "roster",
            "create",
            "--tenant",
            &tenant,
            "--json",
            "ops-team",
        ],
    );
    assert_success(&created);

    let added = run(
        &world,
        &[
            "roster",
            "add-member",
            "--tenant",
            &tenant,
            "ops-team",
            &world.worker_id,
            "--json",
        ],
    );
    assert_success(&added);
    let added_payload: Value = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(added_payload["member_count"], 1);

    let shown = run(
        &world,
        &[
            "worker",
            "show",
            &world.worker_id,
            "--tenant",
            &tenant,
            "--json",
        ],
    );
    assert_success(&shown);
    let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
    let rosters = detail["rosters"].as_array().unwrap();
    assert_eq!(rosters.len(), 1);
    assert_eq!(rosters[0]["name"], "ops-team");

    let removed = run(
        &world,
        &[
            "roster",
            "remove-member",
            "--tenant",
            &tenant,
            "ops-team",
            &world.worker_id,
            "--json",
        ],
    );
    assert_success(&removed);
    let removed_payload: Value = serde_json::from_slice(&removed.stdout).unwrap();
    assert_eq!(removed_payload["member_count"], 0);
}

async fn seed_world() -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    std::fs::write(
        work.path().join(".skycot/auto_approval.toml"),
        "enabled = false\n",
    )
    .unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("roster-cli", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "marketplace",
        Persona::named("Mira", "operator@example.test"),
        Role::new("Triage marketplace approvals."),
    );
    let worker_id = worker.id.to_string();
    let mut worker = worker;
    worker.auto_approval = AutoApprovalPolicy::default();
    identities.create_worker(&worker).await.unwrap();
    World {
        home,
        work,
        tenant,
        worker_id,
    }
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

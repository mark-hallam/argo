//! Integration tests for `argo team ...` (Stream
//! `claude-worker-team-roster-cli`). Spawns the binary against a
//! workspace-scoped temp `ARGO_HOME` so the schema and dispatch
//! path are exercised end-to-end.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
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
async fn team_create_list_show_round_trip() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();

    let created = run(
        &world,
        &["team", "create", "--tenant", &tenant, "--json", "finance"],
    );
    assert_success(&created);
    let payload: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(payload["ok"], true);
    let team_id = payload["team_id"].as_str().unwrap().to_string();

    let listed = run(
        &world,
        &["team", "list", "--tenant", &tenant, "--format", "json"],
    );
    assert_success(&listed);
    let listed_payload: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let teams = listed_payload["teams"].as_array().unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["name"], "finance");

    let shown = run(
        &world,
        &[
            "team", "show", "--tenant", &tenant, &team_id, "--format", "json",
        ],
    );
    assert_success(&shown);
    let shown_payload: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown_payload["name"], "finance");
    assert_eq!(shown_payload["level"], 0);
    assert!(shown_payload["members"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn team_add_remove_round_trip() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();
    let create = run(
        &world,
        &["team", "create", "--tenant", &tenant, "--json", "support"],
    );
    assert_success(&create);
    let team_id = serde_json::from_slice::<Value>(&create.stdout).unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let added = run(
        &world,
        &[
            "team",
            "add",
            "--tenant",
            &tenant,
            &team_id,
            &world.worker_id,
            "--role",
            "lead",
            "--json",
        ],
    );
    assert_success(&added);
    let added_payload: Value = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(added_payload["ok"], true);
    assert_eq!(added_payload["role"], "lead");

    let shown = run(
        &world,
        &[
            "team", "show", "--tenant", &tenant, &team_id, "--format", "json",
        ],
    );
    let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
    let members = detail["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["role"], "lead");

    let removed = run(
        &world,
        &[
            "team",
            "remove",
            "--tenant",
            &tenant,
            &team_id,
            &world.worker_id,
            "--json",
        ],
    );
    assert_success(&removed);
    let removed_payload: Value = serde_json::from_slice(&removed.stdout).unwrap();
    assert_eq!(removed_payload["soft_deleted"], true);

    let shown = run(
        &world,
        &[
            "team", "show", "--tenant", &tenant, &team_id, "--format", "json",
        ],
    );
    let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert!(detail["members"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn team_rename_updates_listing() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();
    let create = run(
        &world,
        &["team", "create", "--tenant", &tenant, "--json", "ops"],
    );
    let team_id = serde_json::from_slice::<Value>(&create.stdout).unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let renamed = run(
        &world,
        &[
            "team",
            "rename",
            "--tenant",
            &tenant,
            &team_id,
            "operations",
            "--json",
        ],
    );
    assert_success(&renamed);

    let listed = run(
        &world,
        &["team", "list", "--tenant", &tenant, "--format", "json"],
    );
    let payload: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(payload["teams"][0]["name"], "operations");
}

#[tokio::test]
async fn team_create_with_parent_renders_markdown_show() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();
    let parent = run(
        &world,
        &["team", "create", "--tenant", &tenant, "--json", "engineering"],
    );
    let parent_id = serde_json::from_slice::<Value>(&parent.stdout).unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();
    let child = run(
        &world,
        &[
            "team",
            "create",
            "--tenant",
            &tenant,
            "--parent",
            &parent_id,
            "--json",
            "platform",
        ],
    );
    let child_id = serde_json::from_slice::<Value>(&child.stdout).unwrap()["team_id"]
        .as_str()
        .unwrap()
        .to_string();

    let shown = run(
        &world,
        &[
            "team", "show", "--tenant", &tenant, &child_id, "--format", "markdown",
        ],
    );
    assert_success(&shown);
    let body = String::from_utf8_lossy(&shown.stdout);
    assert!(body.contains("# Team `platform`"));
    assert!(body.contains("- level: 1"));
    assert!(body.contains(&format!("- parent: `{parent_id}`")));
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
        .ensure_workspace("team-cli", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Mira", "operator@example.test"),
        Role::new("Triage support tickets."),
    );
    let worker_id = worker.id.to_string();
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

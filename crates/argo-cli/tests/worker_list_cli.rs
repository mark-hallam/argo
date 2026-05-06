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
    workspace_default_worker: String,
}

#[tokio::test]
async fn worker_list_show_and_policy_show_are_tenant_scoped() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();

    let listed = run(&world, &["worker", "list", "--tenant", &tenant, "--json"]);
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 3);
    assert!(rows[0].get("created_at").is_some());
    assert!(rows[0].get("created_at_rfc3339").is_some());

    let filtered = run(
        &world,
        &[
            "worker",
            "list",
            "--tenant",
            &tenant,
            "--persona",
            "Maya",
            "--json",
        ],
    );
    assert_success(&filtered);
    let rows: Value = serde_json::from_slice(&filtered.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row["persona"] == "Maya"));

    let table = run(&world, &["worker", "list", "--tenant", &tenant]);
    assert_success(&table);
    let stdout = String::from_utf8_lossy(&table.stdout);
    assert!(stdout.contains("role.charter"));
    assert!(stdout.contains("auto_approval"));
    assert!(stdout.contains("created_at"));

    let show = run(
        &world,
        &[
            "worker",
            "show",
            &world.workspace_default_worker,
            "--tenant",
            &tenant,
            "--json",
        ],
    );
    assert_success(&show);
    let shown: Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(shown["worker"]["persona"]["name"], "Maya");
    assert_eq!(shown["resolved_policy"]["enabled"], true);
    assert_eq!(shown["resolved_policy"]["daily_budget_usd_micros"], 8800000);

    let policy = run(
        &world,
        &[
            "worker",
            "policy",
            "show",
            &world.workspace_default_worker,
            "--tenant",
            &tenant,
            "--json",
        ],
    );
    assert_success(&policy);
    let policy: Value = serde_json::from_slice(&policy.stdout).unwrap();
    assert_eq!(policy["enabled"], true);
    assert_eq!(policy["daily_budget_usd_micros"], 8800000);

    let missing_tenant = run(&world, &["worker", "list", "--json"]);
    assert!(!missing_tenant.status.success());
    let stderr = String::from_utf8_lossy(&missing_tenant.stderr);
    assert!(stderr.contains("--tenant"));
}

async fn seed_world() -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    std::fs::write(
        work.path().join(".skycot/auto_approval.toml"),
        "enabled = true\ndaily_budget_usd_micros = 8800000\n",
    )
    .unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _workspace) = store
        .ensure_workspace("worker-list-cli", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();

    let worker_a = create_worker(&identities, tenant, "Maya", false).await;
    create_worker(&identities, tenant, "Noah", true).await;
    create_worker(&identities, tenant, "Maya", true).await;

    World {
        home,
        work,
        tenant,
        workspace_default_worker: worker_a,
    }
}

async fn create_worker(
    identities: &IdentityStore<'_>,
    tenant: TenantId,
    persona: &str,
    worker_policy_enabled: bool,
) -> String {
    let mut worker = Worker::draft(
        tenant,
        "marketplace",
        Persona::named(persona, "operator@example.test"),
        Role::new(format!(
            "Own {persona} marketplace follow-up with enough detail to require truncation."
        )),
    );
    worker.auto_approval = AutoApprovalPolicy {
        enabled: worker_policy_enabled,
        max_usd_micros_per_action: Some(420000),
        ..AutoApprovalPolicy::default()
    };
    identities.create_worker(&worker).await.unwrap();
    worker.id.to_string()
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

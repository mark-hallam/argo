//! End-to-end smoke for `argo storage` tenant DB routing.
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

#[test]
fn storage_commands_migrate_and_list_two_tenant_pools() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let eu = workspace.path().join("eu.db");
    let us = workspace.path().join("us.db");
    std::fs::write(
        workspace.path().join("argo.toml"),
        format!(
            r#"
            [storage.tenants.eu-a]
            db_url = "{}"
            region = "eu-west-1"

            [storage.tenants.us-a]
            db_url = "{}"
            region = "us-east-1"
            "#,
            eu.display(),
            us.display()
        ),
    )
    .unwrap();

    let migrate = run(
        workspace.path(),
        home.path(),
        &["storage", "migrate-tenant", "eu-a", "--json"],
    );
    assert!(
        migrate.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&migrate.stderr)
    );
    let migrated: Value = serde_json::from_slice(&migrate.stdout).unwrap();
    assert_eq!(migrated["tenant_id"], "eu-a");
    assert!(migrated["schema_version"].as_u64().unwrap() > 0);

    let listed = run(
        workspace.path(),
        home.path(),
        &["storage", "list-tenants", "--json"],
    );
    assert!(
        listed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| {
        r["tenant_id"] == "eu-a" && r["region"] == "eu-west-1" && r["healthy"] == true
    }));
    assert!(rows
        .iter()
        .any(|r| r["tenant_id"] == "us-a" && r["region"] == "us-east-1"));
}

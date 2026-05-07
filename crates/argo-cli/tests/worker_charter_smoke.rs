//! End-to-end lock for `argo worker charter set/get/clear`.
//! Drives the real `argo` binary against a temp store seeded with a
//! single Worker, asserts that set→get→clear round-trips correctly.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn charter_set_get_clear_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-charter-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Ari", "operator@example.test"),
        Role::new("Triage tickets."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();

    // Write a charter markdown file + KPI JSON file in the workspace.
    let charter_path = work.path().join("charter.md");
    std::fs::write(
        &charter_path,
        "# Charter\n\nReply within 24h. Escalate after 2nd contact.\n",
    )
    .unwrap();
    let kpis_path = work.path().join("kpis.json");
    std::fs::write(
        &kpis_path,
        r#"{"reply_sla_hours": 24, "csat_target": 4.5}"#,
    )
    .unwrap();

    // set
    let set_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "charter",
            "set",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--file",
            charter_path.to_str().unwrap(),
            "--kpis",
            kpis_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_success(&set_out);
    let body: Value = serde_json::from_slice(&set_out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["charter_bytes"].as_u64().unwrap_or(0) > 0);
    assert!(body["kpis_bytes"].as_u64().unwrap_or(0) > 0);

    // get --json
    let get_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "charter",
            "get",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&get_out);
    let body: Value = serde_json::from_slice(&get_out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["charter"]
        .as_str()
        .unwrap_or("")
        .contains("Reply within 24h"));
    assert_eq!(body["kpis"]["reply_sla_hours"], 24);

    // get (markdown default)
    let get_md = run(
        &home,
        work.path(),
        &[
            "worker",
            "charter",
            "get",
            &worker_id,
            "--tenant",
            &tenant_str,
        ],
    );
    assert_success(&get_md);
    let stdout = String::from_utf8_lossy(&get_md.stdout);
    assert!(stdout.contains("Reply within 24h"));
    assert!(stdout.contains("## KPIs"));

    // clear
    let clear_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "charter",
            "clear",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&clear_out);
    let body: Value = serde_json::from_slice(&clear_out.stdout).unwrap();
    assert_eq!(body["cleared"], true);

    // get after clear
    let post_clear = run(
        &home,
        work.path(),
        &[
            "worker",
            "charter",
            "get",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&post_clear);
    let body: Value = serde_json::from_slice(&post_clear.stdout).unwrap();
    assert!(body["charter"].is_null());
    assert!(body["kpis"].is_null());
}

fn run(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
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


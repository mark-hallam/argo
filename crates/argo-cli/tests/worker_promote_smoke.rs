//! End-to-end lock for `argo worker level / promote`.
//! Drives the real `argo` binary against a temp store seeded with a
//! single Worker, asserts level reporting and the `--force` /
//! `--to lead` rules from the brief.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerLevel};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn level_then_force_promote_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-promote-smoke", &work.path().to_string_lossy())
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
    drop(store);

    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();

    // Fresh worker → level=entry, gap-to-junior reported.
    let level_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "level",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&level_out);
    let body: Value = serde_json::from_slice(&level_out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["level"], "entry");
    assert_eq!(body["next_level"], "junior");
    let summary = body["summary"].as_str().unwrap_or("");
    assert!(summary.contains("sessions short"), "{summary}");

    // Promote without --force: blocked by criteria gate.
    let blocked = run(
        &home,
        work.path(),
        &[
            "worker",
            "promote",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--to",
            "junior",
            "--json",
        ],
    );
    assert!(!blocked.status.success(), "expected block, got success");

    // Promote with --force --reason: succeeds and records audit row.
    let forced = run(
        &home,
        work.path(),
        &[
            "worker",
            "promote",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--to",
            "junior",
            "--force",
            "--reason",
            "smoke-test override",
            "--json",
        ],
    );
    assert_success(&forced);
    let body: Value = serde_json::from_slice(&forced.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["from_level"], "entry");
    assert_eq!(body["to_level"], "junior");
    assert_eq!(body["forced"], true);

    // Reload via the store and confirm the persisted level moved.
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    let reloaded = identities.get_worker(tenant, worker.id).await.unwrap().unwrap();
    assert_eq!(reloaded.level, WorkerLevel::Junior);
}

#[tokio::test]
async fn promote_to_lead_requires_force_flag() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-promote-lead-smoke", &work.path().to_string_lossy())
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
    // Pre-seed senior so the only remaining hop is senior → lead.
    identities
        .update_level(tenant, worker.id, WorkerLevel::Senior)
        .await
        .unwrap();
    drop(store);

    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();

    // Without --force: rejected even though the wording differs from
    // the criteria-gap rejection ("requires --force").
    let blocked = run(
        &home,
        work.path(),
        &[
            "worker",
            "promote",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--to",
            "lead",
            "--json",
        ],
    );
    assert!(!blocked.status.success(), "expected block, got success");
    let stderr = String::from_utf8_lossy(&blocked.stderr);
    assert!(
        stderr.contains("--force") || stderr.contains("operator confirmation"),
        "stderr should mention force/operator: {stderr}"
    );

    // With --force --reason: succeeds.
    let forced = run(
        &home,
        work.path(),
        &[
            "worker",
            "promote",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--to",
            "lead",
            "--force",
            "--reason",
            "operator confirmed promotion",
            "--json",
        ],
    );
    assert_success(&forced);
    let body: Value = serde_json::from_slice(&forced.stdout).unwrap();
    assert_eq!(body["to_level"], "lead");
    assert_eq!(body["forced"], true);
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

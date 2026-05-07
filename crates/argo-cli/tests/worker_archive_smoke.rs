//! End-to-end lock for `argo worker archive/rehydrate/archive list`.
//! Drives the real `argo` binary against a temp store seeded with one
//! Worker, then archives → lists → rehydrates and asserts the live
//! row is restored.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn archive_list_rehydrate_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-archive-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "operator@example.test"),
        Role::new("Triage tickets."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();

    // archive
    let archive_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "archive",
            "add",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--reason",
            "campaign over",
            "--principal",
            "alice@team",
            "--json",
        ],
    );
    assert_success(&archive_out);
    let body: Value = serde_json::from_slice(&archive_out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["archived_by_principal"], "alice@team");
    assert_eq!(body["reason"], "campaign over");
    assert!(body["frozen_bytes"].as_u64().unwrap_or(0) > 0);

    // live worker is gone
    assert!(
        identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .is_none(),
        "live worker removed by archive"
    );

    // archive list
    let list_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "archive",
            "list",
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&list_out);
    let list: Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(list["count"], 1);
    let row = &list["rows"][0];
    assert_eq!(row["worker_id"], worker_id);
    assert_eq!(row["reason"], "campaign over");
    assert_eq!(row["archived_by_principal"], "alice@team");
    assert!(row["archived_at_rfc3339"].as_str().unwrap_or("").contains('T'));

    // rehydrate
    let rehydrate_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "rehydrate",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&rehydrate_out);
    let body: Value = serde_json::from_slice(&rehydrate_out.stdout).unwrap();
    assert_eq!(body["rehydrated"], true);
    assert_eq!(body["worker_id"], worker_id);

    // worker is back, persona/role preserved
    let restored = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .expect("worker rehydrated");
    assert_eq!(restored.id, worker.id);
    assert_eq!(restored.persona.name, "Maya");
    assert_eq!(restored.role.charter, "Triage tickets.");

    // archive list now empty
    let post_list = run(
        &home,
        work.path(),
        &[
            "worker",
            "archive",
            "list",
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&post_list);
    let list2: Value = serde_json::from_slice(&post_list.stdout).unwrap();
    assert_eq!(list2["count"], 0);
}

#[tokio::test]
async fn archive_then_re_archive_fails_cleanly() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-archive-smoke-2", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Avery", "op2@example.test"),
        Role::new("Handle escalations."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();

    // archive once
    let first = run(
        &home,
        work.path(),
        &[
            "worker",
            "archive",
            "add",
            &worker_id,
            "--tenant",
            &tenant_str,
        ],
    );
    assert!(first.status.success(), "first archive should succeed");

    // second archive: live row already gone, so the friendly error
    // is "no live worker" — the brief's "archive when no live worker
    // exists → error, no mutation" acceptance criterion.
    let second = run(
        &home,
        work.path(),
        &[
            "worker",
            "archive",
            "add",
            &worker_id,
            "--tenant",
            &tenant_str,
        ],
    );
    assert!(
        !second.status.success(),
        "second archive must error: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("not found")
            || String::from_utf8_lossy(&second.stderr).contains("no live worker")
    );
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

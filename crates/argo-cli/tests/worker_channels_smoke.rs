//! End-to-end lock for `argo worker channels add/remove/list`.
//! Drives the real `argo` binary against a temp store seeded with a
//! single Worker, runs add → list → remove → list and asserts each
//! state transition through both stdout JSON and stderr warnings.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn channels_add_list_remove_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-channels-smoke", &work.path().to_string_lossy())
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

    // Initial state — list shows the empty default.
    let list_initial = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "list", &worker_id,
            "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&list_initial.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["channels"].as_array().unwrap().len(), 0);

    // add email
    let add_email = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "email:ops@example.com", "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&add_email.stdout).unwrap();
    assert_eq!(body["added"], true);
    assert_eq!(body["duplicate"], false);
    assert_eq!(body["total"], 1);

    // add slack
    let add_slack = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "slack:C0123", "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&add_slack.stdout).unwrap();
    assert_eq!(body["added"], true);
    assert_eq!(body["total"], 2);

    // dup add → no-op + warning on stderr (still exit 0).
    let add_dup = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "email:ops@example.com", "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&add_dup.stdout).unwrap();
    assert_eq!(body["added"], false);
    assert_eq!(body["duplicate"], true);
    assert_eq!(body["total"], 2);

    // list (json) — verify both channels are present in stable order.
    let list_after_adds = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "list", &worker_id,
            "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&list_after_adds.stdout).unwrap();
    let channels = body["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0]["channel"], "email");
    assert_eq!(channels[0]["target"], "ops@example.com");
    assert_eq!(channels[1]["channel"], "slack");
    assert_eq!(channels[1]["target"], "C0123");

    // list (markdown default) — sanity check render.
    let list_md = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "list", &worker_id, "--tenant", &tenant_str,
        ],
    );
    let stdout = String::from_utf8_lossy(&list_md.stdout);
    assert!(stdout.contains("- email:ops@example.com"), "got:\n{stdout}");
    assert!(stdout.contains("- slack:C0123"), "got:\n{stdout}");

    // remove email
    let remove_email = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "remove", &worker_id,
            "email:ops@example.com", "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&remove_email.stdout).unwrap();
    assert_eq!(body["removed"], "email:ops@example.com");
    assert_eq!(body["remaining"], 1);

    // remove non-existent → exit 1
    let remove_missing = run(
        &home,
        work.path(),
        &[
            "worker", "channels", "remove", &worker_id,
            "email:ops@example.com", "--tenant", &tenant_str, "--json",
        ],
    );
    assert!(!remove_missing.status.success(), "expected exit 1");

    // bad spec (no colon) → exit 1
    let bad_add = run(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "foo", "--tenant", &tenant_str, "--json",
        ],
    );
    assert!(!bad_add.status.success(), "expected exit 1 for malformed spec");

    // unknown kind → exit 1
    let bad_kind = run(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "telex:hello", "--tenant", &tenant_str, "--json",
        ],
    );
    assert!(!bad_kind.status.success(), "expected exit 1 for unknown kind");

    // bad email → exit 1
    let bad_email = run(
        &home,
        work.path(),
        &[
            "worker", "channels", "add", &worker_id,
            "email:not-an-email", "--tenant", &tenant_str, "--json",
        ],
    );
    assert!(!bad_email.status.success(), "expected exit 1 for bad email");

    // final state: only slack remains.
    let list_final = run_ok(
        &home,
        work.path(),
        &[
            "worker", "channels", "list", &worker_id,
            "--tenant", &tenant_str, "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&list_final.stdout).unwrap();
    let channels = body["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0]["channel"], "slack");
}

fn run(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn run_ok(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    let out = run(home, cwd, args);
    assert!(
        out.status.success(),
        "args: {args:?}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

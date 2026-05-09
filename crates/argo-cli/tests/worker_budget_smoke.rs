//! End-to-end lock for `argo worker budget set/show`. Drives the real
//! `argo` binary against a temp store seeded with a single Worker; runs
//! set → show → clear → show and asserts each step's stdout JSON.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn budget_set_show_clear_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("worker-budget-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Mona", "operator@example.test"),
        Role::new("Watch the wallet."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();

    // initial show: no caps, no spend
    let initial = run_ok(
        &home,
        work.path(),
        &["worker", "budget", "show", &worker_id, "--json"],
    );
    let body: Value = serde_json::from_slice(&initial.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["caps"]["daily_micros"], Value::Null);
    assert_eq!(body["caps"]["monthly_micros"], Value::Null);
    assert_eq!(body["today_micros"], 0);

    // set both caps
    let set_both = run_ok(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--daily", "5", "--monthly", "100", "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&set_both.stdout).unwrap();
    assert_eq!(body["caps"]["daily_micros"], 5_000_000_u64);
    assert_eq!(body["caps"]["monthly_micros"], 100_000_000_u64);

    // decimal daily
    let set_decimal = run_ok(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--daily", "5.50", "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&set_decimal.stdout).unwrap();
    assert_eq!(body["caps"]["daily_micros"], 5_500_000_u64);
    assert_eq!(body["caps"]["monthly_micros"], 100_000_000_u64);

    // negative rejected
    let bad_negative = run(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--daily", "-1", "--json",
        ],
    );
    assert!(!bad_negative.status.success());

    // zero allowed (deny all)
    let zero_daily = run_ok(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--daily", "0", "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&zero_daily.stdout).unwrap();
    assert_eq!(body["caps"]["daily_micros"], 0);
    assert_eq!(body["caps"]["monthly_micros"], 100_000_000_u64);

    // idempotent re-set
    let zero_daily_again = run_ok(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--daily", "0", "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&zero_daily_again.stdout).unwrap();
    assert_eq!(body["caps"]["daily_micros"], 0);

    // markdown show — sanity render check
    let show_md = run_ok(
        &home,
        work.path(),
        &["worker", "budget", "show", &worker_id],
    );
    let stdout = String::from_utf8_lossy(&show_md.stdout);
    assert!(
        stdout.contains("daily   cap: $0.00 (deny all)"),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("monthly cap: $100.00"),
        "got:\n{stdout}"
    );

    // clear drops both caps
    let cleared = run_ok(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--clear", "--json",
        ],
    );
    let body: Value = serde_json::from_slice(&cleared.stdout).unwrap();
    assert_eq!(body["caps"]["daily_micros"], Value::Null);
    assert_eq!(body["caps"]["monthly_micros"], Value::Null);

    // clear + dimension flag is rejected
    let bad_combo = run(
        &home,
        work.path(),
        &[
            "worker", "budget", "set", &worker_id, "--clear", "--daily", "1",
        ],
    );
    assert!(!bad_combo.status.success());

    // bare set with no dimensions is rejected
    let bare = run(
        &home,
        work.path(),
        &["worker", "budget", "set", &worker_id],
    );
    assert!(!bare.status.success());

    // unknown worker id surfaces an error
    let ghost = run(
        &home,
        work.path(),
        &[
            "worker", "budget", "show", "00000000-0000-0000-0000-000000000000", "--json",
        ],
    );
    assert!(!ghost.status.success());
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

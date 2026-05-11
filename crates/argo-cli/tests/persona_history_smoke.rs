//! End-to-end lock for `argo persona history` + `argo persona revert`
//! (Stream `mission-persona-evolution-audit-revert`).
//!
//! Flow:
//!   1. seed a worker + persona
//!   2. record two persona revisions via the store API directly
//!      (the `edit` CLI doesn't wire `record_persona_change` yet —
//!      that's a follow-on brief — so this smoke covers the
//!      history/revert path independently)
//!   3. `argo persona history <worker_id> --json` shows 2 rows
//!   4. `argo persona revert <first_rev_id> --yes --json` succeeds
//!   5. the live persona row reflects the reverted value
//!   6. history now shows 3 rows (the revert itself appended).

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::persona_revision::record_persona_change;
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn persona_history_and_revert_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("persona-history-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Riley", "operator@example.test"),
        Role::new("Handle billing tickets."),
    );
    identities.create_worker(&worker).await.unwrap();
    let worker_id_str: String =
        sqlx::query_scalar("SELECT worker_id FROM persona WHERE name = ?")
            .bind("Riley")
            .fetch_one(store.pool())
            .await
            .unwrap();
    let worker_uuid = uuid::Uuid::parse_str(&worker_id_str).unwrap();

    // Capture the seeded persona's voice, then record two revisions
    // that move it forward in two steps.
    let seeded_voice: String =
        sqlx::query_scalar("SELECT voice_preamble FROM persona WHERE worker_id = ?")
            .bind(&worker_id_str)
            .fetch_one(store.pool())
            .await
            .unwrap();
    let first = record_persona_change(
        store.pool(),
        worker_uuid,
        "voice",
        &seeded_voice,
        "v1 voice",
        "test:setup",
    )
    .await
    .unwrap();
    // Mirror the recorded mutation in the live persona row so the
    // revert flow's "lose the current value" diff makes sense.
    sqlx::query("UPDATE persona SET voice_preamble = ? WHERE worker_id = ?")
        .bind("v1 voice")
        .bind(&worker_id_str)
        .execute(store.pool())
        .await
        .unwrap();
    let _second = record_persona_change(
        store.pool(),
        worker_uuid,
        "voice",
        "v1 voice",
        "v2 voice",
        "test:setup",
    )
    .await
    .unwrap();
    sqlx::query("UPDATE persona SET voice_preamble = ? WHERE worker_id = ?")
        .bind("v2 voice")
        .bind(&worker_id_str)
        .execute(store.pool())
        .await
        .unwrap();

    // history --json should show 2 revisions, newest first.
    let out = run(&home, work.path(), &["persona", "history", &worker_id_str, "--json"]);
    assert_success(&out);
    let history: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = history.as_array().expect("json array");
    assert_eq!(arr.len(), 2, "expected 2 revisions, got {arr:?}");
    assert_eq!(arr[0]["new_value"], "v2 voice", "newest first");

    // revert --yes --json the FIRST recorded revision (which means
    // restore the seeded voice).
    let out = run(
        &home,
        work.path(),
        &[
            "persona",
            "revert",
            &first.rev_id.to_string(),
            "--yes",
            "--json",
        ],
    );
    assert_success(&out);
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["field"], "voice");

    // Live persona row should now hold the seeded voice again.
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    let voice_after: String =
        sqlx::query_scalar("SELECT voice_preamble FROM persona WHERE worker_id = ?")
            .bind(&worker_id_str)
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(voice_after, seeded_voice);

    // history should now have 3 rows (the revert recorded itself).
    let out = run(
        &home,
        work.path(),
        &["persona", "history", &worker_id_str, "--json"],
    );
    assert_success(&out);
    let history: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = history.as_array().expect("json array");
    assert_eq!(arr.len(), 3, "revert must append a 3rd row");
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

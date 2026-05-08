//! End-to-end lock for `argo persona edit --in-place`.
//! Drives the real `argo` binary against a temp store seeded with a
//! single Worker (and therefore one persona row), reads back the
//! persona via SQL, asserts the editable subset was updated.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use sqlx::Row;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn persona_edit_in_place_updates_row() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("persona-edit-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Pippa", "operator@example.test"),
        Role::new("Field tickets."),
    );
    identities.create_worker(&worker).await.unwrap();

    // Snapshot original persona row.
    let row = sqlx::query("SELECT id, name, voice_preamble, bio FROM persona WHERE name = ?")
        .bind("Pippa")
        .fetch_one(store.pool())
        .await
        .unwrap();
    let original_id: String = row.get("id");
    let original_voice: String = row.get("voice_preamble");
    assert!(original_voice.contains("Pippa"), "seed voice: {original_voice}");

    // Write the new TOML.
    let fixture = work.path().join("persona.toml");
    std::fs::write(
        &fixture,
        "[persona]\nname = \"Pippa\"\nvoice = \"Be kind, then accurate.\"\nbio = \"Two-time customer-success champion.\"\n",
    )
    .unwrap();

    let out = run(
        &home,
        work.path(),
        &[
            "persona",
            "edit",
            "Pippa",
            "--in-place",
            fixture.to_str().unwrap(),
            "--json",
        ],
    );
    assert_success(&out);
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["name"], "Pippa");
    assert!(body["voice_bytes"].as_u64().unwrap() > 10);

    // Re-open store and verify persisted update.
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    let row = sqlx::query("SELECT name, voice_preamble, bio FROM persona WHERE id = ?")
        .bind(&original_id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    let voice: String = row.get("voice_preamble");
    let bio: Option<String> = row.get("bio");
    assert_eq!(voice, "Be kind, then accurate.");
    assert_eq!(bio.as_deref(), Some("Two-time customer-success champion."));
}

#[tokio::test]
async fn persona_edit_in_place_oversize_voice_errors_no_write() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("persona-edit-oversize", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Quinn", "operator@example.test"),
        Role::new("Field tickets."),
    );
    identities.create_worker(&worker).await.unwrap();
    let original_voice: String = sqlx::query_scalar(
        "SELECT voice_preamble FROM persona WHERE name = ?",
    )
    .bind("Quinn")
    .fetch_one(store.pool())
    .await
    .unwrap();

    // 4 KiB + 1 byte voice — must be rejected.
    let huge = "x".repeat(4 * 1024 + 1);
    let toml_body = format!("[persona]\nname = \"Quinn\"\nvoice = \"{huge}\"\n");
    let fixture = work.path().join("persona.toml");
    std::fs::write(&fixture, toml_body).unwrap();

    let out = run(
        &home,
        work.path(),
        &[
            "persona",
            "edit",
            "Quinn",
            "--in-place",
            fixture.to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "expected non-zero exit, got success. stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("voice exceeds"), "stderr: {stderr}");

    // Voice column must be unchanged.
    let voice_after: String = sqlx::query_scalar(
        "SELECT voice_preamble FROM persona WHERE name = ?",
    )
    .bind("Quinn")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(voice_after, original_voice, "row must NOT have been updated");
}

#[tokio::test]
async fn persona_edit_unknown_name_errors() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .ensure_workspace("persona-edit-missing", &work.path().to_string_lossy())
        .await
        .unwrap();
    IdentityStore::new(&store).ensure_schema().await.unwrap();

    let fixture = work.path().join("persona.toml");
    std::fs::write(
        &fixture,
        "[persona]\nname = \"Ghost\"\nvoice = \"Boo.\"\n",
    )
    .unwrap();

    let out = run(
        &home,
        work.path(),
        &[
            "persona",
            "edit",
            "Ghost",
            "--in-place",
            fixture.to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "expected non-zero exit. stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not found"),
        "stderr should mention not-found: {}",
        String::from_utf8_lossy(&out.stderr)
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

//! End-to-end smoke for `argo worker fleet` (Stream
//! `claude-worker-presence-tracking`). Seeds three workers with
//! distinct presence states (active, >30 days idle, never-seen)
//! and asserts the CLI surface — JSON envelope, table render,
//! `--limit`, and `--since` — matches the brief's acceptance.

mod common;

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::{worker_presence, SessionStore};
use chrono::Utc;
use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn worker_fleet_lists_presence_and_filters() {
    let world = seed_world().await;

    let listed = run(&world, &["worker", "fleet", "--json"]);
    assert_success(&listed);
    let body: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["count"], 3);
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows[0]["worker_id"], world.recent_worker);
    assert_eq!(rows[0]["status"], "active");
    assert_eq!(rows[1]["worker_id"], world.idle_worker);
    assert_eq!(rows[1]["status"], "idle");
    assert_eq!(rows[2]["worker_id"], world.never_worker);
    assert_eq!(rows[2]["status"], "idle");
    assert!(rows[2]["last_active_at"].is_null());
    assert!(rows[2]["last_active_at_rfc3339"].is_null());
    assert!(rows[0]["last_active_at_rfc3339"]
        .as_str()
        .unwrap()
        .contains('T'));

    let recent = run(&world, &["worker", "fleet", "--since", "2d", "--json"]);
    assert_success(&recent);
    let body: Value = serde_json::from_slice(&recent.stdout).unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["rows"][0]["worker_id"], world.recent_worker);

    let limited = run(&world, &["worker", "fleet", "--limit", "1", "--json"]);
    assert_success(&limited);
    let body: Value = serde_json::from_slice(&limited.stdout).unwrap();
    assert_eq!(body["count"], 1);

    let table = run(&world, &["worker", "fleet"]);
    assert_success(&table);
    let stdout = String::from_utf8_lossy(&table.stdout);
    assert!(stdout.contains("worker_id"));
    assert!(stdout.contains("active"));
    assert!(stdout.contains("idle"));
    assert!(stdout.contains("(never seen)"));
}

struct World {
    home: TempDir,
    work: TempDir,
    recent_worker: String,
    idle_worker: String,
    never_worker: String,
}

async fn seed_world() -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _workspace) = store
        .ensure_workspace("worker-fleet-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    worker_presence::ensure_schema(store.pool()).await.unwrap();

    let recent_worker = create_worker(&identities, tenant, "Maya").await;
    let idle_worker = create_worker(&identities, tenant, "Noah").await;
    let never_worker = create_worker(&identities, tenant, "Ari").await;
    let now = Utc::now().timestamp();
    set_last_active(store.pool(), &recent_worker, now).await;
    set_last_active(store.pool(), &idle_worker, now - 31 * 24 * 60 * 60).await;

    World {
        home,
        work,
        recent_worker,
        idle_worker,
        never_worker,
    }
}

async fn create_worker(
    identities: &IdentityStore<'_>,
    tenant: argo_types::TenantId,
    persona: &str,
) -> String {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(persona, "operator@example.test"),
        Role::new("Handle customer follow-up."),
    );
    identities.create_worker(&worker).await.unwrap();
    worker.id.to_string()
}

async fn set_last_active(pool: &sqlx::SqlitePool, worker_id: &str, at: i64) {
    sqlx::query("UPDATE worker SET last_active_at = ? WHERE id = ?")
        .bind(at)
        .bind(worker_id)
        .execute(pool)
        .await
        .unwrap();
}

fn run(world: &World, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", world.home.path())
        .current_dir(world.work.path())
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

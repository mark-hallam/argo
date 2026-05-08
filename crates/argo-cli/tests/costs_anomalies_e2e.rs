mod common;

use argo_store::SessionStore;
use chrono::{Duration, Utc};
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn run(home: &Path, work: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(work)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output) {
    assert!(
        out.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

async fn seed(home: &Path, rows: usize) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    argo_store::spend_anomaly::ensure_schema(store.pool())
        .await
        .unwrap();
    sqlx::query(include_str!(
        "../../argo-store/migrations/12__worker_identity.sql"
    ))
    .execute(store.pool())
    .await
    .unwrap();
    let now = Utc::now();
    for i in 0..rows {
        let tenant = if i % 2 == 0 { "tenant-a" } else { "tenant-b" };
        let worker = format!("worker-{i}");
        sqlx::query(
            "INSERT INTO worker
             (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
              auto_approval_json, channels_json, created_at)
             VALUES (?, ?, 'gtm', 'persona', 'role', 'scope', '{}', '[]', 0)",
        )
        .bind(&worker)
        .bind(tenant)
        .execute(store.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO worker_spend_anomaly
             (id, worker_id, observed_usd_per_hour, baseline_mean_usd, baseline_stddev_usd,
              z_score, hour_of_day, observed_at_micros)
             VALUES (?, ?, ?, 2.0, 1.0, ?, 13, ?)",
        )
        .bind(format!("anom-{i}"))
        .bind(&worker)
        .bind(10.0 + i as f64)
        .bind(4.5 + i as f64)
        .bind((now - Duration::minutes(i as i64)).timestamp_micros())
        .execute(store.pool())
        .await
        .unwrap();
    }
}

#[test]
fn empty_state_prints_markdown_message() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(seed(home.path(), 0));
    let out = run(home.path(), work.path(), &["costs", "anomalies"]);
    assert_ok(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("No anomalies in window"));
}

#[test]
fn markdown_lists_recent_rows_with_limit() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(seed(home.path(), 5));
    let out = run(
        home.path(),
        work.path(),
        &["costs", "anomalies", "--limit", "3"],
    );
    assert_ok(&out);
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(body.contains("| timestamp | tenant_id | worker |"));
    assert!(body.contains("worker-0"));
    assert!(body.contains("worker-2"));
    assert!(!body.contains("worker-3"));
}

#[test]
fn json_filters_since_and_tenant() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(seed(home.path(), 5));
    let since = (Utc::now() - Duration::minutes(3)).to_rfc3339();
    let out = run(
        home.path(),
        work.path(),
        &[
            "costs",
            "anomalies",
            "--format",
            "json",
            "--tenant",
            "tenant-a",
            "--since",
            &since,
        ],
    );
    assert_ok(&out);
    let rows: Value = serde_json::from_slice(&out.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r["tenant_id"] == "tenant-a"));
    assert!(rows.iter().any(|r| r["worker_id"] == "worker-0"));
    assert!(rows.iter().any(|r| r["worker_id"] == "worker-2"));
}

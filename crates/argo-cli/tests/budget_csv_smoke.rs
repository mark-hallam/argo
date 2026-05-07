//! Smoke tests for `argo budget --format csv`.

mod common;

use argo_store::SessionStore;
use chrono::Utc;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[test]
fn budget_csv_outputs_parse_for_all_readers() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".skycot")).unwrap();
    let worker = "worker, \"quoted\"\nrow";
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(seed(home.path(), worker));

    assert_csv_rows(
        run_ok(
            &home,
            workspace.path(),
            &["budget", "show", worker, "--format", "csv"],
        ),
        &["worker_id", "today_usd_micros", "last_charge_count"],
        1,
    );
    assert_csv_rows(
        run_ok(
            &home,
            workspace.path(),
            &["budget", "recent", "--worker", worker, "--format", "csv"],
        ),
        &["worker_id", "action_id", "charged_at"],
        2,
    );
    assert_csv_rows(
        run_ok(
            &home,
            workspace.path(),
            &["budget", "anomalies", "--worker", worker, "--format", "csv"],
        ),
        &["id", "worker_id", "z_score"],
        1,
    );
    assert_csv_rows(
        run_ok(
            &home,
            workspace.path(),
            &[
                "budget", "alerts", "list", "--worker", worker, "--format", "csv",
            ],
        ),
        &["id", "worker_id", "notified_via"],
        1,
    );

    let human = run_ok(
        &home,
        workspace.path(),
        &["budget", "recent", "--worker", worker, "--format", "human"],
    );
    assert!(String::from_utf8_lossy(&human.stdout).contains("budget recent"));
    let formatted_json = run_json(
        &home,
        workspace.path(),
        &["budget", "show", worker, "--format", "json"],
    );
    assert_eq!(formatted_json["worker_id"], worker);
    let legacy_json = run_json(
        &home,
        workspace.path(),
        &["budget", "show", worker, "--json"],
    );
    assert_eq!(legacy_json["worker_id"], worker);
}

async fn seed(home: &Path, worker: &str) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    apply_schema(
        &store,
        include_str!("../../argo-store/migrations/14__worker_budget.sql"),
    )
    .await;
    argo_store::spend_anomaly::ensure_schema(store.pool())
        .await
        .unwrap();
    apply_schema(
        &store,
        include_str!("../../argo-store/migrations/26__worker_budget_alert.sql"),
    )
    .await;
    let now = Utc::now();
    for (idx, amount) in [1_250_000_i64, 2_500_000].into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(vec![0_u8; 16])
        .bind(worker)
        .bind(format!("act-{idx}"))
        .bind(amount)
        .bind(now.timestamp() - i64::try_from(idx).unwrap())
        .execute(store.pool())
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO worker_spend_anomaly
         (id, worker_id, observed_usd_per_hour, baseline_mean_usd, baseline_stddev_usd,
          z_score, hour_of_day, observed_at_micros)
         VALUES ('anom-1', ?, 10.5, 2.25, 1.5, 5.5, 13, ?)",
    )
    .bind(worker)
    .bind(now.timestamp_micros())
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO worker_budget_alert
         (id, worker_id, threshold, fired_on, fired_at, spend_amount, headroom_amount,
          notified_via, synthetic)
         VALUES ('alert-1', ?, 100, ?, ?, 1250000, 0, ?, 1)",
    )
    .bind(worker)
    .bind(now.date_naive().to_string())
    .bind(now.timestamp())
    .bind("push,\"mobile\"\nbackup")
    .execute(store.pool())
    .await
    .unwrap();
}

async fn apply_schema(store: &SessionStore, schema: &str) {
    for stmt in schema.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql).execute(store.pool()).await.unwrap();
        }
    }
}

fn assert_csv_rows(output: Output, required_headers: &[&str], expected_rows: usize) {
    let mut reader = csv::Reader::from_reader(output.stdout.as_slice());
    let headers = reader.headers().unwrap().clone();
    for header in required_headers {
        assert!(headers.iter().any(|h| h == *header), "missing {header}");
    }
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.len(), expected_rows);
}

fn run_json(home: &TempDir, cwd: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&run_ok(home, cwd, args).stdout).unwrap()
}

fn run_ok(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

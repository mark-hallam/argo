//! Schema migration v35 — worker spend anomaly events.
//!
//! Reconciles raw SQL migration `27__spend_anomaly.sql` (Stream
//! `claude-spend-anomaly-alerts`). Single table
//! `worker_spend_anomaly` carrying anomaly events fired by the
//! hourly z-score check. One row per (worker, hour) bucket whose
//! observed spend exceeds the configured |z| threshold against
//! the baseline-for-hour-of-day (see v32 for the baseline table).
//!
//! 27-collision split: this slot pairs with v34 above (handoff
//! auctions). See v34 doc comment for ordering rationale.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and its index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V35_UP {
        exec(tx, "v35 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V35_DOWN {
        exec(tx, "v35 down", sql).await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

const V35_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_spend_anomaly (
        id TEXT PRIMARY KEY,
        worker_id TEXT NOT NULL,
        observed_usd_per_hour REAL NOT NULL,
        baseline_mean_usd REAL NOT NULL,
        baseline_stddev_usd REAL NOT NULL,
        z_score REAL NOT NULL,
        hour_of_day INTEGER NOT NULL,
        observed_at_micros INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_spend_anomaly_worker_recent
     ON worker_spend_anomaly(worker_id, observed_at_micros DESC)",
];

const V35_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_spend_anomaly_worker_recent",
    "DROP TABLE IF EXISTS worker_spend_anomaly",
];

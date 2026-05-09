//! Schema migration v32 — per-(worker, hour-of-day) spend baseline.
//!
//! Reconciles raw SQL migration `26__spend_baseline.sql` (Stream
//! `claude-spend-anomaly-alerts`). Single table
//! `worker_spend_baseline` carrying Welford rolling-stats samples
//! used by the hourly z-score check. One row per (worker_id,
//! hour_of_day).
//!
//! 26-collision split: this slot pairs with v31 above and v33
//! below (see v31 doc comment for ordering rationale).
//!
//! Idempotent: CREATE TABLE IF NOT EXISTS. The raw SQL ships no
//! indexes, only the composite primary key. Rollback drops the
//! table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V32_UP {
        exec(tx, "v32 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V32_DOWN {
        exec(tx, "v32 down", sql).await?;
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

const V32_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_spend_baseline (
        worker_id TEXT NOT NULL,
        hour_of_day INTEGER NOT NULL,
        mean_usd_per_hour REAL NOT NULL,
        m2_usd_per_hour REAL NOT NULL,
        sample_count INTEGER NOT NULL,
        updated_at_micros INTEGER NOT NULL,
        PRIMARY KEY (worker_id, hour_of_day)
    )",
];

const V32_DOWN: &[&str] = &["DROP TABLE IF EXISTS worker_spend_baseline"];

//! Schema migration v33 — worker budget alert push notifications.
//!
//! Reconciles raw SQL migration `26__worker_budget_alert.sql`
//! (Stream `codex-budget-alert-channel`). Single table
//! `worker_budget_alert` recording each push notification fired
//! when worker spend crosses a configured budget threshold;
//! includes a partial unique index for non-synthetic dedup.
//!
//! 26-collision split: this slot pairs with v31 and v32 above
//! (see v31 doc comment for ordering rationale).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and all three indexes (the partial dedup unique
//! index, the recent-fire index, and the per-worker index).
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V33_UP {
        exec(tx, "v33 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V33_DOWN {
        exec(tx, "v33 down", sql).await?;
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

const V33_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_budget_alert (
        id TEXT PRIMARY KEY,
        worker_id TEXT NOT NULL,
        threshold INTEGER NOT NULL,
        fired_on TEXT NOT NULL,
        fired_at INTEGER NOT NULL,
        spend_amount INTEGER NOT NULL,
        headroom_amount INTEGER NOT NULL,
        notified_via TEXT NOT NULL,
        synthetic INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX IF NOT EXISTS idx_worker_budget_alert_recent
     ON worker_budget_alert(fired_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_worker_budget_alert_worker
     ON worker_budget_alert(worker_id, fired_at DESC)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_budget_alert_dedup
     ON worker_budget_alert(worker_id, threshold, fired_on)
     WHERE synthetic = 0",
];

const V33_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_budget_alert_dedup",
    "DROP INDEX IF EXISTS idx_worker_budget_alert_worker",
    "DROP INDEX IF EXISTS idx_worker_budget_alert_recent",
    "DROP TABLE IF EXISTS worker_budget_alert",
];

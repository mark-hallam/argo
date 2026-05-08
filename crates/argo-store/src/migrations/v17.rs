//! Schema migration v17 — `worker_circuit_history` audit trail.
//!
//! Reconciles raw SQL migration `17__worker_circuit_history.sql`.
//! The latest row per `worker_id` is the current circuit-breaker
//! state; earlier rows are kept as durable audit history.
//!
//! Rollback drops the table and its index. The table is purely
//! observational — no other reconciled migration in v16..v21
//! depends on it — so unwinding v17 in tests does not corrupt
//! orchestrator or anomaly state.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V17_UP {
        exec(tx, "v17 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V17_DOWN {
        exec(tx, "v17 down", sql).await?;
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

const V17_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_circuit_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        worker_id TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('closed', 'open', 'half_open')),
        failure_count INTEGER NOT NULL DEFAULT 0,
        reason TEXT,
        created_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_worker_circuit_history_worker
     ON worker_circuit_history(worker_id, created_at, id)",
];

const V17_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_circuit_history_worker",
    "DROP TABLE IF EXISTS worker_circuit_history",
];

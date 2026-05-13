//! Schema migration v65 — `vertical_locks` advisory lock table.
//!
//! Backs `argo_orchestrator::VerticalConcurrencyLock`, a
//! single-host SQLite mutex keyed by `(tenant_id, vertical_id,
//! worker_id)`.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS vertical_locks (
            tenant_id TEXT NOT NULL,
            vertical_id TEXT NOT NULL,
            worker_id TEXT NOT NULL,
            holder_pid INTEGER NOT NULL,
            acquired_at INTEGER NOT NULL,
            lease_secs INTEGER NOT NULL,
            PRIMARY KEY (tenant_id, vertical_id, worker_id)
        ) STRICT, WITHOUT ROWID",
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("v65 up: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS vertical_locks")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v65 down: {e}")))?;
    Ok(())
}

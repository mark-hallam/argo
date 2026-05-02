//! Schema migration v5 — `forge_queue` association table for
//! the v1.1 forge-daemon split. Enqueued forge runs land in
//! `forge_run` with status='queued' and a parallel
//! `forge_queue` row carrying the JSON payload (skill,
//! examples_path, iterations, budget_usd) the daemon needs to
//! actually run the work.
//!
//! Kept as a separate table (rather than a column on
//! forge_run) so the down migration is a clean `DROP TABLE`
//! and so the payload doesn't bloat the read-side `/forge/runs`
//! response.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V5_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v5 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V5_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v5 down: {e}")))?;
    }
    Ok(())
}

const V5_UP: &[&str] = &[
    "CREATE TABLE forge_queue (
        forge_run_id BLOB PRIMARY KEY REFERENCES forge_run(id) ON DELETE CASCADE,
        payload_json TEXT NOT NULL,
        enqueued_at  INTEGER NOT NULL
    )",
    "CREATE INDEX idx_forge_queue_enqueued_at ON forge_queue(enqueued_at)",
];

const V5_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_forge_queue_enqueued_at",
    "DROP TABLE IF EXISTS forge_queue",
];

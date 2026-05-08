//! Schema migration v15 — tenant-scoped shared-team memory.
//!
//! Structurally idempotent version of raw SQL migration
//! `15__shared_team_memory.sql`.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V15_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v15 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Err(ArgoError::Provider(
        "v15 rollback unsupported: raw migration reconciliation is additive and may own live data"
            .into(),
    ))
}

const V15_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS shared_team_memory (
        id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
        tenant_id        BLOB NOT NULL DEFAULT (zeroblob(16)),
        author_worker_id BLOB NOT NULL DEFAULT (zeroblob(16)),
        content          TEXT NOT NULL DEFAULT '',
        written_at       INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX IF NOT EXISTS idx_shared_team_memory_tenant_written
     ON shared_team_memory(tenant_id, written_at, id)",
    "CREATE INDEX IF NOT EXISTS idx_shared_team_memory_author
     ON shared_team_memory(tenant_id, author_worker_id, written_at)",
];

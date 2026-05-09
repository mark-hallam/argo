//! Schema migration v36 — recovery suggestion cache.
//!
//! Reconciles raw SQL migration `28__recovery_cache.sql` (Stream
//! `claude-error-recovery-suggestions`). Single table
//! `recovery_cache` keyed on (`session_id`, `chain_hash`) so that a
//! re-run with the same audit chain skips the LLM call. New audit
//! rows produce a new chain_hash → cache miss.
//!
//! 28-collision split: this slot pairs with v37 (role_duties) and
//! v38 (tenant_quota). See v37 doc comment for ordering rationale.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and its two indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V36_UP {
        exec(tx, "v36 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V36_DOWN {
        exec(tx, "v36 down", sql).await?;
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

const V36_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS recovery_cache (
        session_id TEXT NOT NULL,
        chain_hash TEXT NOT NULL,
        suggestions_json TEXT NOT NULL,
        failure_kind TEXT NOT NULL DEFAULT '',
        cached_at_micros INTEGER NOT NULL,
        PRIMARY KEY (session_id, chain_hash)
    )",
    "CREATE INDEX IF NOT EXISTS idx_recovery_cache_session
     ON recovery_cache(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_recovery_cache_cached_at
     ON recovery_cache(cached_at_micros DESC)",
];

const V36_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_recovery_cache_cached_at",
    "DROP INDEX IF EXISTS idx_recovery_cache_session",
    "DROP TABLE IF EXISTS recovery_cache",
];

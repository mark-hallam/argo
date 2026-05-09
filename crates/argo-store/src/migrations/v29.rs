//! Schema migration v29 — vertical lifecycle (archive marker).
//!
//! Reconciles raw SQL migration `24__vertical_archived.sql`
//! (Stream `codex-vertical-archive-cli`). Single table
//! `vertical_lifecycle` with `archived_at` soft-delete marker
//! per installed vertical.
//!
//! 24-collision split: this slot pairs with v27 and v28 above
//! (see v27 doc comment for ordering rationale).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and its index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V29_UP {
        exec(tx, "v29 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V29_DOWN {
        exec(tx, "v29 down", sql).await?;
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

const V29_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS vertical_lifecycle (
        vertical_id TEXT PRIMARY KEY,
        archived_at INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_vertical_lifecycle_archived
     ON vertical_lifecycle(archived_at)",
];

const V29_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_vertical_lifecycle_archived",
    "DROP TABLE IF EXISTS vertical_lifecycle",
];

//! Schema migration v30 — registered policy files.
//!
//! Reconciles raw SQL migration `25__policy_files.sql` (Stream
//! `claude-policy-as-code`). Single table `policy_files`
//! recording when each `<workspace>/policies/<name>.yaml` was
//! last applied + a body sha so operators can spot drift.
//!
//! No collision: `25__` is a single-prefix entry between the 24
//! and 26 collision groups. Idempotent: CREATE TABLE/INDEX IF
//! NOT EXISTS. Rollback drops the table and its index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V30_UP {
        exec(tx, "v30 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V30_DOWN {
        exec(tx, "v30 down", sql).await?;
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

const V30_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS policy_files (
        workspace_id BLOB NOT NULL,
        name TEXT NOT NULL,
        body_sha256 TEXT NOT NULL,
        applied_at_micros INTEGER NOT NULL,
        PRIMARY KEY (workspace_id, name)
    )",
    "CREATE INDEX IF NOT EXISTS idx_policy_files_applied_at
     ON policy_files(applied_at_micros DESC)",
];

const V30_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_policy_files_applied_at",
    "DROP TABLE IF EXISTS policy_files",
];

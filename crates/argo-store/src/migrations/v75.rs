//! Schema migration v75 — per-target glob on `session_grant`
//! (mission-pm-approval-target-scope).
//!
//! Adds an optional `target_glob TEXT` column. `NULL` means the
//! grant matches any target (back-compat with v7/v8 rows); a glob
//! string narrows the grant to dispatches whose `target` payload
//! satisfies the same glob library used by `Capability::ToolDispatch`.
//!
//! Also creates a non-unique compound index on
//! `(session_id, tool_name, target_glob)` so the consume-side
//! "find most-specific matching grant" query stays O(active).
//!
//! Down migration drops the column and the index. Existing rows
//! are unaffected by the up migration because the column is added
//! with no default → SQLite back-fills `NULL`.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V75_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v75 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V75_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v75 down: {e}")))?;
    }
    Ok(())
}

const V75_UP: &[&str] = &[
    "ALTER TABLE session_grant ADD COLUMN target_glob TEXT",
    "CREATE INDEX IF NOT EXISTS idx_session_grant_target \
     ON session_grant(session_id, tool_name, target_glob)",
];

const V75_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_session_grant_target",
    "ALTER TABLE session_grant DROP COLUMN target_glob",
];

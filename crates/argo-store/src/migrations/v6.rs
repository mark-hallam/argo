//! Schema migration v6 — additive `session.vertical_id TEXT`
//! column for the vertical-template binding (#651).
//!
//! Wholly additive: existing sessions stay valid (column is
//! nullable, default NULL). The string is the vertical's `id`
//! slug (matches `[vertical].id` in the manifest); TEXT not BLOB
//! per the design call so SQL filters / `lower(hex(...))`
//! gymnastics aren't needed.
//!
//! Down migration drops the column (SQLite supports DROP COLUMN
//! since 3.35; the project's sqlx ships with a newer build).
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V6_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v6 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V6_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v6 down: {e}")))?;
    }
    Ok(())
}

const V6_UP: &[&str] = &[
    "ALTER TABLE session ADD COLUMN vertical_id TEXT",
    // Index so `WHERE vertical_id = ?` is cheap when filtering
    // session lists by vertical (CLI: `argo session list
    // --vertical <id>`; gateway: `?vertical=<id>`).
    "CREATE INDEX idx_session_vertical_id ON session(vertical_id)",
];

const V6_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_session_vertical_id",
    "ALTER TABLE session DROP COLUMN vertical_id",
];

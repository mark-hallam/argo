//! Schema migration v8 — one-shot semantics for `session_grant`
//! (#667). Adds two columns to the table created by v7:
//!
//! - `consumed_at INTEGER NULL` — Unix epoch seconds when a
//!   one-shot grant was consumed. NULL for unconsumed grants
//!   and for non-one-shot grants (which never consume).
//! - `is_one_shot INTEGER NOT NULL DEFAULT 0` — boolean (SQLite
//!   doesn't have a real bool; 0=persistent, 1=one-shot). The
//!   default-0 means existing v7 grants stay persistent — wholly
//!   additive.
//!
//! Plus an index on `(session_id, consumed_at)` so the per-
//! TurnStart "list active (unconsumed) grants" query stays
//! O(active) instead of O(history).
//!
//! Down migration drops both columns + the new index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V8_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v8 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V8_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v8 down: {e}")))?;
    }
    Ok(())
}

const V8_UP: &[&str] = &[
    "ALTER TABLE session_grant ADD COLUMN consumed_at INTEGER",
    "ALTER TABLE session_grant ADD COLUMN is_one_shot INTEGER NOT NULL DEFAULT 0",
    // Hot path: the agent runner walks "give me unconsumed
    // grants for this session". Index supports the WHERE clause
    // even when consumed_at column has many NULLs.
    "CREATE INDEX idx_session_grant_active \
     ON session_grant(session_id, consumed_at)",
];

const V8_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_session_grant_active",
    "ALTER TABLE session_grant DROP COLUMN is_one_shot",
    "ALTER TABLE session_grant DROP COLUMN consumed_at",
];

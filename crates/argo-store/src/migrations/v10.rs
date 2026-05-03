//! Schema migration v10 — additive `memory_entry.vertical_id`
//! column for vertical-namespaced memory (#680, Rule 7).
//!
//! Background: `memory_entry` rows are scoped by `workspace_id`
//! and `file` (one of MEMORY/USER/SOUL/AGENTS), but have no
//! linkage to the bound vertical. A session bound to vertical
//! A and a session bound to vertical B in the same workspace
//! see the same memory rows — Rule 7 ("memory reads/writes
//! must use the vertical namespace by default") is violated.
//!
//! This migration adds `vertical_id TEXT NULL`. Existing rows
//! stay valid (NULL = legacy / workspace-global). Writes via
//! the new `MemoryStore::append_for_vertical` populate the
//! column; lookups via `MemoryStore::list_for_vertical` filter
//! by it. The bare `append` / `list_active` paths keep their
//! current behaviour for backward compat — gateway / CLI
//! integration into the vertical-aware path is a follow-up.
//!
//! Down migration drops the column + the supporting index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V10_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v10 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V10_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v10 down: {e}")))?;
    }
    Ok(())
}

const V10_UP: &[&str] = &[
    "ALTER TABLE memory_entry ADD COLUMN vertical_id TEXT",
    // Per-vertical memory lookup is the hot path the agent
    // runner walks each turn. Index `(workspace_id, file,
    // vertical_id)` so the WHERE filter is O(rows in scope).
    "CREATE INDEX idx_memory_entry_vertical \
     ON memory_entry(workspace_id, file, vertical_id)",
];

const V10_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_memory_entry_vertical",
    "ALTER TABLE memory_entry DROP COLUMN vertical_id",
];

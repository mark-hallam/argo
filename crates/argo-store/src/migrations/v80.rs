//! Schema migration v80 — add `tenant_id` to the `skill` table.
//! Stream `mission-skill-table-add-tenant-id-column`. Source: evaluator
//! round-9 PR #2081 finding §2 P0 #2.3.
//!
//! The schema body lives in `migrations/131__skill_tenant_id.sql`;
//! this wrapper applies it via the versioned migration runner so
//! existing workspaces upgrade through the same path as v79 / v78.
//!
//! Slot rationale: v79 (`skill_ab_test_run`) is the highest versioned
//! slot on origin/main since the paired #2101 landed first; this
//! brief bumps to v80 per the swarm-protocol "second-to-land bumps"
//! rule (also documented in `v79.rs`). The SQL filename is 131 —
//! independently assigned (see `scripts/check-migration-slots.sh`).
//!
//! Down-migration drops both the index and the column. SQLite's
//! ALTER TABLE DROP COLUMN works for additive columns since 3.35
//! (April 2021); the runtime ships modern sqlx which targets a
//! recent enough libsqlite.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V80_UP_SQL: &str = include_str!("../../migrations/131__skill_tenant_id.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V80_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v80 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V80_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v80 down: {e}")))?;
    }
    Ok(())
}

const V80_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_tenant_id",
    "ALTER TABLE skill DROP COLUMN tenant_id",
];

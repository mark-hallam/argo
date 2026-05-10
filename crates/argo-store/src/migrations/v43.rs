//! Schema migration v43 — `skill_canary` table for canary rollouts
//! of promoted skill versions (Stream `claude-skill-canary-deployment`).
//!
//! Wraps raw SQL `47__skill_canary.sql` so a single source of truth
//! covers both `argo-store::SessionStore::migrate` and the
//! crate-scoped `ensure_*_schema` patterns (mirrors `v21_skill_ab_tests.rs`).
//!
//! Slot ordering: this is the next free `schema_version` slot after
//! v42 (`mvp-audit-principal-stamp`). The raw-SQL filename uses slot
//! 47 because filename slots 41..46 are already occupied on main by
//! crate-scoped helpers (`roster_role_removed_at`, `worker_presence`,
//! `worker_objective`, `worker_key_result`, `cu_screenshot_index` /
//! `worker_dependency` / `worker_level`, `audit_principal`). Per the
//! v42 docstring, the two counters move independently: `schema_version`
//! is the next free wrapper slot, and the SQL filename is the next
//! free filename slot.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V43_UP_SQL: &str = include_str!("../../migrations/47__skill_canary.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for stmt in V43_UP_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v43 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V43_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v43 down: {e}")))?;
    }
    Ok(())
}

const V43_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_canary_expires",
    "DROP TABLE IF EXISTS skill_canary",
];

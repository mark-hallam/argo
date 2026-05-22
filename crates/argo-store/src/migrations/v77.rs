//! Schema migration v77 — tenant-scoped workplace-knowledge wiki
//! (Stream `mission-workplace-knowledge-substrate`).
//!
//! Adds two tables — `workplace_knowledge_doc` (head row per wiki
//! doc, `UNIQUE(tenant_id, slug)`) and `workplace_knowledge_revision`
//! (append-only edit history) — plus the two indexes that drive the
//! tenant-category list query and per-doc revision walk.
//!
//! The body lives in `migrations/129__workplace_knowledge.sql`; this
//! wrapper applies it through the versioned migration runner so
//! existing workspaces upgrade in lockstep with new ones. Mirrors
//! the v75/v76 wrappers.
//!
//! Slot rationale: v76 (`notifications`) is the highest versioned
//! slot on origin/main. Raw SQL filenames 126/127/128 were taken by
//! lazy-bootstrap (`ensure_schema`) tables that do not need a
//! versioned counter entry; this brief takes the next free filename
//! slot, 129. The two counters move independently per the v55
//! docstring.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V77_UP_SQL: &str = include_str!("../../migrations/129__workplace_knowledge.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V77_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v77 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V77_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v77 down: {e}")))?;
    }
    Ok(())
}

const V77_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_workplace_knowledge_revision_doc",
    "DROP TABLE IF EXISTS workplace_knowledge_revision",
    "DROP INDEX IF EXISTS idx_workplace_knowledge_doc_tenant_category",
    "DROP TABLE IF EXISTS workplace_knowledge_doc",
];

//! Schema migration v72 — workflow definition persistence layer.
//!
//! Adds `workflow_definition` (one row per workflow version) plus
//! `workflow_definition_index` (per `(tenant, vertical, name)` head
//! pointer) for Stream `mission-pm-workflow-persistence-schema`.
//! Downstream edit / rollback / diff briefs (P1 #6, P1 #7, P2 #9,
//! P2 #10) all build against this schema, so the column shape is the
//! contract. Files remain authoritative for v1 workflow loads; the
//! DB layer is additive until the edit API lands.
//!
//! Slot rationale: v71 is the highest registered version on
//! origin/main. Raw SQL slot 117 (`approval_draft_content`) is the
//! highest committed filename; slot 118 is the next free slot.
//! Filename and version counters advance independently per the v55
//! convention.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V72_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v72 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V72_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v72 down: {e}")))?;
    }
    Ok(())
}

const V72_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS workflow_definition (
        id              TEXT    PRIMARY KEY,
        tenant_id       TEXT    NOT NULL,
        vertical_id     TEXT    NOT NULL,
        name            TEXT    NOT NULL,
        definition_toml TEXT    NOT NULL,
        version         INTEGER NOT NULL,
        parent_id       TEXT,
        lifecycle_state TEXT    NOT NULL DEFAULT 'draft',
        created_at      TEXT    NOT NULL,
        modified_by     TEXT    NOT NULL,
        UNIQUE (tenant_id, vertical_id, name, version)
    )",
    "CREATE INDEX IF NOT EXISTS idx_workflow_definition_lookup \
     ON workflow_definition(tenant_id, vertical_id, name)",
    "CREATE INDEX IF NOT EXISTS idx_workflow_definition_parent \
     ON workflow_definition(parent_id)",
    "CREATE TABLE IF NOT EXISTS workflow_definition_index (
        tenant_id   TEXT NOT NULL,
        vertical_id TEXT NOT NULL,
        name        TEXT NOT NULL,
        head_id     TEXT NOT NULL,
        PRIMARY KEY (tenant_id, vertical_id, name)
    )",
];

const V72_DOWN: &[&str] = &[
    "DROP TABLE IF EXISTS workflow_definition_index",
    "DROP INDEX IF EXISTS idx_workflow_definition_parent",
    "DROP INDEX IF EXISTS idx_workflow_definition_lookup",
    "DROP TABLE IF EXISTS workflow_definition",
];

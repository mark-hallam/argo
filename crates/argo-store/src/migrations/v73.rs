//! Schema migration v73 — worker freeze state.
//!
//! Adds the tenant-scoped `worker_freeze` table for Stream
//! `mission-pm-kill-switch-1-store-audit`. This is the persistence
//! layer only: runner checks, CLI, gateway routes, and UI controls land
//! in sibling kill-switch slices.
//!
//! Slot rationale: v72 is the highest registered version on
//! origin/main. Raw SQL slot 118 (`workflow_definition`) is the
//! highest committed filename; slot 119 is the next free slot.
//! Filename and version counters advance independently per the v55
//! convention.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V73_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v73 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V73_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v73 down: {e}")))?;
    }
    Ok(())
}

const V73_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_freeze (
        tenant_id    TEXT      NOT NULL,
        worker_id    TEXT      NOT NULL,
        frozen_at    TIMESTAMP NOT NULL,
        frozen_by    TEXT      NOT NULL,
        reason       TEXT      NOT NULL,
        unfrozen_at  TIMESTAMP NULL,
        unfrozen_by  TEXT      NULL,
        PRIMARY KEY (tenant_id, worker_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_worker_freeze_active \
     ON worker_freeze(tenant_id, unfrozen_at)",
];

const V73_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_freeze_active",
    "DROP TABLE IF EXISTS worker_freeze",
];

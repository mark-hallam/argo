//! Schema migration v60 — `retro` table for Stream
//! `mission-retro-review-ui` (MISSION_BACKLOG Section 6 #209).
//!
//! Per-worker retrospective record over a time window. Stores
//! aggregated metrics (frozen at creation from `audit_record`) plus a
//! 5-section operator-fillable template. `finalized_at` is non-null
//! once the operator locks the retro; the gateway PATCH handler
//! refuses mutations against finalized rows.
//!
//! Slot rationale: v59 (`elevated_view_grant`) was the prior tail, so
//! v60 is the next free versioned slot. The raw SQL filename slot is
//! 72 — slot 71 is pinned to `mission-approval-bundle-ui` (still
//! CLAIMED on origin/main when this brief landed); filename and
//! version counters advance independently per the v55 convention.
//!
//! UP creates the table + two indexes (worker-keyed + tenant-keyed).
//! Idempotent via `IF NOT EXISTS`. DOWN drops the indexes in reverse
//! order, then the table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V60_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v60 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V60_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v60 down: {e}")))?;
    }
    Ok(())
}

const V60_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS retro (
        id              BLOB PRIMARY KEY,
        tenant_id       BLOB NOT NULL,
        worker_id       BLOB NOT NULL,
        window_start    INTEGER NOT NULL,
        window_end      INTEGER NOT NULL,
        metrics_json    TEXT NOT NULL,
        sections_json   TEXT NOT NULL,
        created_at      INTEGER NOT NULL,
        finalized_at    INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_retro_worker \
     ON retro(worker_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_retro_tenant \
     ON retro(tenant_id, created_at)",
];

const V60_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_retro_tenant",
    "DROP INDEX IF EXISTS idx_retro_worker",
    "DROP TABLE IF EXISTS retro",
];

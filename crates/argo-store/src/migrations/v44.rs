//! Schema migration v44 — first-class approvals table.
//!
//! Reconciles raw SQL `48__approvals.sql` (Stream
//! `claude-approval-request-store`). Single table `approval_request`
//! tracks pending approval requests for the auto-approval gate, the
//! deadline + default decision a future timeout sweep
//! (`claude-approval-timeout-default-decision`) will promote stale
//! rows by, and the resolved decision once granted/denied/expired.
//!
//! Slot rationale: v43 (`skill_canary`) is the highest versioned slot
//! on origin/main; v44 is the next free slot. Raw SQL slot 47 is
//! taken by `skill_canary`, so this file lands at 48.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops the
//! table and both indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V44_UP {
        exec(tx, "v44 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V44_DOWN {
        exec(tx, "v44 down", sql).await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

const V44_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS approval_request (
        id                 BLOB PRIMARY KEY,
        action_id          BLOB NOT NULL,
        tenant_id          BLOB NOT NULL,
        requested_at       INTEGER NOT NULL,
        requested_by       TEXT NOT NULL,
        deadline_at        INTEGER NOT NULL,
        default_decision   TEXT NOT NULL,
        status             TEXT NOT NULL DEFAULT 'pending',
        decided_at         INTEGER,
        decided_by         TEXT,
        decision           TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_approval_request_tenant_status
     ON approval_request(tenant_id, status)",
    "CREATE INDEX IF NOT EXISTS idx_approval_request_deadline
     ON approval_request(deadline_at)",
];

const V44_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_approval_request_deadline",
    "DROP INDEX IF EXISTS idx_approval_request_tenant_status",
    "DROP TABLE IF EXISTS approval_request",
];

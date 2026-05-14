//! Schema migration v68 — `approval_bundle` + `approval_bundle_member`
//! tables for Stream `mission-approval-bundle-ui`. Server-defined
//! approval bundles let a worker group N coherent low-risk actions
//! into one operator decision.
//!
//! Slot rationale: v60-v67 landed on main while this PR was DIRTY
//! (`retro`, `worker_role_name`, `operator_risk_overrides`,
//! `elevated_view_grant`, `audit_record_tenant_id`, `vertical_locks`,
//! `gateway_request_audit`, `audit_record_filter_columns`), so v68
//! is the next free versioned slot.
//! The raw SQL filename slot is 71 (reserved for this brief; main's
//! adjacent migrations took 72-76); filename and version counters
//! advance independently per the v55 convention.
//!
//! UP creates both tables + two indexes. Idempotent via `IF NOT
//! EXISTS`. DOWN drops the indexes in reverse order, then the member
//! table, then the bundle table (FK order).
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V68_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v68 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V68_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v68 down: {e}")))?;
    }
    Ok(())
}

const V68_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS approval_bundle (
        id          BLOB PRIMARY KEY,
        tenant_id   BLOB NOT NULL,
        label       TEXT NOT NULL,
        created_at  INTEGER NOT NULL,
        status      TEXT NOT NULL DEFAULT 'pending',
        decided_at  INTEGER,
        decided_by  TEXT,
        decision    TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_approval_bundle_tenant_status \
     ON approval_bundle(tenant_id, status)",
    "CREATE TABLE IF NOT EXISTS approval_bundle_member (
        bundle_id    BLOB NOT NULL,
        approval_id  TEXT NOT NULL,
        member_idx   INTEGER NOT NULL,
        PRIMARY KEY (bundle_id, approval_id),
        FOREIGN KEY (bundle_id) REFERENCES approval_bundle(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_approval_bundle_member_bundle \
     ON approval_bundle_member(bundle_id, member_idx)",
];

const V68_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_approval_bundle_member_bundle",
    "DROP TABLE IF EXISTS approval_bundle_member",
    "DROP INDEX IF EXISTS idx_approval_bundle_tenant_status",
    "DROP TABLE IF EXISTS approval_bundle",
];

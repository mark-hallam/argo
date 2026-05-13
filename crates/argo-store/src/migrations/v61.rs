//! Schema migration v61 — `audit_record.tenant_id` denorm column for
//! Stream `mission-enterprise-cross-tenant-audit`.
//!
//! Threads workspace-scoped `tenant_id` through the audit chain so
//! the `GET /api/v1/audit` feed can scope rows per tenant. The
//! `__legacy__` sentinel default grandfathers every historical row
//! into one bucket — the default workspace tenant continues to read
//! that bucket (see `routes_audit_feed.rs`) so existing audit
//! visibility is preserved without an env-flag flip.
//!
//! UP adds the column with a `__legacy__` default and indexes
//! `(tenant_id, created_at)` for the feed's WHERE-clause pattern.
//! The chain hash inputs (`parent_hash, kind, canonical(payload)`)
//! are unchanged so every historical `self_hash` keeps verifying.
//!
//! DOWN drops the index then the column. Note: SQLite < 3.35 does
//! not support `ALTER TABLE ... DROP COLUMN`; this DOWN assumes a
//! modern SQLite (matches the rest of the migrations).
//!
//! Slot rationale: v60 (`retro`) landed on main while this PR was
//! stale, so v61 is the next free versioned slot. Raw SQL filename
//! slot is 65 — slot 64 is reserved for the sibling recovery PR
//! #1447 (`mission-enterprise-audit-full-text-search`); filename +
//! version counters advance independently per the v55 module's
//! documented convention.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V61_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v61 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V61_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v61 down: {e}")))?;
    }
    Ok(())
}

const V61_UP: &[&str] = &[
    "ALTER TABLE audit_record ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__legacy__'",
    "CREATE INDEX IF NOT EXISTS idx_audit_record_tenant_created \
     ON audit_record(tenant_id, created_at)",
];

const V61_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_audit_record_tenant_created",
    "ALTER TABLE audit_record DROP COLUMN tenant_id",
];

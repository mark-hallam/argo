//! Schema migration v59 — `elevated_view_grant` table for Stream J16
//! (`gateway-elevated-view-request-endpoint`).
//!
//! Per-principal, per-category, time-bounded "show me the raw value"
//! override that the J2 redact-policy resolver unions with the
//! viewer's RBAC permissions. v1 auto-grants on request; the
//! `mission-enterprise-elevated-view-approval` follow-up inserts an
//! operator approval step.
//!
//! Slot rationale: v58 (`operator_risk_overrides`) was the prior tail,
//! so v59 is the next free versioned slot. The raw SQL filename slot
//! is 63 (after 62 = `operator_risk_overrides`); filename and version
//! counters advance independently, per the v55 convention.
//!
//! SQLite has no native `timestamptz` / `text[]`, so the brief's
//! `timestamptz` columns are encoded as `INTEGER` (unix seconds) and
//! `categories` is encoded as a JSON-array `TEXT` column.
//!
//! UP creates the table + two indexes. Idempotent via `IF NOT EXISTS`.
//! DOWN drops the indexes in reverse order, then the table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V59_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v59 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V59_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v59 down: {e}")))?;
    }
    Ok(())
}

const V59_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS elevated_view_grant (
        id              BLOB PRIMARY KEY,
        tenant_id       BLOB NOT NULL,
        principal_id    TEXT NOT NULL,
        categories_json TEXT NOT NULL,
        reason          TEXT NOT NULL,
        granted_at      INTEGER NOT NULL,
        expires_at      INTEGER NOT NULL,
        revoked_at      INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_elevated_view_grant_principal \
     ON elevated_view_grant(principal_id, revoked_at)",
    "CREATE INDEX IF NOT EXISTS idx_elevated_view_grant_tenant \
     ON elevated_view_grant(tenant_id)",
];

const V59_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_elevated_view_grant_tenant",
    "DROP INDEX IF EXISTS idx_elevated_view_grant_principal",
    "DROP TABLE IF EXISTS elevated_view_grant",
];

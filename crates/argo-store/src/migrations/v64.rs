//! Schema migration v64 -- `scim_user` + `scim_provisioned_tombstones`
//! tables for Stream `mission-scim-provisioning` (MISSION_BACKLOG
//! Section 7 #231).
//!
//! Backs the SCIM 2.0 user-provisioning endpoints mounted by
//! `argo-cli`'s gateway under `/scim/v2/Users`. The two tables split
//! the live-row store (`scim_user`) from the post-DELETE tombstone
//! index (`scim_provisioned_tombstones`) so the audit chain can still
//! resolve a tenant + `userName` for a soft-deleted SCIM user without
//! scanning every tombstoned row in `scim_user`.
//!
//! Slot rationale: v63 landed on main while this PR was stale, so SCIM
//! moves to v64. The raw SQL filename slot is 75: slot 74 is already
//! owned by the roster policy override migration. Filename and version
//! counters advance independently per the v55 convention.
//!
//! UP creates both tables + supporting indexes. Idempotent via
//! `IF NOT EXISTS`. DOWN drops indexes and tables in reverse order.

use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V64_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v64 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V64_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v64 down: {e}")))?;
    }
    Ok(())
}

const V64_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS scim_user (
        id            BLOB    PRIMARY KEY,
        tenant_id     BLOB    NOT NULL,
        external_id   TEXT,
        user_name     TEXT    NOT NULL,
        display_name  TEXT,
        emails_json   TEXT    NOT NULL DEFAULT '[]',
        active        INTEGER NOT NULL DEFAULT 1,
        created_at    INTEGER NOT NULL,
        updated_at    INTEGER NOT NULL,
        deleted_at    INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_scim_user_tenant_username \
     ON scim_user(tenant_id, user_name)",
    "CREATE INDEX IF NOT EXISTS idx_scim_user_external \
     ON scim_user(tenant_id, external_id)",
    "CREATE TABLE IF NOT EXISTS scim_provisioned_tombstones (
        id          BLOB    PRIMARY KEY,
        tenant_id   BLOB    NOT NULL,
        user_name   TEXT    NOT NULL,
        deleted_at  INTEGER NOT NULL,
        reason      TEXT    NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_scim_tombstones_tenant \
     ON scim_provisioned_tombstones(tenant_id)",
];

const V64_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_scim_tombstones_tenant",
    "DROP TABLE IF EXISTS scim_provisioned_tombstones",
    "DROP INDEX IF EXISTS idx_scim_user_external",
    "DROP INDEX IF EXISTS idx_scim_user_tenant_username",
    "DROP TABLE IF EXISTS scim_user",
];

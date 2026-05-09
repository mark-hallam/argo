//! Schema migration v38 — per-tenant resource quotas.
//!
//! Reconciles raw SQL migration `28__tenant_quota.sql`. Single
//! table `tenant_quota` carrying daily/monthly caps on USD micros,
//! action count, and worker count per tenant.
//!
//! 28-collision split: this slot pairs with v36 (recovery_cache)
//! and v37 (role_duties). See v37 doc comment for ordering
//! rationale.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and its index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V38_UP {
        exec(tx, "v38 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V38_DOWN {
        exec(tx, "v38 down", sql).await?;
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

const V38_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tenant_quota (
        tenant_id TEXT PRIMARY KEY,
        period TEXT NOT NULL CHECK (period IN ('daily', 'monthly')),
        max_usd_micros INTEGER,
        max_actions INTEGER,
        max_workers INTEGER,
        set_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_tenant_quota_period
     ON tenant_quota(period, set_at)",
];

const V38_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_tenant_quota_period",
    "DROP TABLE IF EXISTS tenant_quota",
];

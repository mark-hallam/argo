//! Schema migration v14 — worker budget ledger.
//!
//! Structurally idempotent version of raw SQL migration
//! `14__worker_budget.sql`.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V14_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v14 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Err(ArgoError::Provider(
        "v14 rollback unsupported: raw migration reconciliation is additive and may own live data"
            .into(),
    ))
}

const V14_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_budget (
        tenant_id BLOB NOT NULL,
        worker_id TEXT NOT NULL,
        action_id TEXT NOT NULL,
        usd_micros INTEGER NOT NULL,
        charged_at INTEGER NOT NULL,
        PRIMARY KEY (tenant_id, worker_id, action_id, charged_at)
    )",
    "CREATE INDEX IF NOT EXISTS idx_worker_budget_day
     ON worker_budget(tenant_id, worker_id, charged_at)",
];

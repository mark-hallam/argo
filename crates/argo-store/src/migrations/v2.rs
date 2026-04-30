//! Schema migration v2 — `spend_ledger` for item C (hard $$$ cost
//! ceilings). Stores per-`(tenant, session)` accumulators that
//! [`argo_types::enforce_pre_spend`] reads before authorising a
//! provider call.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V2_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v2 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V2_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v2 down: {e}")))?;
    }
    Ok(())
}

const V2_UP: &[&str] = &[
    // Per-charge ledger row. The unique key (tenant, session,
    // charged_at_micros, seq) lets us replay history without
    // collapsing entries; the indexes power day/month aggregation.
    "CREATE TABLE spend_ledger (
        seq                INTEGER PRIMARY KEY AUTOINCREMENT,
        tenant_id          BLOB NOT NULL,
        session_id         BLOB NOT NULL,
        charged_at_micros  INTEGER NOT NULL,
        amount_usd_micros  INTEGER NOT NULL,
        provider           TEXT NOT NULL,
        model              TEXT NOT NULL
    )",
    "CREATE INDEX idx_spend_tenant_session
        ON spend_ledger(tenant_id, session_id, charged_at_micros)",
    "CREATE INDEX idx_spend_tenant_day
        ON spend_ledger(tenant_id, charged_at_micros)",
];

const V2_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_spend_tenant_day",
    "DROP INDEX IF EXISTS idx_spend_tenant_session",
    "DROP TABLE IF EXISTS spend_ledger",
];

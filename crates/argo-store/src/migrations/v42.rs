//! Schema migration v42 — additive `audit_record.principal_id`
//! column for the Tier 1 MVP `mvp-audit-principal-stamp` stream.
//!
//! v41 was the most recent versioned slot; SQL files 42..45 are
//! managed by their own runtime helpers (worker_presence, …) and
//! never advance `schema_version`. This is the next free
//! versioned slot. Reconciles raw SQL `46__audit_principal.sql`
//! (file picks the next-free filename slot, separate counter from
//! `schema_version`). UP adds the column + an index for
//! principal-scoped audit lookups. DOWN drops them.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V42_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v42 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V42_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v42 down: {e}")))?;
    }
    Ok(())
}

const V42_UP: &[&str] = &[
    "ALTER TABLE audit_record ADD COLUMN principal_id TEXT",
    "CREATE INDEX IF NOT EXISTS idx_audit_principal \
     ON audit_record(principal_id)",
];

const V42_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_audit_principal",
    "ALTER TABLE audit_record DROP COLUMN principal_id",
];

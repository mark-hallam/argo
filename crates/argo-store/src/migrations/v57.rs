//! Schema migration v57 — additive `worker.role_name` (TEXT,
//! nullable) column for Stream `mission-routing-skillmatch-rolematch`
//! (Tier M4). Adds the persistence primitive for
//! `TargetSelector::RoleMatch(String)` resolution.
//!
//! `role_name` is NULL by default — operators populate it via the
//! `argo worker onboard --role-name <s>` CLI flag. The routing engine
//! filters by exact match and ranks the resulting workers by
//! `trust_grade DESC, last_used_at DESC, worker_id ASC`. Empty result
//! is normal in v1 because most existing workers won't have it set.
//!
//! v56 (`tenant_rate_limit`, brief `mission-per-tenant-rate-limiting`)
//! was the prior versioned slot when this brief landed, so v57 is the
//! next free slot. Reconciles raw SQL `61__worker_role_name.sql`
//! (filename slot 61 picks the next free SQL filename — 60 is taken
//! by `tenant_rate_limit`).
//!
//! UP adds the column + supporting index. DOWN drops both in reverse
//! order.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V57_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v57 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V57_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v57 down: {e}")))?;
    }
    Ok(())
}

const V57_UP: &[&str] = &[
    "ALTER TABLE worker ADD COLUMN role_name TEXT",
    "CREATE INDEX IF NOT EXISTS idx_worker_role_name \
     ON worker(role_name)",
];

const V57_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_role_name",
    "ALTER TABLE worker DROP COLUMN role_name",
];

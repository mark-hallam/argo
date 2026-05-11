//! Schema migration v56 — `tenant_rate_limit` table for Stream
//! `mission-per-tenant-rate-limiting` (MISSION_BACKLOG Section 1
//! #21).
//!
//! Per-tenant override of the gateway's per-tenant rate limit. The
//! gateway token-buckets every tenant at a config-driven default
//! (`[gateway.rate_limit.tenant].requests_per_minute`); an operator
//! can replace that default for one tenant by writing a row here.
//! Missing row → fall back to the config default.
//!
//! Slot rationale: v55 (`skill_flag`, brief
//! `mission-forge-operator-feedback-eval-signal`) was the prior tail,
//! so v56 is the next free versioned slot. The raw SQL filename slot
//! is 60 (after 59 = `skill_flag`) — filename + version counters
//! advance independently, per the v55 module's documented convention.
//!
//! UP creates the table with a `BLOB PRIMARY KEY` on `tenant_id`
//! (matches every other tenant-scoped table in the workspace store).
//! Idempotent via `IF NOT EXISTS`. DOWN drops it.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V56_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v56 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V56_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v56 down: {e}")))?;
    }
    Ok(())
}

const V56_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tenant_rate_limit (
        tenant_id            BLOB    PRIMARY KEY,
        requests_per_minute  INTEGER NOT NULL,
        updated_at           INTEGER NOT NULL
    )",
];

const V56_DOWN: &[&str] = &["DROP TABLE IF EXISTS tenant_rate_limit"];

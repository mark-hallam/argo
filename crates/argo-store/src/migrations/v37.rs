//! Schema migration v37 — worker duty scheduler tables.
//!
//! Reconciles raw SQL migration `28__role_duties.sql` (Stream
//! `claude-worker-duty-scheduler`). Two tables: `role_duty`
//! (firable duties attached to a worker's role) and
//! `duty_fired_session` (link from a fired duty to the spawned
//! session for audit attribution).
//!
//! 28-collision split: this slot pairs with v36 (recovery_cache)
//! above and v38 (tenant_quota) below. The brief assigns
//! recovery_cache → v36, role_duties → v37, tenant_quota → v38,
//! consistent with the published-contract ordering used for prior
//! collision groups (see v31 doc comment).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! both tables and all indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V37_UP {
        exec(tx, "v37 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V37_DOWN {
        exec(tx, "v37 down", sql).await?;
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

const V37_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS role_duty (
        id TEXT PRIMARY KEY,
        tenant_id BLOB NOT NULL,
        worker_id TEXT NOT NULL,
        cron TEXT NOT NULL,
        entry_action TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        enabled INTEGER NOT NULL DEFAULT 1,
        paused_until INTEGER,
        last_fired_at INTEGER,
        created_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_role_duty_worker
     ON role_duty(tenant_id, worker_id, enabled)",
    "CREATE INDEX IF NOT EXISTS idx_role_duty_active
     ON role_duty(enabled, paused_until)",
    "CREATE TABLE IF NOT EXISTS duty_fired_session (
        duty_id TEXT NOT NULL,
        session_id TEXT NOT NULL,
        fired_at INTEGER NOT NULL,
        decision TEXT NOT NULL DEFAULT 'auto_approved',
        PRIMARY KEY (duty_id, session_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_duty_fired_session_duty
     ON duty_fired_session(duty_id, fired_at DESC)",
];

const V37_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_duty_fired_session_duty",
    "DROP TABLE IF EXISTS duty_fired_session",
    "DROP INDEX IF EXISTS idx_role_duty_active",
    "DROP INDEX IF EXISTS idx_role_duty_worker",
    "DROP TABLE IF EXISTS role_duty",
];

//! Schema migration v12 — worker identity tables.
//!
//! This reconciles raw SQL migration `12__worker_identity.sql` into
//! the versioned runner. The `up` path is structural: it creates
//! missing tables and probes `role` columns before additive ALTERs,
//! so databases that already ran the raw SQL continue cleanly.
//!
//! Rollback is intentionally unsupported for this reconciliation
//! range. These tables may already have been created by lazy raw-SQL
//! callers outside the versioned runner, so dropping them here would
//! risk deleting live worker identity data.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V12_TABLES_AND_INDEXES {
        exec(tx, "v12 up", sql).await?;
    }
    for (column, ddl) in ROLE_COLUMNS {
        if !column_exists(tx, "role", column).await? {
            exec(tx, "v12 up", ddl).await?;
        }
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Err(unsupported_down(12))
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

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v12 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}

fn unsupported_down(version: u32) -> ArgoError {
    ArgoError::Provider(format!(
        "v{version} rollback unsupported: raw migration reconciliation is additive and may own live data"
    ))
}

const V12_TABLES_AND_INDEXES: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS persona (
        id TEXT PRIMARY KEY,
        worker_id TEXT NOT NULL UNIQUE,
        tenant_id TEXT NOT NULL,
        name TEXT NOT NULL,
        voice_preamble TEXT NOT NULL,
        avatar TEXT,
        bio TEXT,
        builder_attribution TEXT NOT NULL,
        provenance_chain_json TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS worker (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        vertical_id TEXT NOT NULL,
        persona_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        memory_scope TEXT NOT NULL,
        auto_approval_json TEXT NOT NULL,
        channels_json TEXT NOT NULL,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS duty (
        id TEXT PRIMARY KEY,
        worker_id TEXT NOT NULL,
        role_id TEXT NOT NULL,
        tenant_id TEXT NOT NULL,
        cron TEXT NOT NULL,
        entry_action TEXT NOT NULL,
        description TEXT NOT NULL,
        created_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_worker_tenant ON worker(tenant_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_duty_worker ON duty(worker_id)",
];

const ROLE_COLUMNS: &[(&str, &str)] = &[
    ("worker_id", "ALTER TABLE role ADD COLUMN worker_id TEXT"),
    ("charter", "ALTER TABLE role ADD COLUMN charter TEXT"),
    ("kpis_json", "ALTER TABLE role ADD COLUMN kpis_json TEXT"),
    ("escalation", "ALTER TABLE role ADD COLUMN escalation TEXT"),
];

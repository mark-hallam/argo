//! Schema migration v22 — org chart + addressable names.
//!
//! Reconciles raw SQL migration `21__org_chart.sql` (Stream
//! `claude-org-awareness`). Four tables — `org_unit`, `org_member`,
//! `org_member_alias`, `org_reports_to` — form the org graph used
//! by `argo org member <alias>` and the reports-to walker.
//!
//! Idempotent: every CREATE uses `IF NOT EXISTS`, so databases
//! that already ran the lazy raw SQL (via `argo-identity` ensure
//! paths) re-enter cleanly. Rollback drops all four tables and
//! their indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V22_UP {
        exec(tx, "v22 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V22_DOWN {
        exec(tx, "v22 down", sql).await?;
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

const V22_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS org_unit (
        id          TEXT PRIMARY KEY,
        tenant_id   TEXT NOT NULL,
        name        TEXT NOT NULL,
        parent_id   TEXT,
        created_at  INTEGER NOT NULL,
        UNIQUE(tenant_id, name)
    )",
    "CREATE INDEX IF NOT EXISTS idx_org_unit_parent ON org_unit(parent_id)",
    "CREATE INDEX IF NOT EXISTS idx_org_unit_tenant ON org_unit(tenant_id)",
    "CREATE TABLE IF NOT EXISTS org_member (
        id          TEXT PRIMARY KEY,
        tenant_id   TEXT NOT NULL,
        kind        TEXT NOT NULL,
        canonical   TEXT NOT NULL,
        unit_id     TEXT,
        manager_id  TEXT,
        worker_id   TEXT,
        created_at  INTEGER NOT NULL,
        UNIQUE(tenant_id, canonical)
    )",
    "CREATE INDEX IF NOT EXISTS idx_org_member_unit ON org_member(unit_id)",
    "CREATE INDEX IF NOT EXISTS idx_org_member_manager ON org_member(manager_id)",
    "CREATE INDEX IF NOT EXISTS idx_org_member_worker ON org_member(worker_id)",
    "CREATE TABLE IF NOT EXISTS org_member_alias (
        member_id   TEXT NOT NULL,
        alias       TEXT NOT NULL,
        PRIMARY KEY (member_id, alias)
    )",
    "CREATE INDEX IF NOT EXISTS idx_org_member_alias_alias ON org_member_alias(alias)",
    "CREATE TABLE IF NOT EXISTS org_reports_to (
        member_id   TEXT NOT NULL,
        manager_id  TEXT NOT NULL,
        depth       INTEGER NOT NULL,
        PRIMARY KEY (member_id, manager_id)
    )",
];

const V22_DOWN: &[&str] = &[
    "DROP TABLE IF EXISTS org_reports_to",
    "DROP INDEX IF EXISTS idx_org_member_alias_alias",
    "DROP TABLE IF EXISTS org_member_alias",
    "DROP INDEX IF EXISTS idx_org_member_worker",
    "DROP INDEX IF EXISTS idx_org_member_manager",
    "DROP INDEX IF EXISTS idx_org_member_unit",
    "DROP TABLE IF EXISTS org_member",
    "DROP INDEX IF EXISTS idx_org_unit_tenant",
    "DROP INDEX IF EXISTS idx_org_unit_parent",
    "DROP TABLE IF EXISTS org_unit",
];

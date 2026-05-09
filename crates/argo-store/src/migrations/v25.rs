//! Schema migration v25 — declarative handoff routing rules.
//!
//! Reconciles raw SQL migration `23__handoff_routing.sql` (Stream
//! `claude-handoff-routing-rules`). Single table
//! `handoff_routing_rule` carrying tenant-scoped condition/target
//! JSON blobs; consulted by HandoffWalker before falling through
//! to the static escalation chain.
//!
//! 23-collision split: this slot pairs with v26 below. The brief's
//! recommended assignment maps handoff_routing → v25 and
//! workflow_checkpoints → v26. Git-log shows handoff_routing
//! (PR #883, 08:09) was authored before workflow_checkpoints
//! (PR #887, 08:40), matching the brief's order.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and its index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V25_UP {
        exec(tx, "v25 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V25_DOWN {
        exec(tx, "v25 down", sql).await?;
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

const V25_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS handoff_routing_rule (
        id              TEXT PRIMARY KEY,
        tenant_id       TEXT NOT NULL,
        name            TEXT NOT NULL,
        condition_json  TEXT NOT NULL,
        target_json     TEXT NOT NULL,
        priority        INTEGER NOT NULL,
        active          INTEGER NOT NULL,
        created_at      INTEGER NOT NULL,
        UNIQUE(tenant_id, name)
    )",
    "CREATE INDEX IF NOT EXISTS idx_handoff_routing_tenant
     ON handoff_routing_rule(tenant_id)",
];

const V25_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_handoff_routing_tenant",
    "DROP TABLE IF EXISTS handoff_routing_rule",
];

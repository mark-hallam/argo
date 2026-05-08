//! Schema migration v16 — orchestrator session columns + `handoff`
//! table.
//!
//! Reconciles raw SQL migration `16__orchestrator.sql` into the
//! versioned runner. Idempotent so databases that already ran the
//! lazy raw SQL continue cleanly: each `session` column is probed
//! with `PRAGMA table_info` before issuing `ALTER TABLE ... ADD
//! COLUMN`, and the `handoff` table uses `CREATE TABLE IF NOT
//! EXISTS`.
//!
//! Rollback is unsupported: the four `session` columns may carry
//! orchestrator state on live databases, and there is no safe way
//! to drop them once application code starts depending on the
//! reconciled v17..v21 schema below them.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for (column, ddl) in SESSION_COLUMNS {
        if !column_exists(tx, "session", column).await? {
            exec(tx, "v16 up", ddl).await?;
        }
    }
    for sql in V16_TABLES_AND_INDEXES {
        exec(tx, "v16 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Err(ArgoError::Provider(
        "v16 rollback unsupported: orchestrator session columns may own live data"
            .into(),
    ))
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
        .map_err(|e| ArgoError::Provider(format!("v16 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}

const SESSION_COLUMNS: &[(&str, &str)] = &[
    (
        "status",
        "ALTER TABLE session ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
    ),
    (
        "participants",
        "ALTER TABLE session ADD COLUMN participants TEXT NOT NULL DEFAULT '[]'",
    ),
    (
        "created_at",
        "ALTER TABLE session ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "deadline",
        "ALTER TABLE session ADD COLUMN deadline INTEGER",
    ),
];

const V16_TABLES_AND_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_orchestrator_session_active
     ON session(tenant_id, status, created_at)",
    "CREATE TABLE IF NOT EXISTS handoff (
        id TEXT PRIMARY KEY,
        session_id BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        from_worker_id TEXT NOT NULL,
        \"to\" TEXT NOT NULL,
        expected_outcome TEXT NOT NULL,
        payload TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        deadline INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_handoff_session
     ON handoff(session_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_handoff_status
     ON handoff(status, created_at)",
];

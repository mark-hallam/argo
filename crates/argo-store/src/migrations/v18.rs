//! Schema migration v18 — `workflow_run` history table.
//!
//! Reconciles raw SQL migration `18__workflow_runs.sql`. Each
//! non-dry-run `argo workflow run` writes one row at start and
//! updates it at end; the CLI surfaces these via
//! `argo workflow status <run-id>` and `argo workflow list-runs`.
//!
//! Rollback drops the table and its index. No other reconciled
//! migration in v16..v21 depends on it.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V18_UP {
        exec(tx, "v18 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V18_DOWN {
        exec(tx, "v18 down", sql).await?;
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

const V18_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS workflow_run (
        id TEXT PRIMARY KEY,
        workflow_id TEXT NOT NULL,
        workflow_yaml TEXT NOT NULL,
        started_at INTEGER NOT NULL,
        finished_at INTEGER,
        ok INTEGER,
        task_verdicts_json TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_workflow_run_started_at
     ON workflow_run(started_at DESC)",
];

const V18_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_workflow_run_started_at",
    "DROP TABLE IF EXISTS workflow_run",
];

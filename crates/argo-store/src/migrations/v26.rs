//! Schema migration v26 — workflow checkpoint + resume.
//!
//! Reconciles raw SQL migration `23__workflow_checkpoints.sql`
//! (Stream `claude-checkpoint-resume`). Single table
//! `workflow_checkpoint` capturing per-task outputs for
//! `argo workflow resume` and the `cache_key` whole-workflow
//! idempotency path.
//!
//! 23-collision split: this slot pairs with v25 above (see v25
//! doc comment).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and both of its indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V26_UP {
        exec(tx, "v26 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V26_DOWN {
        exec(tx, "v26 down", sql).await?;
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

const V26_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS workflow_checkpoint (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        workflow_run_id TEXT NOT NULL,
        task_id TEXT NOT NULL,
        completed_at INTEGER NOT NULL,
        outputs_json TEXT NOT NULL,
        cache_key TEXT,
        seq INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_run
     ON workflow_checkpoint(workflow_run_id, seq)",
    "CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_cache_key
     ON workflow_checkpoint(cache_key)
     WHERE cache_key IS NOT NULL",
];

const V26_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_workflow_checkpoint_cache_key",
    "DROP INDEX IF EXISTS idx_workflow_checkpoint_run",
    "DROP TABLE IF EXISTS workflow_checkpoint",
];

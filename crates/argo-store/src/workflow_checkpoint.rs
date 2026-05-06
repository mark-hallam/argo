//! Sqlx CRUD helpers for `workflow_checkpoint` (Stream
//! `claude-checkpoint-resume`). This module is intentionally
//! a thin typed wrapper over the SQL — the higher-level
//! `CheckpointStore` trait (in `argo-workflow`) is bound to
//! these helpers in `argo-cli/handlers/workflow_checkpoint.rs`.

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

/// One row in the `workflow_checkpoint` table. Mirrors the
/// argo-workflow `Checkpoint` struct but kept independent so
/// argo-store doesn't depend on argo-workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRow {
    /// Per-run sequence number.
    pub seq: i64,
    /// Workflow run id this row belongs to.
    pub workflow_run_id: String,
    /// Task id within the workflow.
    pub task_id: String,
    /// Microseconds since epoch.
    pub completed_at_micros: i64,
    /// JSON-encoded outputs (BTreeMap<String, serde_json::Value>).
    pub outputs_json: String,
    /// Optional cache key for cross-run idempotency.
    pub cache_key: Option<String>,
}

/// Errors surfaced by the workflow_checkpoint helpers.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointSqlError {
    /// Underlying sqlx error.
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Append a checkpoint row. Returns the auto-assigned `seq`.
pub async fn insert_checkpoint(
    pool: &Pool<Sqlite>,
    workflow_run_id: &str,
    task_id: &str,
    outputs_json: &str,
    cache_key: Option<&str>,
) -> Result<i64, CheckpointSqlError> {
    let next_seq: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM workflow_checkpoint \
         WHERE workflow_run_id = ?",
    )
    .bind(workflow_run_id)
    .fetch_one(pool)
    .await?;
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO workflow_checkpoint \
         (workflow_run_id, task_id, completed_at, outputs_json, cache_key, seq) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(workflow_run_id)
    .bind(task_id)
    .bind(now)
    .bind(outputs_json)
    .bind(cache_key)
    .bind(next_seq)
    .execute(pool)
    .await?;
    Ok(next_seq)
}

/// All checkpoints for one workflow run, ordered by `seq` ascending.
pub async fn list_for_run(
    pool: &Pool<Sqlite>,
    workflow_run_id: &str,
) -> Result<Vec<CheckpointRow>, CheckpointSqlError> {
    let rows: Vec<(i64, String, String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT seq, workflow_run_id, task_id, completed_at, outputs_json, cache_key \
         FROM workflow_checkpoint WHERE workflow_run_id = ? ORDER BY seq ASC",
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(seq, run, task, ts, outs, key)| CheckpointRow {
            seq,
            workflow_run_id: run,
            task_id: task,
            completed_at_micros: ts,
            outputs_json: outs,
            cache_key: key,
        })
        .collect())
}

/// Look up the most-recent checkpoint with a matching cache_key.
pub async fn lookup_cache_key(
    pool: &Pool<Sqlite>,
    cache_key: &str,
) -> Result<Option<CheckpointRow>, CheckpointSqlError> {
    let row: Option<(i64, String, String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT seq, workflow_run_id, task_id, completed_at, outputs_json, cache_key \
         FROM workflow_checkpoint WHERE cache_key = ? \
         ORDER BY completed_at DESC LIMIT 1",
    )
    .bind(cache_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(seq, run, task, ts, outs, key)| CheckpointRow {
        seq,
        workflow_run_id: run,
        task_id: task,
        completed_at_micros: ts,
        outputs_json: outs,
        cache_key: key,
    }))
}

/// Drop every checkpoint row strictly above `target_seq` for one run.
/// Returns `false` if `target_seq` doesn't match any existing row.
pub async fn rollback_to(
    pool: &Pool<Sqlite>,
    workflow_run_id: &str,
    target_seq: i64,
) -> Result<bool, CheckpointSqlError> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM workflow_checkpoint WHERE workflow_run_id = ? AND seq = ?",
    )
    .bind(workflow_run_id)
    .bind(target_seq)
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        return Ok(false);
    }
    sqlx::query(
        "DELETE FROM workflow_checkpoint WHERE workflow_run_id = ? AND seq > ?",
    )
    .bind(workflow_run_id)
    .bind(target_seq)
    .execute(pool)
    .await?;
    Ok(true)
}

/// Pretty-print a microseconds timestamp as RFC-3339.
#[must_use]
pub fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
}

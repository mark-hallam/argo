//! Sqlite-backed [`CheckpointStore`] (Stream
//! `claude-checkpoint-resume`). Bridges the argo-workflow trait
//! to the sqlx helpers in `argo-store::workflow_checkpoint`.

use argo_store::workflow_checkpoint as repo;
use argo_store::SessionStore;
use argo_workflow::{Checkpoint, CheckpointError, CheckpointStore, WorkflowId};
use async_trait::async_trait;
use std::sync::Arc;

const SCHEMA_SQL: &str =
    include_str!("../../../argo-store/migrations/23__workflow_checkpoints.sql");

/// Sqlite-backed checkpoint store.
pub struct SqliteCheckpointStore {
    store: Arc<SessionStore>,
}

impl SqliteCheckpointStore {
    /// Wrap an open `SessionStore`.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }
}

/// Apply the workflow_checkpoint schema (idempotent).
pub async fn ensure_schema(store: &SessionStore) -> Result<(), CheckpointError> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| CheckpointError::Backend(format!("schema: {e}")))?;
    }
    Ok(())
}

#[async_trait]
impl CheckpointStore for SqliteCheckpointStore {
    async fn write(&self, ckpt: Checkpoint) -> Result<i64, CheckpointError> {
        let outputs_json = serde_json::to_string(&ckpt.outputs)
            .map_err(|e| CheckpointError::Backend(format!("outputs json: {e}")))?;
        repo::insert_checkpoint(
            self.store.pool(),
            ckpt.workflow_run_id.as_str(),
            &ckpt.task_id,
            &outputs_json,
            ckpt.cache_key.as_deref(),
        )
        .await
        .map_err(|e| CheckpointError::Backend(format!("insert: {e}")))
    }

    async fn list_for_run(
        &self,
        workflow_run_id: &WorkflowId,
    ) -> Result<Vec<Checkpoint>, CheckpointError> {
        let rows = repo::list_for_run(self.store.pool(), workflow_run_id.as_str())
            .await
            .map_err(|e| CheckpointError::Backend(format!("list_for_run: {e}")))?;
        rows.into_iter().map(into_checkpoint).collect()
    }

    async fn lookup_cache_key(
        &self,
        cache_key: &str,
    ) -> Result<Option<Checkpoint>, CheckpointError> {
        let row = repo::lookup_cache_key(self.store.pool(), cache_key)
            .await
            .map_err(|e| CheckpointError::Backend(format!("lookup_cache_key: {e}")))?;
        row.map(into_checkpoint).transpose()
    }

    async fn rollback_to(
        &self,
        workflow_run_id: &WorkflowId,
        target_seq: i64,
    ) -> Result<(), CheckpointError> {
        let ok = repo::rollback_to(self.store.pool(), workflow_run_id.as_str(), target_seq)
            .await
            .map_err(|e| CheckpointError::Backend(format!("rollback_to: {e}")))?;
        if !ok {
            return Err(CheckpointError::SeqNotFound {
                run: workflow_run_id.as_str().to_string(),
                seq: target_seq,
            });
        }
        Ok(())
    }
}

fn into_checkpoint(row: repo::CheckpointRow) -> Result<Checkpoint, CheckpointError> {
    let outputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| CheckpointError::Backend(format!("outputs parse: {e}")))?;
    Ok(Checkpoint {
        seq: row.seq,
        workflow_run_id: WorkflowId(row.workflow_run_id),
        task_id: row.task_id,
        completed_at_micros: row.completed_at_micros,
        outputs,
        cache_key: row.cache_key,
    })
}

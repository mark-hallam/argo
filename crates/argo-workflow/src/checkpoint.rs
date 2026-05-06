//! Workflow checkpoint + resume primitives (Stream
//! `claude-checkpoint-resume`).
//!
//! [`CheckpointStore`] is the storage abstraction the executor
//! consults during [`crate::Executor::execute_with_checkpoints`]
//! and [`crate::Executor::resume`]. Production binds a sqlite
//! impl in `argo-cli`; tests bind [`InMemoryCheckpointStore`].
//!
//! Each [`Checkpoint`] captures one task's outputs at the moment
//! it succeeded. Resume replays the chain to restore the
//! executor's `outputs` map then runs only the still-pending
//! tasks. `cache_key` (per-task, optional) lets different runs
//! reuse a prior task's output without re-execution — handy for
//! skipping expensive LLM calls on retry.

use crate::dag::WorkflowId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use thiserror::Error;

/// Errors a [`CheckpointStore`] backend can surface. The exec
/// path treats checkpoint write errors as fatal (a silent
/// failure would defeat the point of resume).
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Underlying storage failure (sqlx, IO, etc.). The string
    /// is the backend's own message.
    #[error("checkpoint backend: {0}")]
    Backend(String),
    /// Resume requested for a run id with no checkpoints.
    #[error("no checkpoints for workflow run '{0}'")]
    NoCheckpoints(String),
    /// Rollback target seq doesn't exist on this run.
    #[error("checkpoint seq {seq} not found on run '{run}'")]
    SeqNotFound {
        /// Run id.
        run: String,
        /// Requested seq.
        seq: i64,
    },
}

/// One checkpoint row — emitted after a task succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Per-run sequence number (1-based).
    pub seq: i64,
    /// Workflow run id this checkpoint belongs to.
    pub workflow_run_id: WorkflowId,
    /// Task id within the workflow.
    pub task_id: String,
    /// Microseconds since epoch when the task finished.
    pub completed_at_micros: i64,
    /// Task outputs (the same map a downstream
    /// `${tasks.<id>.out.<key>}` reference resolves against).
    pub outputs: BTreeMap<String, serde_json::Value>,
    /// Optional cache key (set by the task in its YAML).
    pub cache_key: Option<String>,
}

/// Storage abstraction the executor uses to persist + replay
/// checkpoints.
#[async_trait]
pub trait CheckpointStore: Send + Sync {
    /// Append a checkpoint row. Returns the stored row's `seq`.
    async fn write(&self, ckpt: Checkpoint) -> Result<i64, CheckpointError>;

    /// All checkpoints for one workflow run, ordered by `seq`
    /// ascending.
    async fn list_for_run(
        &self,
        workflow_run_id: &WorkflowId,
    ) -> Result<Vec<Checkpoint>, CheckpointError>;

    /// Look up a previously-checkpointed task by its
    /// `cache_key` (matches the most recent row across all
    /// runs). Returns `None` if no row matches.
    async fn lookup_cache_key(
        &self,
        cache_key: &str,
    ) -> Result<Option<Checkpoint>, CheckpointError>;

    /// Drop every checkpoint row strictly above `target_seq`
    /// for one run. Used by rollback.
    async fn rollback_to(
        &self,
        workflow_run_id: &WorkflowId,
        target_seq: i64,
    ) -> Result<(), CheckpointError>;
}

/// Test-only in-memory backend. Not thread-safe across tasks
/// in production use.
pub struct InMemoryCheckpointStore {
    rows: Mutex<Vec<Checkpoint>>,
}

impl Default for InMemoryCheckpointStore {
    fn default() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
        }
    }
}

impl InMemoryCheckpointStore {
    /// Build an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn locked_rows(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Vec<Checkpoint>>, CheckpointError> {
        self.rows
            .lock()
            .map_err(|_| CheckpointError::Backend("in-memory mutex poisoned".into()))
    }
}

#[async_trait]
impl CheckpointStore for InMemoryCheckpointStore {
    async fn write(&self, mut ckpt: Checkpoint) -> Result<i64, CheckpointError> {
        let mut rows = self.locked_rows()?;
        let next_seq = rows
            .iter()
            .filter(|r| r.workflow_run_id == ckpt.workflow_run_id)
            .map(|r| r.seq)
            .max()
            .unwrap_or(0)
            + 1;
        ckpt.seq = next_seq;
        rows.push(ckpt);
        Ok(next_seq)
    }

    async fn list_for_run(
        &self,
        workflow_run_id: &WorkflowId,
    ) -> Result<Vec<Checkpoint>, CheckpointError> {
        let rows = self.locked_rows()?;
        let mut out: Vec<Checkpoint> = rows
            .iter()
            .filter(|r| &r.workflow_run_id == workflow_run_id)
            .cloned()
            .collect();
        out.sort_by_key(|r| r.seq);
        Ok(out)
    }

    async fn lookup_cache_key(
        &self,
        cache_key: &str,
    ) -> Result<Option<Checkpoint>, CheckpointError> {
        let rows = self.locked_rows()?;
        Ok(rows
            .iter()
            .filter(|r| r.cache_key.as_deref() == Some(cache_key))
            .max_by_key(|r| r.completed_at_micros)
            .cloned())
    }

    async fn rollback_to(
        &self,
        workflow_run_id: &WorkflowId,
        target_seq: i64,
    ) -> Result<(), CheckpointError> {
        let mut rows = self.locked_rows()?;
        let exists = rows
            .iter()
            .any(|r| &r.workflow_run_id == workflow_run_id && r.seq == target_seq);
        if !exists {
            return Err(CheckpointError::SeqNotFound {
                run: workflow_run_id.as_str().to_string(),
                seq: target_seq,
            });
        }
        rows.retain(|r| !(&r.workflow_run_id == workflow_run_id && r.seq > target_seq));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ckpt(run: &WorkflowId, task: &str, cache_key: Option<&str>) -> Checkpoint {
        Checkpoint {
            seq: 0,
            workflow_run_id: run.clone(),
            task_id: task.into(),
            completed_at_micros: 0,
            outputs: BTreeMap::new(),
            cache_key: cache_key.map(String::from),
        }
    }

    #[tokio::test]
    async fn in_memory_store_assigns_increasing_seq_per_run() {
        let store = InMemoryCheckpointStore::new();
        let run = WorkflowId("run-1".into());
        let s1 = store.write(ckpt(&run, "a", None)).await.unwrap();
        let s2 = store.write(ckpt(&run, "b", None)).await.unwrap();
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
    }

    #[tokio::test]
    async fn list_for_run_returns_only_matching_run_in_seq_order() {
        let store = InMemoryCheckpointStore::new();
        let r1 = WorkflowId("r1".into());
        let r2 = WorkflowId("r2".into());
        store.write(ckpt(&r1, "a", None)).await.unwrap();
        store.write(ckpt(&r2, "x", None)).await.unwrap();
        store.write(ckpt(&r1, "b", None)).await.unwrap();
        let got = store.list_for_run(&r1).await.unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].task_id, "a");
        assert_eq!(got[1].task_id, "b");
    }

    #[tokio::test]
    async fn lookup_cache_key_returns_most_recent_match_across_runs() {
        let store = InMemoryCheckpointStore::new();
        let r1 = WorkflowId("r1".into());
        let r2 = WorkflowId("r2".into());
        let mut older = ckpt(&r1, "summarise", Some("k1"));
        older.completed_at_micros = 10;
        let mut newer = ckpt(&r2, "summarise", Some("k1"));
        newer.completed_at_micros = 20;
        store.write(older).await.unwrap();
        store.write(newer.clone()).await.unwrap();
        let hit = store.lookup_cache_key("k1").await.unwrap().unwrap();
        assert_eq!(hit.workflow_run_id, r2);
    }

    #[tokio::test]
    async fn rollback_to_drops_rows_strictly_above_target_seq() {
        let store = InMemoryCheckpointStore::new();
        let run = WorkflowId("run".into());
        store.write(ckpt(&run, "a", None)).await.unwrap();
        store.write(ckpt(&run, "b", None)).await.unwrap();
        store.write(ckpt(&run, "c", None)).await.unwrap();
        store.rollback_to(&run, 1).await.unwrap();
        let remaining = store.list_for_run(&run).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].task_id, "a");
    }

    #[tokio::test]
    async fn rollback_to_unknown_seq_errors() {
        let store = InMemoryCheckpointStore::new();
        let run = WorkflowId("run".into());
        store.write(ckpt(&run, "a", None)).await.unwrap();
        let err = store.rollback_to(&run, 99).await.unwrap_err();
        assert!(matches!(err, CheckpointError::SeqNotFound { .. }));
    }

    #[tokio::test]
    async fn lookup_cache_key_misses_return_none() {
        let store = InMemoryCheckpointStore::new();
        assert!(store.lookup_cache_key("nope").await.unwrap().is_none());
    }
}

//! Checkpoint-aware execution paths for [`Executor`] (Stream
//! `claude-checkpoint-resume`). Lives in a sibling file so
//! `executor.rs` stays under the per-file LOC budget. Adds two
//! public methods (`execute_with_checkpoints`, `resume`) and a
//! shared `run_loop` that walks the DAG with checkpoint reads
//! + writes interleaved.

use crate::checkpoint::{Checkpoint, CheckpointStore};
use crate::dag::Trigger;
use crate::executor::{
    ExecutionError, ExecutionReport, ExecutionRequest, Executor, TaskOutcome, TaskRunner,
    TaskVerdict,
};
use crate::expr::resolve_inputs;
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};

impl<R: TaskRunner> Executor<R> {
    /// Execute the workflow, persisting a checkpoint after each
    /// task succeeds. Tasks declaring a `cache_key` are checked
    /// against the store first; a hit reuses the cached output
    /// without invoking the runner. Mid-run checkpoint write
    /// failures abort the run with [`ExecutionError::Checkpoint`].
    pub async fn execute_with_checkpoints(
        &self,
        req: ExecutionRequest,
        store: &dyn CheckpointStore,
    ) -> Result<ExecutionReport, ExecutionError> {
        self.run_loop(req, store, &BTreeMap::new()).await
    }

    /// Resume a previously-checkpointed run. The executor
    /// pre-populates its outputs map from `existing` (typically
    /// `store.list_for_run(run_id)`) and runs only tasks that
    /// have no checkpoint yet. New checkpoints are appended for
    /// the still-pending tasks.
    pub async fn resume(
        &self,
        req: ExecutionRequest,
        store: &dyn CheckpointStore,
        existing: &[Checkpoint],
    ) -> Result<ExecutionReport, ExecutionError> {
        let mut prior: BTreeMap<String, BTreeMap<String, serde_json::Value>> = BTreeMap::new();
        for c in existing {
            prior.insert(c.task_id.clone(), c.outputs.clone());
        }
        self.run_loop(req, store, &prior).await
    }

    async fn run_loop(
        &self,
        req: ExecutionRequest,
        store: &dyn CheckpointStore,
        prior_outputs: &BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    ) -> Result<ExecutionReport, ExecutionError> {
        let edges = req.workflow.validate().map_err(ExecutionError::Validate)?;
        let order = req
            .workflow
            .topo_order()
            .map_err(ExecutionError::Validate)?;
        let mut upstream: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for e in &edges {
            if matches!(e.trigger, Trigger::Success | Trigger::Always) {
                upstream
                    .entry(e.to_task.clone())
                    .or_default()
                    .push(e.from_task.clone());
            }
        }
        let mut outputs: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
            prior_outputs.clone();
        let mut verdicts: BTreeMap<String, TaskVerdict> = BTreeMap::new();
        let mut failed_or_skipped: BTreeSet<String> = BTreeSet::new();

        for task in order {
            if let Some(outs) = prior_outputs.get(&task.id) {
                verdicts.insert(
                    task.id.clone(),
                    TaskVerdict::Success {
                        outputs: outs.clone(),
                    },
                );
                continue;
            }
            if let Some(ups) = upstream.get(&task.id) {
                if let Some(bad) = ups.iter().find(|u| failed_or_skipped.contains(*u)) {
                    failed_or_skipped.insert(task.id.clone());
                    verdicts.insert(
                        task.id.clone(),
                        TaskVerdict::Skipped {
                            upstream: bad.clone(),
                        },
                    );
                    continue;
                }
            }
            if let Some(key) = task.cache_key.as_deref() {
                if let Some(hit) = store.lookup_cache_key(key).await? {
                    outputs.insert(task.id.clone(), hit.outputs.clone());
                    let _ = store
                        .write(Checkpoint {
                            seq: 0,
                            workflow_run_id: req.workflow_id.clone(),
                            task_id: task.id.clone(),
                            completed_at_micros: Utc::now().timestamp_micros(),
                            outputs: hit.outputs.clone(),
                            cache_key: Some(key.to_string()),
                        })
                        .await?;
                    verdicts.insert(
                        task.id.clone(),
                        TaskVerdict::Success {
                            outputs: hit.outputs,
                        },
                    );
                    continue;
                }
            }
            let resolved = resolve_inputs(&task.inputs, &outputs).map_err(|e| {
                ExecutionError::Resolve {
                    task: task.id.clone(),
                    error: e,
                }
            })?;
            match self.runner.run(task, &resolved).await {
                TaskOutcome::Success { outputs: outs } => {
                    outputs.insert(task.id.clone(), outs.clone());
                    let _ = store
                        .write(Checkpoint {
                            seq: 0,
                            workflow_run_id: req.workflow_id.clone(),
                            task_id: task.id.clone(),
                            completed_at_micros: Utc::now().timestamp_micros(),
                            outputs: outs.clone(),
                            cache_key: task.cache_key.clone(),
                        })
                        .await?;
                    verdicts.insert(task.id.clone(), TaskVerdict::Success { outputs: outs });
                }
                TaskOutcome::Failed { reason } => {
                    failed_or_skipped.insert(task.id.clone());
                    verdicts.insert(task.id.clone(), TaskVerdict::Failed { reason });
                }
            }
        }

        let ok = verdicts
            .values()
            .all(|v| matches!(v, TaskVerdict::Success { .. } | TaskVerdict::Skipped { .. }))
            && verdicts.values().any(|v| matches!(v, TaskVerdict::Success { .. }));
        let no_failures = !verdicts
            .values()
            .any(|v| matches!(v, TaskVerdict::Failed { .. }));
        Ok(ExecutionReport {
            workflow_id: req.workflow_id,
            task_verdicts: verdicts,
            ok: ok && no_failures,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::checkpoint::{Checkpoint, CheckpointStore, InMemoryCheckpointStore};
    use crate::dag::{TaskNode, Workflow, WorkflowId};
    use crate::executor::{ExecutionRequest, Executor, MockRunner, TaskVerdict};
    use std::collections::BTreeMap;

    fn task(id: &str, deps: &[&str], cache_key: Option<&str>) -> TaskNode {
        TaskNode {
            id: id.into(),
            worker: "w".into(),
            persona: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            cache_key: cache_key.map(String::from),
        }
    }

    fn linear(cache_key_b: Option<&str>) -> Workflow {
        Workflow {
            id: "linear".into(),
            description: String::new(),
            tasks: vec![
                task("a", &[], None),
                task("b", &["a"], cache_key_b),
                task("c", &["b"], None),
            ],
            edges: vec![],
        }
    }

    #[tokio::test]
    async fn execute_with_checkpoints_writes_one_row_per_successful_task() {
        let store = InMemoryCheckpointStore::new();
        let exec = Executor::new(MockRunner::echo());
        let report = exec
            .execute_with_checkpoints(ExecutionRequest::new(linear(None)), &store)
            .await
            .unwrap();
        assert!(report.ok);
        let rows = store.list_for_run(&report.workflow_id).await.unwrap();
        assert_eq!(rows.len(), 3);
        let task_order: Vec<&str> = rows.iter().map(|r| r.task_id.as_str()).collect();
        assert_eq!(task_order, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn execute_with_checkpoints_failed_task_stops_writes_for_downstream() {
        let store = InMemoryCheckpointStore::new();
        let exec = Executor::new(MockRunner::echo().fail("b"));
        let report = exec
            .execute_with_checkpoints(ExecutionRequest::new(linear(None)), &store)
            .await
            .unwrap();
        assert!(!report.ok);
        let rows = store.list_for_run(&report.workflow_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].task_id, "a");
    }

    #[tokio::test]
    async fn cache_key_hit_skips_runner_and_reuses_outputs() {
        let store = InMemoryCheckpointStore::new();
        let prior_run = WorkflowId("prior".into());
        let mut outs = BTreeMap::new();
        outs.insert(
            "echo".into(),
            serde_json::Value::String("from-cache".into()),
        );
        store
            .write(Checkpoint {
                seq: 0,
                workflow_run_id: prior_run,
                task_id: "b".into(),
                completed_at_micros: 1,
                outputs: outs.clone(),
                cache_key: Some("k-b".into()),
            })
            .await
            .unwrap();
        let exec = Executor::new(MockRunner::echo().fail("b"));
        let report = exec
            .execute_with_checkpoints(
                ExecutionRequest::new(linear(Some("k-b"))),
                &store,
            )
            .await
            .unwrap();
        assert!(report.ok, "cache hit should bypass the failing runner");
        let b_verdict = report.task_verdicts.get("b").unwrap();
        match b_verdict {
            TaskVerdict::Success { outputs } => {
                assert_eq!(
                    outputs.get("echo"),
                    Some(&serde_json::Value::String("from-cache".into()))
                );
            }
            other => panic!("expected b success from cache, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resume_skips_already_checkpointed_tasks_and_runs_remainder() {
        let store = InMemoryCheckpointStore::new();
        let req = ExecutionRequest::new(linear(None));
        let run_id = req.workflow_id.clone();
        let mut outs_a = BTreeMap::new();
        outs_a.insert("echo".into(), serde_json::Value::String("a".into()));
        store
            .write(Checkpoint {
                seq: 0,
                workflow_run_id: run_id.clone(),
                task_id: "a".into(),
                completed_at_micros: 1,
                outputs: outs_a,
                cache_key: None,
            })
            .await
            .unwrap();
        let existing = store.list_for_run(&run_id).await.unwrap();
        let exec = Executor::new(MockRunner::echo());
        let report = exec.resume(req, &store, &existing).await.unwrap();
        assert!(report.ok);
        for id in ["a", "b", "c"] {
            assert!(matches!(
                report.task_verdicts.get(id),
                Some(TaskVerdict::Success { .. })
            ));
        }
        let rows = store.list_for_run(&run_id).await.unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[tokio::test]
    async fn resume_with_no_existing_runs_full_workflow() {
        let store = InMemoryCheckpointStore::new();
        let exec = Executor::new(MockRunner::echo());
        let req = ExecutionRequest::new(linear(None));
        let run_id = req.workflow_id.clone();
        let report = exec.resume(req, &store, &[]).await.unwrap();
        assert!(report.ok);
        let rows = store.list_for_run(&run_id).await.unwrap();
        assert_eq!(rows.len(), 3);
    }
}

//! DAG executor — walks tasks in topological order, calls a
//! [`TaskRunner`] for each, propagates outputs into downstream
//! `${tasks...}` expressions, records per-task verdicts.
//!
//! The runner is an async trait so production callers can plug
//! in real worker dispatch (Session-per-task per the brief);
//! tests use [`MockRunner`].
use crate::dag::{TaskNode, Trigger, Workflow, WorkflowId};
use crate::expr::{resolve_inputs, ExprError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Outcome the runner returns for each task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    /// Task succeeded; outputs go into the execution context.
    Success {
        /// Task-emitted output map (`${tasks.<id>.out.<key>}`
        /// references resolve against this).
        outputs: BTreeMap<String, serde_json::Value>,
    },
    /// Task ran but failed. Downstream tasks following a `Success`
    /// edge are skipped.
    Failed {
        /// Free-form failure reason. Recorded into the verdict.
        reason: String,
    },
}

/// Final verdict per task — what happened, when, and any output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskVerdict {
    /// Task ran + succeeded.
    Success {
        /// Outputs the runner emitted.
        outputs: BTreeMap<String, serde_json::Value>,
    },
    /// Task ran + failed.
    Failed {
        /// Failure reason from the runner.
        reason: String,
    },
    /// Task was skipped because an upstream dependency failed.
    Skipped {
        /// Upstream task whose failure caused the skip.
        upstream: String,
    },
}

/// Aggregate report of an [`Executor::execute`] run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Workflow id.
    pub workflow_id: WorkflowId,
    /// Verdicts per task, keyed by task id.
    pub task_verdicts: BTreeMap<String, TaskVerdict>,
    /// `true` iff every task that ran succeeded (skipped tasks
    /// don't count against this).
    pub ok: bool,
}

/// One-shot dry-run plan rendering. Emitted by
/// [`Executor::dry_run`] without invoking the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunPlan {
    /// Workflow id.
    pub workflow_id: WorkflowId,
    /// Task ids in topological execution order.
    pub task_order: Vec<String>,
}

/// Errors surfaced by [`Executor::execute`].
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// Workflow failed shape validation.
    #[error("validate: {0}")]
    Validate(crate::dag::ValidationError),
    /// `${tasks...}` reference couldn't resolve.
    #[error("resolve inputs for task '{task}': {error}")]
    Resolve {
        /// Task whose inputs failed to resolve.
        task: String,
        /// Underlying expr error.
        error: ExprError,
    },
}

/// Request handed to [`Executor::execute`].
pub struct ExecutionRequest {
    workflow: Workflow,
    workflow_id: WorkflowId,
}

impl ExecutionRequest {
    /// Build a request with a freshly-generated id.
    #[must_use]
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow,
            workflow_id: WorkflowId::new(),
        }
    }

    /// Build a request with a caller-supplied id (e.g. when
    /// resuming or replaying).
    #[must_use]
    pub fn with_id(workflow: Workflow, id: WorkflowId) -> Self {
        Self {
            workflow,
            workflow_id: id,
        }
    }
}

/// Plug for the actual task implementation. Production binds an
/// impl that invokes worker dispatch; tests bind [`MockRunner`].
#[async_trait]
pub trait TaskRunner: Send + Sync {
    /// Run one task with its already-resolved inputs.
    async fn run(
        &self,
        task: &TaskNode,
        inputs: &BTreeMap<String, serde_json::Value>,
    ) -> TaskOutcome;
}

/// Walks a workflow's tasks in topological order via the
/// [`TaskRunner`]; propagates outputs into downstream inputs.
pub struct Executor<R: TaskRunner> {
    runner: R,
}

impl<R: TaskRunner> Executor<R> {
    /// Build an executor over the supplied runner.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Render the dry-run plan for a workflow without invoking
    /// the runner.
    pub fn dry_run(&self, workflow: &Workflow) -> Result<DryRunPlan, ExecutionError> {
        let order = workflow.topo_order().map_err(ExecutionError::Validate)?;
        Ok(DryRunPlan {
            workflow_id: WorkflowId::new(),
            task_order: order.iter().map(|t| t.id.clone()).collect(),
        })
    }

    /// Execute the workflow. Walks tasks in topo order; for each
    /// task: resolve inputs (against prior outputs), call the
    /// runner, record verdict. A failed task marks every
    /// downstream task on a `Success` edge as `Skipped`.
    pub async fn execute(
        &self,
        req: ExecutionRequest,
    ) -> Result<ExecutionReport, ExecutionError> {
        let edges = req.workflow.validate().map_err(ExecutionError::Validate)?;
        let order = req.workflow.topo_order().map_err(ExecutionError::Validate)?;
        // task_id → set of upstream task_ids (for Success edges)
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
            BTreeMap::new();
        let mut verdicts: BTreeMap<String, TaskVerdict> = BTreeMap::new();
        let mut failed_or_skipped: BTreeSet<String> = BTreeSet::new();

        for task in order {
            // Skip if any upstream succ-edge dependency failed/skipped.
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
            let resolved = resolve_inputs(&task.inputs, &outputs).map_err(|e| {
                ExecutionError::Resolve {
                    task: task.id.clone(),
                    error: e,
                }
            })?;
            match self.runner.run(task, &resolved).await {
                TaskOutcome::Success { outputs: outs } => {
                    outputs.insert(task.id.clone(), outs.clone());
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
        // Report `ok` only when no task `Failed`. Skipped tasks
        // alone don't make the run fail (they reflect an upstream
        // failure that's already accounted for).
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

/// Test-only runner that echoes inputs back as outputs. The
/// `failure_set` field can mark specific task ids as failures —
/// useful for the "downstream skip" test.
#[derive(Default)]
pub struct MockRunner {
    failure_set: BTreeSet<String>,
}

impl MockRunner {
    /// Echoes every input → output `{ "echo": <value> }` so
    /// downstream `${tasks.<id>.out.echo}` references resolve.
    #[must_use]
    pub fn echo() -> Self {
        Self::default()
    }

    /// Marks a task id as a forced failure when run. Builder-style.
    #[must_use]
    pub fn fail(mut self, task_id: &str) -> Self {
        self.failure_set.insert(task_id.to_string());
        self
    }
}

#[async_trait]
impl TaskRunner for MockRunner {
    async fn run(
        &self,
        task: &TaskNode,
        inputs: &BTreeMap<String, serde_json::Value>,
    ) -> TaskOutcome {
        if self.failure_set.contains(&task.id) {
            return TaskOutcome::Failed {
                reason: format!("mock-forced failure for task '{}'", task.id),
            };
        }
        let mut outputs = BTreeMap::new();
        // Echo each input under its own key.
        for (k, v) in inputs {
            outputs.insert(k.clone(), v.clone());
        }
        // Always emit a synthetic `echo` so downstream
        // `${tasks.<id>.out.echo}` references work for the
        // simplest "linear chain" test cases.
        outputs.insert(
            "echo".to_string(),
            serde_json::Value::String(task.id.clone()),
        );
        TaskOutcome::Success { outputs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::TaskNode;
    use std::collections::BTreeMap;

    fn task(id: &str, deps: &[&str]) -> TaskNode {
        TaskNode {
            id: id.to_string(),
            worker: "w".into(),
            persona: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn linear_workflow() -> Workflow {
        Workflow {
            id: "linear".into(),
            description: String::new(),
            tasks: vec![task("a", &[]), task("b", &["a"]), task("c", &["b"])],
            edges: vec![],
        }
    }

    #[tokio::test]
    async fn dry_run_emits_topological_task_order() {
        let exec = Executor::new(MockRunner::echo());
        let plan = exec.dry_run(&linear_workflow()).unwrap();
        assert_eq!(plan.task_order, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn execute_runs_every_task_in_a_clean_run() {
        let exec = Executor::new(MockRunner::echo());
        let report = exec
            .execute(ExecutionRequest::new(linear_workflow()))
            .await
            .unwrap();
        assert!(report.ok);
        assert_eq!(report.task_verdicts.len(), 3);
        for id in ["a", "b", "c"] {
            assert!(matches!(
                report.task_verdicts.get(id),
                Some(TaskVerdict::Success { .. })
            ));
        }
    }

    #[tokio::test]
    async fn failed_task_skips_downstream() {
        let exec = Executor::new(MockRunner::echo().fail("b"));
        let report = exec
            .execute(ExecutionRequest::new(linear_workflow()))
            .await
            .unwrap();
        assert!(!report.ok, "report.ok must be false when a task failed");
        assert!(matches!(
            report.task_verdicts.get("a"),
            Some(TaskVerdict::Success { .. })
        ));
        assert!(matches!(
            report.task_verdicts.get("b"),
            Some(TaskVerdict::Failed { .. })
        ));
        match report.task_verdicts.get("c") {
            Some(TaskVerdict::Skipped { upstream }) => assert_eq!(upstream, "b"),
            other => panic!("expected c skipped, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_propagates_outputs_into_downstream_inputs() {
        let mut a = task("a", &[]);
        a.inputs
            .insert("greeting".into(), serde_json::Value::String("hello".into()));
        let mut b = task("b", &["a"]);
        b.inputs.insert(
            "from_a".into(),
            serde_json::Value::String("${tasks.a.out.echo}".into()),
        );
        let w = Workflow {
            id: "out-binding".into(),
            description: String::new(),
            tasks: vec![a, b],
            edges: vec![],
        };
        let exec = Executor::new(MockRunner::echo());
        let report = exec.execute(ExecutionRequest::new(w)).await.unwrap();
        let b_verdict = report.task_verdicts.get("b").unwrap();
        match b_verdict {
            TaskVerdict::Success { outputs } => {
                assert_eq!(
                    outputs.get("from_a"),
                    Some(&serde_json::Value::String("a".into()))
                );
            }
            other => panic!("expected b success, got {other:?}"),
        }
    }
}

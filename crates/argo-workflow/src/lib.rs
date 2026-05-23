//! `argo-workflow` — declarative DAG workflow executor (Stream
//! `claude-workflow-dag`, V1.3 multi-worker).
//!
//! A workflow is a YAML describing a DAG of tasks; each task
//! specifies a worker (or persona-match) + inputs + outputs
//! referencing upstream tasks. The DAG executor schedules tasks
//! when their dependencies are satisfied.
//!
//! Pieces:
//! - [`Workflow`] / [`TaskNode`] / [`WorkflowEdge`] — declarative DAG
//!   shape. [`Workflow::validate`] enforces DAG topology (no
//!   cycles + every edge endpoint references a declared task).
//! - [`parse_yaml`] / [`parse_yaml_str`] — YAML + JSON-bridge
//!   parser. Schema-checks via serde then runs the topology
//!   validator.
//! - [`Expr`] — `${tasks.foo.out.bar}` upstream-output reference.
//!   Resolved at task-launch time against the running execution
//!   context.
//! - [`Executor`] / [`TaskRunner`] — the DAG executor walks
//!   ready tasks, calls the supplied `TaskRunner`, propagates
//!   outputs into downstream `Expr`s, and records a per-task
//!   verdict (Success / Failed / Skipped).
//!
//! ```
//! use argo_workflow::{parse_yaml_str, Executor, ExecutionRequest, MockRunner};
//! # async fn demo() {
//! let yaml = r#"
//! id: demo
//! tasks:
//!   - id: a
//!     worker: support-agent
//!     inputs: { greeting: "hello" }
//!   - id: b
//!     worker: support-agent
//!     inputs: { upstream: "${tasks.a.out.echo}" }
//!     depends_on: [a]
//! "#;
//! let workflow = parse_yaml_str(yaml).unwrap();
//! workflow.validate().unwrap();
//! let runner = MockRunner::echo();
//! let exec = Executor::new(runner);
//! let report = exec.execute(ExecutionRequest::new(workflow)).await.unwrap();
//! assert_eq!(report.task_verdicts.len(), 2);
//! # }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod checkpoint;
mod dag;
#[cfg(test)]
mod dag_tests;
mod executor;
mod executor_checkpoint;
mod expr;
/// Production [`TaskRunner`] that wires MCP dispatch + audit emission
/// + capability checks (`wire-worker-session-runner`). See
/// `runner.rs` module docs for the load-bearing contracts.
pub mod runner;
mod yaml;

pub use checkpoint::{Checkpoint, CheckpointError, CheckpointStore, InMemoryCheckpointStore};
pub use dag::{TaskNode, Trigger, ValidationError, Workflow, WorkflowEdge, WorkflowId};
pub use executor::{
    DryRunPlan, ExecutionReport, ExecutionRequest, Executor, MockRunner, TaskOutcome, TaskRunner,
    TaskVerdict,
};
pub use expr::{Expr, ExprError};
pub use runner::{
    StepAuditContext, ToolDispatchError, ToolDispatcher, WorkerSessionRunner,
    WorkflowAuditEmitter, WorkflowCapabilityChecker,
};
pub use yaml::{parse_yaml, parse_yaml_str, YamlError};

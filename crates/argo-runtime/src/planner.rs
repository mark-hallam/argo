//! Planner trait + two impls. See [`crate`] for the high-level
//! pitch.
//!
//! Errors during LLM calls or response parsing are returned
//! cleanly so the CLI can surface them, but never panic.

use argo_workflow::Workflow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// One node a [`Planner`] decomposed out of a coarse goal. Wrapped
/// for the [`SubtaskSink`] hand-off so the sink doesn't have to
/// re-derive the (path, description, worker) triple from the
/// raw [`argo_workflow::Workflow`] each time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSubtask {
    /// DAG path, typically `/<workflow_id>/<task_id>`.
    pub plan_node_path: String,
    /// Free-text description (workflow desc + task id).
    pub description: String,
    /// Worker id that owns the node — falls back to
    /// `ctx.worker_id` when the [`argo_workflow::TaskNode`]
    /// doesn't override.
    pub worker_id: String,
}

/// Sink for write-through of Planner output. Implemented in
/// `argo-store` (`subtask::SqliteSubtaskSink`) so the runtime
/// crate stays free of database deps. Tests use a vec-backed stub.
#[async_trait]
pub trait SubtaskSink: Send + Sync {
    /// Persist one decomposed subtask. Implementations are free to
    /// swallow conflicts (e.g. unique-index hits on the same plan
    /// node path) so re-decompose is idempotent.
    async fn record_subtask(
        &self,
        parent_goal: &str,
        parent_session_id: Option<&str>,
        subtask: &PlannedSubtask,
    ) -> Result<(), PlannerError>;
}

/// Walk a [`DecomposeResult`] and hand every task to the sink. The
/// CLI handler calls this immediately after `Planner::decompose`
/// returns.
pub async fn record_decomposition(
    sink: &dyn SubtaskSink,
    parent_goal: &str,
    parent_session_id: Option<&str>,
    ctx: &PlannerContext,
    result: &DecomposeResult,
) -> Result<(), PlannerError> {
    for task in &result.workflow.tasks {
        let worker_id = if task.worker.is_empty() {
            ctx.worker_id.clone()
        } else {
            task.worker.clone()
        };
        let plan_node_path = format!("/{}/{}", result.workflow.id, task.id);
        let description = if result.workflow.description.is_empty() {
            task.id.clone()
        } else {
            format!("{} :: {}", result.workflow.description, task.id)
        };
        sink.record_subtask(
            parent_goal,
            parent_session_id,
            &PlannedSubtask {
                plan_node_path,
                description,
                worker_id,
            },
        )
        .await?;
    }
    Ok(())
}

/// Decomposition errors. The CLI surfaces these as
/// `ArgoError::Provider`.
#[derive(Debug, Error)]
pub enum PlannerError {
    /// No template matched the goal in the available set.
    #[error("no template matched goal '{goal}' (available: {available:?})")]
    NoTemplateMatch {
        /// Goal that was rejected.
        goal: String,
        /// Names of templates that were considered.
        available: Vec<String>,
    },
    /// LLM call failed (network, auth, etc).
    #[error("llm call failed: {0}")]
    Llm(String),
    /// LLM responded but the YAML didn't parse / validate.
    #[error("llm response invalid: {0}")]
    InvalidResponse(String),
    /// YAML / template body failed parse.
    #[error("yaml parse: {0}")]
    YamlParse(String),
}

/// Pluggable Anthropic caller. Genesis already ships this
/// pattern in `argo_genesis::refine`; we keep our own copy here
/// so this crate doesn't depend on argo-genesis. Implementations
/// outside tests live in argo-cli or argo-provider; tests use a
/// canned-response stub.
#[async_trait]
pub trait AnthropicCaller: Send + Sync {
    /// One LLM call. The implementation handles retries / auth /
    /// model selection. Returns the raw response body — the
    /// caller is responsible for shape.
    async fn call(&self, prompt: String) -> Result<String, PlannerError>;
}

/// Context handed to the planner: which worker is asking, what
/// vertical context is in play, and any prior subtask outputs
/// (for re-decomposition mid-flight).
///
/// All fields are optional except `worker_id` so callers can
/// build progressively: a fresh `argo plan generate` only knows
/// the worker; richer integrations add the rest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlannerContext {
    /// Worker that owns the eventual Session if `--apply` runs.
    /// Used inside YAML rendering as the default `worker:` field.
    pub worker_id: String,
    /// Optional vertical id (matches `argo vertical list`).
    pub vertical: Option<String>,
    /// Optional persona expression for tasks the planner should
    /// label `persona:` instead of `worker:`.
    pub default_persona: Option<String>,
    /// Free-form key→value hints (skill list, tenant, etc) folded
    /// into the LLM prompt.
    pub hints: HashMap<String, String>,
}

impl PlannerContext {
    /// Tiny builder — most call sites only know the worker id.
    #[must_use]
    pub fn for_worker(worker_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            ..Self::default()
        }
    }

    /// Set the vertical id.
    #[must_use]
    pub fn with_vertical(mut self, vertical: impl Into<String>) -> Self {
        self.vertical = Some(vertical.into());
        self
    }

    /// Add a free-form hint.
    pub fn add_hint(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.hints.insert(key.into(), value.into());
    }
}

/// One decompose call's full result. Carries the workflow plus
/// the planner's reasoning string so `argo plan show` can render
/// "why these subtasks?" without re-running the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeResult {
    /// Resolved DAG. Already validated.
    pub workflow: Workflow,
    /// Free-form reasoning the planner emitted alongside the
    /// workflow. For LlmPlanner this is the model's commentary;
    /// for TemplatePlanner this is the matching rule's
    /// description.
    pub reasoning: String,
    /// Which planner ran (`"template"` / `"llm"`). Useful for
    /// audit.
    pub planner_kind: String,
    /// Template name that matched, when applicable.
    pub template_name: Option<String>,
}

/// Planner trait — decompose a goal into a workflow DAG. Errors
/// are surfaced cleanly; the CLI is responsible for fall-through.
#[async_trait]
pub trait Planner: Send + Sync {
    /// Decompose `goal` under `ctx`. Implementations validate
    /// the resulting workflow before returning.
    async fn decompose(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<DecomposeResult, PlannerError>;
}

pub use crate::planner_templates::{PlanTemplate, TemplatePlanner};

/// LLM-backed planner. Calls Anthropic via the pluggable
/// [`AnthropicCaller`] and parses the response as a workflow YAML.
pub struct LlmPlanner {
    caller: Box<dyn AnthropicCaller>,
}

impl std::fmt::Debug for LlmPlanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmPlanner").finish_non_exhaustive()
    }
}

impl LlmPlanner {
    /// Build over a caller. Tests pass a canned-response stub;
    /// the CLI passes the real Anthropic client wrapper.
    #[must_use]
    pub fn new(caller: Box<dyn AnthropicCaller>) -> Self {
        Self { caller }
    }

    /// Build the planning prompt. Exposed for testability.
    #[must_use]
    pub fn build_prompt(&self, goal: &str, ctx: &PlannerContext) -> String {
        let mut prompt = String::new();
        prompt.push_str(PLANNING_INSTRUCTIONS);
        prompt.push_str("\n\nWorker id: ");
        prompt.push_str(&ctx.worker_id);
        if let Some(v) = &ctx.vertical {
            prompt.push_str("\nVertical: ");
            prompt.push_str(v);
        }
        if !ctx.hints.is_empty() {
            prompt.push_str("\nContext hints:");
            for (k, v) in &ctx.hints {
                prompt.push_str(&format!("\n  {k}: {v}"));
            }
        }
        prompt.push_str("\n\nGoal:\n");
        prompt.push_str(goal);
        prompt.push_str("\n\nRespond with a JSON object: ");
        prompt.push_str("{\"reasoning\": \"...\", \"workflow_yaml\": \"...\"}");
        prompt
    }
}

#[async_trait]
impl Planner for LlmPlanner {
    async fn decompose(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<DecomposeResult, PlannerError> {
        let prompt = self.build_prompt(goal, ctx);
        let raw = self.caller.call(prompt).await?;
        let response: LlmResponse = serde_json::from_str(&raw)
            .map_err(|e| PlannerError::InvalidResponse(format!("json: {e}")))?;
        let workflow = argo_workflow::parse_yaml_str(&response.workflow_yaml).map_err(|e| {
            PlannerError::InvalidResponse(format!("workflow yaml: {e}"))
        })?;
        let workflow = apply_context(workflow, ctx);
        Ok(DecomposeResult {
            workflow,
            reasoning: response.reasoning,
            planner_kind: "llm".into(),
            template_name: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    reasoning: String,
    workflow_yaml: String,
}

/// Apply context defaults to the workflow's tasks: any task
/// missing a `worker:` and `persona:` falls back to
/// `ctx.worker_id`. Also called from [`crate::planner_templates`].
pub(crate) fn apply_context(mut workflow: Workflow, ctx: &PlannerContext) -> Workflow {
    for task in &mut workflow.tasks {
        if task.worker.is_empty() && task.persona.is_none() {
            task.worker = ctx.worker_id.clone();
        }
    }
    workflow
}

const PLANNING_INSTRUCTIONS: &str =
    "You are a workflow planner. Decompose the user's goal into a small DAG of \
     tasks (3-7 nodes). Each task has an id, a worker reference, and free-form \
     inputs. Edges express dependency.";

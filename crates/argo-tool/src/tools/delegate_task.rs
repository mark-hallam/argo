//! `delegate_task` — declared in-core per §9. Emits a Pending
//! [`Handoff`](argo_orchestrator::Handoff) to the target worker
//! through [`OrchestratorStore::insert_handoff`]. The actual
//! subagent spawn + fallback-chain inheritance lands separately
//! (`mission-delegate-task-fallback-chain-v2`); this surface
//! covers the wire-up — caller context lives on
//! [`ToolContext::session_id`] / [`ToolContext::current_handoff_id`],
//! and a wired `OrchestratorStore` handle is what flips dispatch
//! out of the historical "not wired" error path.
use crate::tool::{error_text, success_text, Tool, ToolContext};
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, OrchestratorStore};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Tenant used by the in-process agent-runner dispatch path. The
/// inner agent loop is currently single-tenant per process, so a
/// stable nil UUID matches the sentinel used by
/// `argo-agent-runner::dispatch::dispatch_tenant`. The downstream
/// fallback-chain brief is the natural place to thread a real
/// tenant id through `ToolContext`.
fn dispatch_tenant() -> argo_types::TenantId {
    argo_types::TenantId(Uuid::nil())
}

/// In-core `delegate_task` tool. `Default::default()` returns a
/// stub that errors at dispatch — used by the legacy
/// `builtin_tools(cwd)` call sites that don't have an
/// `OrchestratorStore` to thread through. The wired form
/// [`DelegateTask::new`] actually emits the handoff.
#[derive(Default)]
pub struct DelegateTask {
    store: Option<Arc<OrchestratorStore>>,
}

impl std::fmt::Debug for DelegateTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegateTask")
            .field("wired", &self.store.is_some())
            .finish()
    }
}

impl DelegateTask {
    /// Build a delegate-task tool wired to a live [`OrchestratorStore`].
    /// The agent runner calls this once per session so dispatch can
    /// actually persist a [`Handoff`].
    ///
    /// ```
    /// # async fn doc() -> argo_types::ArgoResult<()> {
    /// use std::sync::Arc;
    /// use argo_orchestrator::OrchestratorStore;
    /// use argo_store::SessionStore;
    /// use argo_tool::DelegateTask;
    /// let session = SessionStore::open(":memory:").await?;
    /// session.migrate().await?;
    /// let orch = Arc::new(OrchestratorStore::new(Arc::new(session)));
    /// let _tool = DelegateTask::new(orch);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn new(store: Arc<OrchestratorStore>) -> Self {
        Self { store: Some(store) }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    /// Target worker id (UUID v7 string).
    worker_id: String,
    /// What the receiver should accomplish; surfaces as
    /// `Handoff.expected_outcome`.
    task: String,
    /// Opaque payload JSON forwarded verbatim to the receiver.
    /// Defaults to `{}`.
    #[serde(default)]
    payload: Option<serde_json::Value>,
    /// Optional deadline, milliseconds from now. When set the
    /// emitted handoff carries a `deadline` that the orchestrator
    /// sweep can expire.
    #[serde(default)]
    deadline_ms: Option<u64>,
    /// Reserved for the downstream fallback-chain brief; the
    /// current impl forwards it verbatim into the handoff payload
    /// so the receiver can pick it up without a payload-shape
    /// migration later.
    #[serde(default)]
    model: Option<String>,
    /// Reserved (see `model`).
    #[serde(default)]
    budget_tokens: Option<u32>,
}

#[async_trait]
impl Tool for DelegateTask {
    fn name(&self) -> &str {
        "delegate_task"
    }
    fn description(&self) -> &str {
        "Hand a sub-task to a child Argo agent with its own budget and \
         context. Emits a Pending handoff to the target worker; the \
         orchestrator drives the actual handoff lifecycle. Inherits \
         the parent's fallback chain by default (Amendment 7)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["worker_id", "task"],
            "properties": {
                "worker_id": { "type": "string", "description": "Target worker id (UUID v7)." },
                "task": { "type": "string", "description": "Self-contained task statement; surfaces as Handoff.expected_outcome." },
                "payload": { "type": "object", "description": "Opaque payload JSON forwarded to the receiver." },
                "deadline_ms": { "type": "integer", "minimum": 1, "description": "Optional handoff deadline, ms from now." },
                "model": { "type": "string", "description": "Reserved: model override the receiver should honour." },
                "budget_tokens": { "type": "integer", "minimum": 1, "description": "Reserved: token budget for the receiver." },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::Spawn {
            spawn_kind: SpawnKind::Subagent,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = serde_json::from_value(args).map_err(|e| ArgoError::Tool {
            tool: "delegate_task".into(),
            msg: format!("invalid arguments: {e}"),
        })?;
        let Some(store) = self.store.as_ref() else {
            return Ok(error_text(
                ctx.call_id,
                "delegate_task not wired — agent runner did not supply an \
                 OrchestratorStore handle. Tool stays usable only from \
                 turns dispatched under the wired agent runner."
                    .into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        let target = match WorkerId::from_str(&parsed.worker_id) {
            Ok(id) => id,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("delegate_task: invalid worker_id '{}': {e}", parsed.worker_id),
                    started.elapsed().as_millis() as u64,
                ));
            }
        };
        let payload = build_payload(&parsed);
        let mut handoff = Handoff::new(
            ctx.session_id.clone(),
            WorkerId::nil(),
            EscalationStep::Worker(target),
            parsed.task,
            payload,
        );
        if let Some(ms) = parsed.deadline_ms {
            handoff.deadline = Some(Utc::now() + chrono::Duration::milliseconds(ms as i64));
        }
        match store.insert_handoff(dispatch_tenant(), &handoff).await {
            Ok(()) => Ok(success_text(
                ctx.call_id,
                handoff.id.to_string(),
                started.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(error_text(
                ctx.call_id,
                format!("delegate_task: insert_handoff failed: {e}"),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

fn build_payload(args: &Args) -> serde_json::Value {
    let mut p = args
        .payload
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = p.as_object_mut() {
        if let Some(model) = &args.model {
            obj.insert("model".into(), serde_json::Value::String(model.clone()));
        }
        if let Some(budget) = args.budget_tokens {
            obj.insert("budget_tokens".into(), serde_json::Value::from(budget));
        }
    }
    p
}

#[cfg(test)]
#[path = "delegate_task_tests.rs"]
mod tests;

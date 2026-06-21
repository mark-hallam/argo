//! `enqueue_action` — the in-turn chat→delivery bridge tool (WAVE-2 ⑥b).
//!
//! ## What it does
//!
//! This is the in-turn entry point a teammate uses to actually *submit*
//! a drafted action for approval + delivery, mid-conversation. The model
//! calls it with an `action_type` (one the bound vertical declares) and a
//! `payload`; the runtime materializes that into a real `worker_action`
//! row that targets the vertical's REAL executor and is born
//! **pending-approval** behind the operator gate. Nothing auto-executes.
//!
//! Before this tool the chat plane could only *draft* and stop: a teammate
//! that wanted to write a document / call a connector / post over a channel
//! had no in-turn way to push the draft toward delivery. This tool closes
//! that gap while keeping the operator in the loop.
//!
//! ## Layering (why the heavy logic is injected, not inlined)
//!
//! The actual materialization
//! ([`argo-cli`'s `enqueue_action_for_delivery`]) lives in the TOP `argo-cli`
//! crate — it needs the bound [`argo_vertical::VerticalManifest`], the
//! [`argo_memory::MemoryStore`], the workspace, and the originating
//! `job_id` / `worker_id`. `argo-tool` sits BELOW `argo-cli` and cannot call
//! up into it. So this tool depends ONLY on the [`DeliverDispatcher`] trait
//! defined here; the production implementation (which captures all that
//! context) is constructed in `argo-cli` and injected as an
//! `Arc<dyn DeliverDispatcher>` at turn setup. This is the same
//! inject-the-backend pattern [`super::send_message`] / [`super::clarify`]
//! / [`super::vision`] use, and it introduces NO upward crate dependency.
//!
//! ## Truth-bounded result (DEC-029 NO THEATER)
//!
//! On success the tool returns **"submitted for approval"** — never
//! "done" / "delivered" / "sent". The action is parked at the approval
//! gate; the existing executor path delivers it only after an operator
//! approves. The teammate that called it must report the same.
//!
//! ```
//! use argo_tool::{EnqueueAction, Tool};
//! let t = EnqueueAction::default();
//! assert_eq!(t.name(), "enqueue_action");
//! ```
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// Successful outcome a [`DeliverDispatcher`] returns: the action was
/// materialized + persisted **pending-approval**, targeting a real
/// executor. There is deliberately no "delivered" variant — the bridge
/// stops at the approval gate.
#[derive(Debug, Clone)]
pub struct DeliverOutcome {
    /// The persisted `worker_action` row id (the operator approves THIS
    /// row in `/approvals`).
    pub action_id: String,
    /// The real executor the action targets (e.g. `document_csv`, `mcp`,
    /// `browser`). Never `human` — the dispatcher refuses that shape.
    pub executor_kind: String,
}

/// Backend trait the `enqueue_action` tool calls into. The runtime
/// (`argo-cli`) constructs the production implementation per turn —
/// capturing the bound vertical manifest, memory store, workspace, and
/// originating job/worker — and injects it via
/// [`EnqueueAction::with_dispatcher`]. Tests inject a capture stub.
///
/// The implementation MUST submit the action through the approval gate
/// (state `pending`, `requires_approval = true`) and MUST NOT execute it.
#[async_trait]
pub trait DeliverDispatcher: Send + Sync + std::fmt::Debug {
    /// Materialize `action_type` (+ `payload`) into a real
    /// `worker_action` row targeting the bound vertical's declared
    /// executor, persisted **pending-approval**. Returns the parked
    /// row's id + the executor it targets on success, or a human-readable
    /// error string on a fail-closed rejection (undeclared action,
    /// `human` dead-end, or an un-gated auto-deliver action).
    async fn enqueue(
        &self,
        action_type: &str,
        payload: serde_json::Value,
    ) -> Result<DeliverOutcome, String>;
}

/// The default, unwired backend. Returns a clear "not wired" error so the
/// tool still appears in the model's schema and a misconfigured runtime
/// gets a useful diagnostic (mirrors [`super::send_message`]'s
/// `UnwiredDispatcher`).
#[derive(Debug, Default)]
struct UnwiredDeliverDispatcher;

#[async_trait]
impl DeliverDispatcher for UnwiredDeliverDispatcher {
    async fn enqueue(
        &self,
        _action_type: &str,
        _payload: serde_json::Value,
    ) -> Result<DeliverOutcome, String> {
        Err("enqueue_action backend not wired — this turn was not composed \
             with a delivery bridge (only worker/job turns can submit actions \
             for approval)"
            .into())
    }
}

/// In-turn tool that submits a drafted action for approval + delivery.
#[derive(Debug, Clone)]
pub struct EnqueueAction {
    dispatcher: Arc<dyn DeliverDispatcher>,
}

impl Default for EnqueueAction {
    fn default() -> Self {
        Self {
            dispatcher: Arc::new(UnwiredDeliverDispatcher),
        }
    }
}

impl EnqueueAction {
    /// Build the tool with a real delivery backend. The runtime uses this
    /// at turn setup to inject the `argo-cli` bridge dispatcher; tests
    /// inject a capture stub.
    #[must_use]
    pub fn with_dispatcher(dispatcher: Arc<dyn DeliverDispatcher>) -> Self {
        Self { dispatcher }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    action_type: String,
    #[serde(default)]
    payload: serde_json::Value,
}

#[async_trait]
impl Tool for EnqueueAction {
    fn name(&self) -> &str {
        "enqueue_action"
    }

    fn description(&self) -> &str {
        "Submit a drafted action for operator approval and delivery. \
         `action_type` must be one your bound role declares (e.g. \
         `export_report`, `send_invoice`); `payload` is the action's \
         arguments. The action is queued PENDING APPROVAL — it is NOT \
         delivered until an operator approves it in /approvals. On success \
         this returns 'submitted for approval', never 'done' or \
         'delivered'. Use this when you have drafted something the user \
         wants actually carried out (a document written, a message sent, a \
         connector called)."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action_type"],
            "properties": {
                "action_type": {
                    "type": "string",
                    "description": "The declared action to enqueue (must exist in your role's action vocabulary).",
                },
                "payload": {
                    "type": "object",
                    "description": "Arguments for the action (shape depends on the action_type).",
                    "additionalProperties": true,
                },
            },
            "additionalProperties": false,
        })
    }

    fn concurrency(&self) -> Concurrency {
        // Persisting an approval row is a stateful write; serialize so two
        // parallel enqueues can't interleave on the same job's ledger.
        Concurrency::Serial
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        // The registry already enforces `ToolDispatch { "enqueue_action" }`
        // before this tool runs, so a worker must carry that grant (via its
        // profile allow-set) to reach the tool at all. No EXTRA fine-grained
        // capability is required: the row is parked at the approval gate and
        // never executes, so submitting it is not itself a side-effecting
        // capability — the executor run (which IS gated) happens later, on
        // operator approval, through the existing execute path.
        Ok(CapabilitySet(vec![]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("invalid arguments: {e}"),
                    started.elapsed().as_millis() as u64,
                ))
            }
        };
        let action_type = parsed.action_type.trim();
        if action_type.is_empty() {
            return Ok(error_text(
                ctx.call_id,
                "action_type must not be empty".to_string(),
                started.elapsed().as_millis() as u64,
            ));
        }
        match self.dispatcher.enqueue(action_type, parsed.payload).await {
            Ok(outcome) => {
                // Truth-bounded: "submitted for approval", never delivered.
                let body = serde_json::json!({
                    "status": "submitted for approval",
                    "action_id": outcome.action_id,
                    "action_type": action_type,
                    "executor": outcome.executor_kind,
                    "delivered": false,
                    "note": "Queued pending operator approval; it will be \
                             delivered only after an operator approves it in \
                             /approvals.",
                });
                Ok(success_text(
                    ctx.call_id,
                    serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".into()),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(e) => Ok(error_text(
                ctx.call_id,
                format!("could not submit action for approval: {e}"),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

#[cfg(test)]
#[path = "deliver_tests.rs"]
mod tests;

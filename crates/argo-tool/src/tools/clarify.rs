//! `clarify` — pause mid-run to ask the operator a question (Hermes
//! parity, brief `tool-todo-and-clarify`).
//!
//! ## Stub-only by default
//!
//! The brief asked us to wrap the existing Ask/Approval pathway used
//! for gated tools. Today that pathway is a manifest declaration
//! (`[escalation].human_approval_required` honoured at dispatch in
//! `argo-agent-runner/src/turn_helpers.rs`); it has no callable
//! "enqueue a question, await an answer" surface. Per the brief's
//! "do not invent a new surface" rule we ship this tool with its
//! schema present always, and a tool-side [`ClarifyDispatcher`]
//! trait that the agent runner can wire later (mirrors the
//! [`crate::Vision`] / [`crate::Memory`] pattern). Default-constructed
//! `Clarify` returns a clear "clarify dispatcher not wired" error at
//! dispatch — the model still sees the tool in its list so it can
//! plan around availability.
//!
//! Capability gate: declared via existing
//! [`argo_types::Capability::ToolDispatch`] with
//! `tool_name_glob = "clarify"` — no new capability variant needed.
//!
//! ## Example
//!
//! ```
//! use argo_tool::Clarify;
//! // Default form — schema present, dispatch errors with a clear
//! // "not wired" hint until the agent runner wires a dispatcher.
//! let _c = Clarify::default();
//! ```
use crate::tool::{error_text, success_text, Tool, ToolContext};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// `clarify` tool — schema present always, dispatch errors cleanly
/// until [`Clarify::new`] wires a live dispatcher.
#[derive(Debug, Default)]
pub struct Clarify {
    wired: Option<Arc<dyn ClarifyDispatcher>>,
}

impl Clarify {
    /// Build a `Clarify` tool wired to a live dispatcher. The agent
    /// runner constructs this once per turn from whatever
    /// operator-facing Ask surface is available (TUI, gateway,
    /// CLI prompt).
    #[must_use]
    pub fn new(dispatcher: Arc<dyn ClarifyDispatcher>) -> Self {
        Self {
            wired: Some(dispatcher),
        }
    }
}

/// Operator-facing Ask surface. Implemented by the agent runner's
/// session layer (out of scope for this brief — wiring lands in a
/// follow-up alongside the operator's reply channel).
#[async_trait]
pub trait ClarifyDispatcher: Send + Sync + std::fmt::Debug {
    /// Stable surface id (`"tui"`, `"gateway"`, `"cli"`) — surfaced
    /// in the success payload so the model can cite the channel that
    /// answered.
    fn surface_id(&self) -> &str;

    /// Send the question to the operator and return their answer.
    /// `options`, when present, are the multiple-choice answers the
    /// model offered; the dispatcher may return either an option
    /// verbatim or a free-form reply.
    async fn ask(
        &self,
        question: &str,
        options: Option<&[String]>,
    ) -> Result<ClarifyAnswer, String>;
}

/// Operator's reply to a `clarify` call.
#[derive(Debug, Clone, Default)]
pub struct ClarifyAnswer {
    /// The operator's answer text. When `options` were supplied this
    /// is typically one of them; otherwise it is free-form.
    pub answer: String,
    /// `Some(i)` when the answer matched `options[i]`; `None` for
    /// free-form replies.
    pub option_index: Option<usize>,
}

impl ClarifyAnswer {
    /// Convenience for tests / wirings that always free-form.
    #[must_use]
    pub fn free_form(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            option_index: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    question: String,
    #[serde(default)]
    options: Option<Vec<String>>,
}

#[async_trait]
impl Tool for Clarify {
    fn name(&self) -> &str {
        "clarify"
    }

    fn description(&self) -> &str {
        "Pause to ask the operator a clarifying question. \
         `question` is the natural-language ask; `options`, when \
         provided, is a list of suggested answers to choose from \
         (free-form reply still allowed). Returns the operator's \
         answer."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["question"],
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The clarifying question to ask the operator.",
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of suggested answers.",
                },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let _ = parse_args(args)?;
        Ok(CapabilitySet(vec![Capability::ToolDispatch {
            tool_name_glob: "clarify".into(),
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed = parse_args(&args)?;
        let Some(dispatcher) = self.wired.as_ref() else {
            return Ok(error_text(
                ctx.call_id,
                "clarify dispatcher not wired — register via \
                 `Clarify::new(dispatcher)` (the agent runner wires \
                 this from the session's operator surface) instead \
                 of `builtin_tools(cwd)`'s default."
                    .into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        match dispatcher
            .ask(&parsed.question, parsed.options.as_deref())
            .await
        {
            Ok(answer) => {
                let payload = serde_json::json!({
                    "answer": answer.answer,
                    "option_index": answer.option_index,
                    "surface": dispatcher.surface_id(),
                });
                Ok(success_text(
                    ctx.call_id,
                    serde_json::to_string(&payload).unwrap_or_default(),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(msg) => Ok(error_text(
                ctx.call_id,
                format!("clarify: dispatcher failed: {msg}"),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

fn parse_args(args: &serde_json::Value) -> ArgoResult<Args> {
    serde_json::from_value(args.clone()).map_err(|e| ArgoError::Tool {
        tool: "clarify".into(),
        msg: format!("invalid arguments: {e}"),
    })
}

#[cfg(test)]
#[path = "clarify_tests.rs"]
mod tests;

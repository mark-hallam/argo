//! Walking-skeleton `Agent` (§6, Phase 1).
//!
//! Scope:
//! - `run_turn` with non-streaming provider call.
//! - Single tool dispatch path (no parallelism, no compression, no
//!   retry/repair, no streaming).
//! - Capability check before each tool dispatch (via `ToolRegistry`).
//! - Append every conversation message to `SessionStore`.
//! - Hash-chained `AuditRecord` per turn boundary + tool dispatch.
//!
//! Stops short of: streaming, parallel tool execution, compression,
//! retry/repair, subagent delegation, routing, judge integration, steer
//! hooks, heartbeat. Those land with Phase 2 of §6.
use crate::audit::{AuditKind, AuditSink};
use crate::steer::SteerChannel;
use argo_capability::CapabilityEngine;
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_tool::{ToolContext, ToolRegistry};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, Principal, SessionId,
    ToolCallId,
};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

/// Dependencies for the agent loop. Phase-1 minimum surface; Phase 2
/// extends with router / judge / steer / telemetry / memory / skills.
#[derive(Clone)]
pub struct AgentDeps {
    /// Session + message persistence.
    pub store: Arc<argo_store::SessionStore>,
    /// Provider transport (non-streaming for Phase 1).
    pub transport: Arc<dyn Transport>,
    /// Tool registry — `dispatch` does the cap check.
    pub registry: Arc<ToolRegistry>,
    /// Capability engine.
    pub capability_engine: Arc<CapabilityEngine>,
    /// Hash-chained audit log.
    pub audit: Arc<AuditSink>,
    /// Mid-run steer channel (Amendment 14). Drained between tool
    /// results and the next provider call so steers reach the
    /// model on the very next turn without breaking prompt cache.
    pub steer: Arc<SteerChannel>,
}

impl std::fmt::Debug for AgentDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentDeps")
            .field("transport_id", &self.transport.id())
            .field("registry", &self.registry)
            .finish()
    }
}

/// Per-turn knobs.
#[derive(Debug, Clone)]
pub struct RunTurnOptions {
    /// Hard cap on assistant ↔ tool round-trips per turn (Phase 1
    /// equivalent of §6.6's iteration budget; auto-continue is
    /// out-of-scope for the walking skeleton).
    pub max_iterations: u32,
    /// Optional system prompt.
    pub system: Option<String>,
    /// Acting principal for cap checks + audit attribution.
    pub principal: Principal,
    /// Workspace root for tools (file_read et al. resolve here).
    pub workspace_root: PathBuf,
    /// Provider model to call.
    pub model: String,
    /// Sampling knobs for the provider call.
    pub sampling: Sampling,
}

/// Final outcome of a `run_turn` call.
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    /// The final assistant text the user sees.
    pub final_text: String,
    /// How many provider round-trips were spent.
    pub iterations: u32,
    /// Total provider-reported input tokens (sum across iterations).
    pub tokens_in: u64,
    /// Total provider-reported output tokens.
    pub tokens_out: u64,
}

/// The walking-skeleton agent.
pub struct Agent {
    deps: AgentDeps,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent").field("deps", &self.deps).finish()
    }
}

impl Agent {
    /// Build over a complete `AgentDeps` set.
    #[must_use]
    pub fn new(deps: AgentDeps) -> Self {
        Self { deps }
    }

    /// Borrow the dependency bundle (for tests + telemetry).
    #[must_use]
    pub fn deps(&self) -> &AgentDeps {
        &self.deps
    }

    /// Run one turn: append the user message, call the provider,
    /// dispatch any tool calls, loop until the assistant returns plain
    /// text (no tool calls) — or `max_iterations` is hit.
    pub async fn run_turn(
        &self,
        session_id: SessionId,
        user_input: String,
        opts: RunTurnOptions,
    ) -> ArgoResult<TurnOutcome> {
        self.deps
            .audit
            .write(
                Some(session_id),
                AuditKind::TurnStart,
                serde_json::json!({
                    "session_id": session_id.to_string(),
                    "model": opts.model,
                    "principal": opts.principal,
                }),
                None,
            )
            .await?;
        let user_msg = build_user_message(session_id, &user_input);
        self.persist_message(session_id, &user_msg).await?;
        let mut history: Vec<Message> = vec![user_msg];
        let mut iter: u32 = 0;
        let mut tokens_in: u64 = 0;
        let mut tokens_out: u64 = 0;
        let final_text = loop {
            iter = iter.saturating_add(1);
            if iter > opts.max_iterations {
                self.audit_halt(session_id, "max_iterations").await?;
                return Err(ArgoError::BudgetExhausted {
                    kind: argo_types::BudgetKind::Iterations,
                });
            }
            let req = build_provider_request(&opts, &history);
            let resp = self.deps.transport.complete(&req).await?;
            tokens_in = tokens_in.saturating_add(u64::from(resp.tokens_in.unwrap_or(0)));
            tokens_out = tokens_out.saturating_add(u64::from(resp.tokens_out.unwrap_or(0)));
            let assistant = build_assistant_message(session_id, &resp);
            self.persist_message(session_id, &assistant).await?;
            self.deps
                .audit
                .write(
                    Some(session_id),
                    AuditKind::AssistantMessage,
                    serde_json::json!({
                        "iteration": iter,
                        "text_len": assistant
                            .content
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::Text { text } => Some(text.len()),
                                _ => None,
                            })
                            .sum::<usize>(),
                        "tool_calls": resp.tool_calls.len(),
                    }),
                    None,
                )
                .await?;
            if resp.tool_calls.is_empty() {
                break extract_text(&assistant);
            }
            history.push(assistant);
            for call in &resp.tool_calls {
                let result_msg = self.dispatch_one(session_id, call, &opts).await?;
                self.persist_message(session_id, &result_msg).await?;
                history.push(result_msg);
            }
            // Amendment 14: drain any steers enqueued during this
            // turn and inject them as a synthesized user message
            // BEFORE the next provider call. Steers come after the
            // last cached prefix so the next call still hits cache.
            let steers = self.deps.steer.drain(session_id);
            if !steers.is_empty() {
                let steer_msg = build_steer_message(session_id, &steers);
                self.persist_message(session_id, &steer_msg).await?;
                self.deps
                    .audit
                    .write(
                        Some(session_id),
                        AuditKind::SteerEnqueued,
                        serde_json::json!({
                            "session_id": session_id.to_string(),
                            "by_principal": opts.principal,
                            "count": steers.len(),
                        }),
                        None,
                    )
                    .await?;
                history.push(steer_msg);
            }
        };
        self.deps
            .audit
            .write(
                Some(session_id),
                AuditKind::TurnEnd,
                serde_json::json!({
                    "iterations": iter,
                    "tokens_in": tokens_in,
                    "tokens_out": tokens_out,
                }),
                None,
            )
            .await?;
        Ok(TurnOutcome {
            final_text,
            iterations: iter,
            tokens_in,
            tokens_out,
        })
    }

    async fn audit_halt(&self, session_id: SessionId, reason: &str) -> ArgoResult<()> {
        self.deps
            .audit
            .write(
                Some(session_id),
                AuditKind::Halt,
                serde_json::json!({"reason": reason}),
                None,
            )
            .await
    }

    async fn dispatch_one(
        &self,
        session_id: SessionId,
        call: &argo_types::ToolCallRequest,
        opts: &RunTurnOptions,
    ) -> ArgoResult<Message> {
        self.deps
            .audit
            .write(
                Some(session_id),
                AuditKind::ToolCall,
                serde_json::json!({
                    "tool": call.name,
                    "call_id": call.id.to_string(),
                    "provider_call_id": call.provider_call_id,
                }),
                None,
            )
            .await?;
        let mut ctx =
            ToolContext::new(opts.principal.clone(), opts.workspace_root.clone());
        ctx.call_id = call.id;
        let result = self
            .deps
            .registry
            .dispatch(
                &call.name,
                call.arguments.clone(),
                &ctx,
                &self.deps.capability_engine,
            )
            .await?;
        self.deps
            .audit
            .write(
                Some(session_id),
                AuditKind::ToolResult,
                serde_json::json!({
                    "tool": call.name,
                    "call_id": call.id.to_string(),
                    "is_error": result.is_error,
                    "duration_ms": result.duration_ms,
                }),
                None,
            )
            .await?;
        Ok(build_tool_message(session_id, call.id, result))
    }

    async fn persist_message(&self, session_id: SessionId, m: &Message) -> ArgoResult<()> {
        let role_str = match m.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let _ = role_str; // role mapping done inside append_message
        let body = serde_json::to_string(m)
            .map_err(|e| ArgoError::Schema(format!("message serialize: {e}")))?;
        self.deps
            .store
            .append_message(session_id, m.role, &body)
            .await?;
        Ok(())
    }
}

fn build_user_message(session_id: SessionId, text: &str) -> Message {
    Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: text.to_string(),
        }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

fn build_steer_message(session_id: SessionId, steers: &[String]) -> Message {
    let body = if steers.len() == 1 {
        format!("[steer] {}", steers[0])
    } else {
        let mut s = String::from("[steer]");
        for line in steers {
            s.push('\n');
            s.push_str(line);
        }
        s
    };
    build_user_message(session_id, &body)
}

fn build_assistant_message(
    session_id: SessionId,
    resp: &argo_provider::ProviderResponse,
) -> Message {
    let mut content = Vec::new();
    if let Some(text) = resp.text.clone() {
        content.push(ContentBlock::Text { text });
    }
    Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::Assistant,
        content,
        tool_calls: resp.tool_calls.clone(),
        tool_call_id: None,
        reasoning: resp.reasoning.clone(),
        created_at: Utc::now(),
        provider_meta: resp.provider_meta.clone(),
    }
}

fn build_tool_message(
    session_id: SessionId,
    call_id: ToolCallId,
    result: argo_types::ToolCallResult,
) -> Message {
    Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::Tool,
        content: result.content.clone(),
        tool_calls: vec![],
        tool_call_id: Some(call_id),
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::json!({
            "is_error": result.is_error,
            "duration_ms": result.duration_ms,
        }),
    }
}

fn build_provider_request(opts: &RunTurnOptions, history: &[Message]) -> ProviderRequest {
    ProviderRequest {
        model: opts.model.clone(),
        messages: history.to_vec(),
        tools: vec![], // tool schemas wired via registry in Phase 2
        system: opts.system.clone(),
        sampling: opts.sampling.clone(),
    }
}

fn extract_text(m: &Message) -> String {
    m.content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

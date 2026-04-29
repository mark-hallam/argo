//! Provider-transport trait and wire types.
//!
//! Every transport is constructed from a `ProviderManifest`. The trait is the
//! single seam between the agent loop (§6) and the wire — concrete wire
//! formats (OpenAI / Responses / Anthropic / Bedrock) live behind it.
//!
//! The transport returns *wire-level* data only (text, tool calls, token
//! counts). Assembling a `Message` (with session id, timestamps, ids) is the
//! agent loop's job — that keeps argo-provider free of chrono / store deps.
use argo_types::{ArgoResult, CapabilitySet, Message, ToolCallRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One turn's worth of input: messages + tools + sampling knobs.
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    /// Provider-side model id (already resolved — no `"latest"` here).
    pub model: String,
    /// Conversation prefix.
    pub messages: Vec<Message>,
    /// Tool schemas for this turn (zero or more).
    pub tools: Vec<ToolSchema>,
    /// Optional system prompt (some wire formats split this from messages).
    pub system: Option<String>,
    /// Sampling knobs.
    pub sampling: Sampling,
}

/// Sampling knobs.
#[derive(Debug, Clone, Default)]
pub struct Sampling {
    /// Temperature.
    pub temperature: Option<f32>,
    /// Top-p.
    pub top_p: Option<f32>,
    /// Output token cap.
    pub max_tokens: Option<u32>,
    /// Stop sequences.
    pub stop: Vec<String>,
}

/// One tool's wire schema, ready to ship to the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name.
    pub name: String,
    /// One-line description (visible to the model).
    pub description: String,
    /// JSON Schema for the tool's `arguments` object.
    pub parameters: serde_json::Value,
}

/// Wire-level response: text + tool calls + token counts. The agent loop
/// upgrades this into an `argo_types::Message` for persistence.
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// Assistant text, if any.
    pub text: Option<String>,
    /// Reasoning content (Anthropic `thinking`, OpenAI `reasoning`), if any.
    pub reasoning: Option<String>,
    /// Tool calls extracted by the configured `ToolCallFormat` extractor.
    pub tool_calls: Vec<ToolCallRequest>,
    /// Why generation stopped.
    pub stop_reason: StopReason,
    /// Provider-reported input tokens.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens.
    pub tokens_out: Option<u32>,
    /// Opaque provider metadata (id, model echo, fingerprint, raw body, …).
    pub provider_meta: serde_json::Value,
}

/// Why the provider stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Natural end of turn.
    EndTurn,
    /// Stopped to invoke a tool.
    ToolUse,
    /// Hit `max_tokens`.
    MaxTokens,
    /// Filtered by the provider's safety system.
    ContentFilter,
    /// Anything else.
    Other(String),
}

/// The transport seam.
///
/// ```
/// # use argo_provider::Transport;
/// fn assert_object_safe(_t: &dyn Transport) {}
/// ```
#[async_trait]
pub trait Transport: Send + Sync {
    /// Stable id (matches `manifest.provider.id`).
    fn id(&self) -> &str;

    /// Capabilities the transport requires when called — every public-facing
    /// call surface declares its needs (Invariant 3, see CLAUDE.md).
    fn capabilities(&self) -> &CapabilitySet;

    /// One non-streaming request → response round-trip.
    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse>;
}

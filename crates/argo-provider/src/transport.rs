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
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::circuit::{is_breaker_failure, Breaker};

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

/// One delta from a streaming provider call.
///
/// The agent loop accumulates these into a final [`ProviderResponse`]
/// equivalent, but consumers (TUI, gateway) can also tail them live to drive
/// the NFR §1.3 first-token latency budget.
///
/// Spec: issue #3 / F-1.
///
/// ```
/// use argo_provider::{ProviderResponseDelta, StopReason};
/// let d = ProviderResponseDelta::TextChunk("hello".into());
/// match d {
///     ProviderResponseDelta::TextChunk(s) => assert_eq!(s, "hello"),
///     _ => unreachable!(),
/// }
/// // FinishReason and TokenUsage close the stream:
/// let _ = ProviderResponseDelta::FinishReason(StopReason::EndTurn);
/// let _ = ProviderResponseDelta::TokenUsage { tokens_in: 10, tokens_out: 3 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderResponseDelta {
    /// One chunk of assistant text.
    TextChunk(String),
    /// One chunk of reasoning content (e.g. Anthropic `thinking`).
    ReasoningChunk(String),
    /// One incremental piece of a tool-call.
    ///
    /// `args_partial` is the raw JSON fragment as the provider emits it; the
    /// agent loop concatenates fragments per `id` and parses once finished.
    ToolCallDelta {
        /// Provider-side tool-call id (stable across deltas in one call).
        id: String,
        /// Tool name (set on the first delta; may be empty on continuations).
        name: String,
        /// Raw argument JSON fragment.
        args_partial: String,
    },
    /// The provider closed the turn with this stop reason.
    FinishReason(StopReason),
    /// Final usage report. Many providers send this in the last event;
    /// some inline it in headers — transports normalise to this delta.
    TokenUsage {
        /// Provider-reported input tokens.
        tokens_in: u32,
        /// Provider-reported output tokens.
        tokens_out: u32,
    },
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

    /// Streaming variant of [`Transport::complete`] (issue #3 / F-1).
    ///
    /// Returns a stream that yields [`ProviderResponseDelta`] values until
    /// the provider closes the turn. The stream terminates after a
    /// `FinishReason` (and optionally a `TokenUsage`) is emitted.
    ///
    /// Errors during streaming surface as `Err` items; the stream may yield
    /// further items after one (some providers continue after a recoverable
    /// blip), so callers should keep polling until `None`.
    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>>;
}

/// Build a `reqwest::Client` with this crate's pool-sizing knobs
/// stamped on plus the manifest-driven connect/read timeouts.
///
/// Centralised here so each manifest-backed transport
/// (`anthropic`, `openai_compat`, `openai_responses`) has a single
/// hook to read [`crate::pool_config::ProviderPoolConfig::from_env`]
/// without reimplementing the builder chain.
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::transport::build_http_client;
/// let _client = build_http_client(5_000, 60_000)?;
/// # Ok(()) }
/// ```
pub fn build_http_client(
    connect_timeout_ms: u64,
    read_timeout_ms: u64,
) -> ArgoResult<reqwest::Client> {
    let cfg = crate::pool_config::ProviderPoolConfig::from_env();
    cfg.apply_pool(reqwest::Client::builder())
        .connect_timeout(std::time::Duration::from_millis(connect_timeout_ms))
        .timeout(std::time::Duration::from_millis(read_timeout_ms))
        .build()
        .map_err(|e| argo_types::ArgoError::Provider(format!("reqwest client build: {e}")))
}

/// [`Transport`] adapter that gates calls through a per-provider circuit
/// [`Breaker`].
///
/// Wrap any concrete transport with [`BreakerTransport::new`]; calls
/// short-circuit when the breaker is Open or HalfOpen-with-probe-in-flight.
/// Errors are classified by [`is_breaker_failure`] (HTTP 5xx + network-layer
/// failures count; HTTP 4xx / 429 are ignored).
///
/// ```
/// # async fn example<T: argo_provider::Transport>(inner: T) {
/// use argo_provider::{Breaker, BreakerConfig, transport::BreakerTransport};
/// let breaker = Breaker::new(BreakerConfig::default());
/// let _wrapped = BreakerTransport::new(inner, breaker);
/// # }
/// ```
pub struct BreakerTransport<T: Transport> {
    inner: T,
    breaker: Breaker,
}

impl<T: Transport> BreakerTransport<T> {
    /// Wrap an inner transport with a breaker.
    pub fn new(inner: T, breaker: Breaker) -> Self {
        Self { inner, breaker }
    }

    /// Borrowed inner transport.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Borrowed breaker.
    pub fn breaker(&self) -> &Breaker {
        &self.breaker
    }
}

#[async_trait]
impl<T: Transport> Transport for BreakerTransport<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.inner.capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let permit = self.breaker.try_acquire(self.inner.id())?;
        match self.inner.complete(req).await {
            Ok(resp) => {
                permit.record_success();
                Ok(resp)
            }
            Err(e) => {
                if is_breaker_failure(&e) {
                    permit.record_failure();
                } else {
                    permit.record_ignored();
                }
                Err(e)
            }
        }
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        // The breaker gates stream *establishment* — the thundering-herd
        // failure mode this brief targets. Mid-stream deltas ride the
        // existing long-lived connection and are not metered here; a
        // mid-stream Err surfaces to the caller unchanged.
        let permit = self.breaker.try_acquire(self.inner.id())?;
        match self.inner.complete_stream(req).await {
            Ok(stream) => {
                permit.record_success();
                Ok(stream)
            }
            Err(e) => {
                if is_breaker_failure(&e) {
                    permit.record_failure();
                } else {
                    permit.record_ignored();
                }
                Err(e)
            }
        }
    }
}

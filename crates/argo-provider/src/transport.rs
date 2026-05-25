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
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::manifest::{DiscoverySpec, ProviderManifest};

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
/// stamped on plus the manifest-driven connect/per-read timeouts.
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
        .read_timeout(std::time::Duration::from_millis(read_timeout_ms))
        .build()
        .map_err(|e| argo_types::ArgoError::Provider(format!("reqwest client build: {e}")))
}

/// Resolve `"latest"` through the transport's runtime cache when present.
#[must_use]
pub fn resolve_latest_provider_request(
    cache: Option<&LatestModelCache>,
    req: &ProviderRequest,
) -> ProviderRequest {
    cache
        .map(|cache| cache.resolve_provider_request(req))
        .unwrap_or_else(|| req.clone())
}

#[derive(Debug, Clone, Default)]
struct LatestModelState {
    model: Option<String>,
    refreshed_at: Option<Instant>,
}

/// Cache for manifest-driven `model = "latest"` resolution.
///
/// Daemons call [`LatestModelCache::refresh_if_due_with`] with their
/// `/v1/models` fetcher on startup and before provider requests. Calls made
/// before the manifest's `refresh_interval_hours` expires reuse the cached
/// value; calls after it expires fetch ids again and apply
/// [`crate::resolve_latest`].
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{DiscoverySpec, PreferRule};
/// use argo_provider::transport::LatestModelCache;
/// use std::time::Instant;
///
/// let spec = DiscoverySpec {
///     endpoint: "{base_url}/v1/models".into(),
///     include_pattern: "^gpt-".into(),
///     exclude_pattern: None,
///     prefer: PreferRule::LexicographicMax,
///     refresh_interval_hours: 24,
/// };
/// let cache = LatestModelCache::new(spec, "gpt-4o-mini");
/// cache.refresh_if_due_with(Instant::now(), || Ok(vec!["gpt-4o", "gpt-4.1"]))?;
/// assert_eq!(cache.resolve_request_model("latest"), "gpt-4o");
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct LatestModelCache {
    spec: DiscoverySpec,
    fallback_model: String,
    refresh_interval: Duration,
    state: Arc<RwLock<LatestModelState>>,
}

impl LatestModelCache {
    /// Build a cache from a discovery spec plus the manifest's static
    /// fallback model. A zero-hour refresh interval is treated as one hour so
    /// an invalid config cannot create a tight fetch loop.
    #[must_use]
    pub fn new(spec: DiscoverySpec, fallback_model: impl Into<String>) -> Self {
        let hours = spec.refresh_interval_hours.max(1);
        Self {
            spec,
            fallback_model: fallback_model.into(),
            refresh_interval: Duration::from_secs(u64::from(hours) * 60 * 60),
            state: Arc::new(RwLock::new(LatestModelState::default())),
        }
    }

    /// Build a cache from a manifest's optional `[discovery]` block.
    ///
    /// Returns `None` when the manifest has no discovery config; callers
    /// should keep using [`ProviderManifest::resolve_model_id`] in that case.
    #[must_use]
    pub fn from_manifest(manifest: &ProviderManifest) -> Option<Self> {
        let spec = manifest.discovery.clone()?;
        let fallback = manifest
            .models
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "latest".to_string());
        Some(Self::new(spec, fallback))
    }

    /// Effective refresh interval after applying the one-hour floor.
    #[must_use]
    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }

    /// Last successfully resolved model id, if any.
    #[must_use]
    pub fn cached_model(&self) -> Option<String> {
        self.state.read().model.clone()
    }

    /// Resolve a request model, replacing `"latest"` with the cached value
    /// when available and otherwise with the static manifest fallback.
    #[must_use]
    pub fn resolve_request_model(&self, requested: &str) -> String {
        if requested != "latest" {
            return requested.to_string();
        }
        self.cached_model()
            .unwrap_or_else(|| self.fallback_model.clone())
    }

    /// Clone a provider request with `"latest"` replaced by the current
    /// cached model id. Other request fields are preserved.
    #[must_use]
    pub fn resolve_provider_request(&self, req: &ProviderRequest) -> ProviderRequest {
        let mut resolved = req.clone();
        resolved.model = self.resolve_request_model(&req.model);
        resolved
    }

    /// Refresh the cache from a known model-id list immediately.
    ///
    /// This is useful for startup pre-warming and tests that already have a
    /// recorded `/v1/models` fixture.
    ///
    /// # Errors
    /// Returns the same errors as [`crate::resolve_latest`] when filtering
    /// or selection fails.
    pub fn refresh_with_ids<S: AsRef<str>>(
        &self,
        now: Instant,
        model_ids: &[S],
    ) -> ArgoResult<String> {
        let model = crate::discovery::resolve_latest(&self.spec, model_ids)?;
        let mut state = self.state.write();
        state.model = Some(model.clone());
        state.refreshed_at = Some(now);
        Ok(model)
    }

    /// Fetch and refresh only when the cache is empty or expired.
    ///
    /// The fetcher is intentionally injected so the daemon can use its own
    /// authenticated HTTP client while tests can use an in-memory mock.
    ///
    /// # Errors
    /// Propagates fetcher errors and [`crate::resolve_latest`] failures.
    pub fn refresh_if_due_with<S, F>(
        &self,
        now: Instant,
        fetch_model_ids: F,
    ) -> ArgoResult<Option<String>>
    where
        S: AsRef<str>,
        F: FnOnce() -> ArgoResult<Vec<S>>,
    {
        if !self.needs_refresh(now) {
            return Ok(None);
        }
        let ids = fetch_model_ids()?;
        self.refresh_with_ids(now, &ids).map(Some)
    }

    fn needs_refresh(&self, now: Instant) -> bool {
        let state = self.state.read();
        if state.model.is_none() {
            return true;
        }
        match state.refreshed_at {
            Some(refreshed_at) => {
                now.saturating_duration_since(refreshed_at) >= self.refresh_interval
            }
            None => true,
        }
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;

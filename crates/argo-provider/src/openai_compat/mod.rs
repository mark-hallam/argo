//! Manifest-driven `Transport` for the `openai-compatible` family.
//!
//! Speaks the OpenAI Chat Completions wire (the de-facto common subset:
//! OpenAI proper, OpenRouter, vLLM, Together, Fireworks, etc).
//!
//! Spec: `docs/plan/argo-build-plan.md` §8 / Phase 2 plan.
mod stream;
mod wire;

#[cfg(test)]
mod tests;

use crate::breaker_observability::ObservedBreakerTransport;
use crate::circuit::{Breaker, BreakerConfig};
use crate::extractors::extract_tool_calls;
use crate::manifest::{ProviderFamily, ProviderManifest};
use crate::sse::SseBuffer;
use crate::transport::{
    build_http_client, ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport,
};
use argo_types::{ArgoError, ArgoResult, CapabilitySet, RequestId};
use async_trait::async_trait;
use futures::stream::{self as fstream, BoxStream, StreamExt};
use reqwest::{header::HeaderName, Client};
use std::time::Duration;
pub use stream::event_to_deltas;
use wire::{assemble_response, build_request_body, OpenAiResponse};

/// Cap for HTTP error body text we attach to `ArgoError::Provider` (issue #5
/// / F-7). Anything larger gets a `... [truncated N bytes]` suffix.
pub(super) const ERROR_BODY_CAP: usize = 4096;

/// Manifest-driven OpenAI-compatible transport.
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{OpenAiCompatTransport, ProviderManifest, Transport};
/// let toml = include_str!("../../../../manifests/providers/openai-compatible.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let transport = OpenAiCompatTransport::from_manifest(manifest, "sk-test")?;
/// let _: &str = transport.id();
/// # Ok(()) }
/// ```
pub struct OpenAiCompatTransport {
    manifest: ProviderManifest,
    api_key: String,
    client: Client,
    capabilities: CapabilitySet,
}

impl std::fmt::Debug for OpenAiCompatTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatTransport")
            .field("provider_id", &self.manifest.provider.id)
            .field("base_url", &self.manifest.provider.base_url)
            .field("api_key", &"<redacted>")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl OpenAiCompatTransport {
    /// Build a transport from a parsed manifest + an already-resolved API
    /// key. The caller is responsible for reading the env var declared in
    /// `manifest.auth.api_key_env` (Capability::EnvRead enforced upstream).
    pub fn from_manifest(manifest: ProviderManifest, api_key: &str) -> ArgoResult<Self> {
        if manifest.provider.family != ProviderFamily::OpenaiCompatible {
            return Err(ArgoError::Provider(format!(
                "OpenAiCompatTransport requires family=openai-compatible; got {:?}",
                manifest.provider.family
            )));
        }
        if api_key.is_empty() {
            return Err(ArgoError::Provider(format!(
                "missing API key for env {}",
                manifest.auth.api_key_env
            )));
        }
        let client = build_http_client(
            manifest.transport.connect_timeout_ms,
            manifest.transport.read_timeout_ms,
        )?;
        let capabilities = manifest.capabilities.to_set(&manifest.auth.api_key_env);
        Ok(Self {
            manifest,
            api_key: api_key.to_string(),
            client,
            capabilities,
        })
    }

    /// Build from a parsed manifest + a resolved API key, then wrap
    /// the transport with a per-provider [`ObservedBreakerTransport`]
    /// (HTTP-5xx + network-layer failures trip the breaker, every
    /// state transition emits one `tracing::warn!`).
    ///
    /// Each call returns its own [`Breaker`]; never share across
    /// providers.
    pub fn from_manifest_with_breaker(
        manifest: ProviderManifest,
        api_key: &str,
        breaker_config: BreakerConfig,
    ) -> ArgoResult<ObservedBreakerTransport<Self>> {
        let inner = Self::from_manifest(manifest, api_key)?;
        Ok(ObservedBreakerTransport::new(
            inner,
            Breaker::new(breaker_config),
        ))
    }

    /// Borrowed manifest — useful for callers that need provider-level config
    /// (cost router, model resolution, …) without re-parsing.
    #[must_use]
    pub fn manifest(&self) -> &ProviderManifest {
        &self.manifest
    }

    pub(crate) fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.manifest.provider.base_url, self.manifest.transport.chat_path
        )
    }

    pub(crate) fn auth_header_value(&self) -> String {
        self.manifest
            .auth
            .header_format
            .replace("{key}", &self.api_key)
    }
}

#[async_trait]
impl Transport for OpenAiCompatTransport {
    fn id(&self) -> &str {
        &self.manifest.provider.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let body = build_request_body(&self.manifest, req)?;
        let raw = retry_post_json(
            &self.client,
            &self.endpoint(),
            &self.manifest.auth.header_name,
            &self.auth_header_value(),
            &body,
            self.manifest.transport.max_retries,
            self.manifest.transport.initial_backoff_ms,
            self.manifest.transport.max_backoff_ms,
        )
        .await?;
        let parsed: OpenAiResponse = serde_json::from_value(raw.clone())
            .map_err(|e| ArgoError::Provider(format!("openai response parse: {e}")))?;
        let tool_calls = extract_tool_calls(self.manifest.tool_calls.format, &raw)?;
        Ok(assemble_response(parsed, tool_calls, raw))
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        let mut body = build_request_body(&self.manifest, req)?;
        body["stream"] = serde_json::Value::Bool(true);
        let resp = self
            .client
            .post(self.endpoint())
            .header(
                self.manifest.auth.header_name.as_str(),
                self.auth_header_value(),
            )
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .header(IDEMPOTENCY_KEY, RequestId::new().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("reqwest send (stream): {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = truncate_for_error(resp.text().await.unwrap_or_default());
            return Err(ArgoError::Provider(format!(
                "openai HTTP {status}: {body_text}"
            )));
        }
        Ok(stream_response_body(resp).boxed())
    }
}

/// HTTP header name for idempotent retries (issue #5 / F-8). One key per
/// logical request — same value across retry attempts so the provider can
/// dedupe.
pub(super) const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

fn stream_response_body(
    resp: reqwest::Response,
) -> impl futures::Stream<Item = ArgoResult<ProviderResponseDelta>> + Send + 'static {
    fstream::unfold(StreamState::new(resp), step_stream)
}

struct StreamState {
    resp: reqwest::Response,
    buf: SseBuffer,
    pending: std::collections::VecDeque<ArgoResult<ProviderResponseDelta>>,
    done: bool,
}

impl StreamState {
    fn new(resp: reqwest::Response) -> Self {
        Self {
            resp,
            buf: SseBuffer::new(),
            pending: std::collections::VecDeque::new(),
            done: false,
        }
    }
}

async fn step_stream(
    mut s: StreamState,
) -> Option<(ArgoResult<ProviderResponseDelta>, StreamState)> {
    if let Some(item) = s.pending.pop_front() {
        return Some((item, s));
    }
    if s.done {
        return None;
    }
    loop {
        if let Some(event) = s.buf.take_event() {
            queue_event_deltas(&mut s, &event);
            if let Some(item) = s.pending.pop_front() {
                return Some((item, s));
            }
            continue;
        }
        match s.resp.chunk().await {
            Ok(Some(bytes)) => {
                let chunk = String::from_utf8_lossy(&bytes);
                s.buf.extend(&chunk);
            }
            Ok(None) => {
                s.done = true;
                return None;
            }
            Err(e) => {
                s.done = true;
                return Some((
                    Err(ArgoError::Provider(format!("openai SSE chunk: {e}"))),
                    s,
                ));
            }
        }
    }
}

fn queue_event_deltas(s: &mut StreamState, event: &str) {
    if event.is_empty() {
        return;
    }
    if event == stream::DONE_SENTINEL {
        s.done = true;
        return;
    }
    match event_to_deltas(event) {
        Ok(deltas) => {
            for d in deltas {
                s.pending.push_back(Ok(d));
            }
        }
        Err(e) => s.pending.push_back(Err(e)),
    }
}

#[allow(clippy::too_many_arguments)]
async fn retry_post_json(
    client: &Client,
    url: &str,
    header_name: &str,
    header_value: &str,
    body: &serde_json::Value,
    max_retries: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
) -> ArgoResult<serde_json::Value> {
    let idem = RequestId::new().to_string();
    let mut attempt: u32 = 0;
    loop {
        let resp = client
            .post(url)
            .header(header_name, header_value)
            .header("content-type", "application/json")
            .header(IDEMPOTENCY_KEY, idem.as_str())
            .json(body)
            .send()
            .await;
        let outcome = classify_attempt(resp).await;
        match outcome {
            AttemptOutcome::Ok(value) => return Ok(value),
            AttemptOutcome::Fatal(err) => return Err(err),
            AttemptOutcome::Retry(err) => {
                if attempt >= max_retries {
                    return Err(err);
                }
                let delay = compute_backoff_delay(initial_backoff_ms, max_backoff_ms, attempt);
                tracing::warn!(
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "openai-compat retry"
                );
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

/// Compute the backoff delay for a given attempt with full jitter.
///
/// Algorithm from issue #3 / F-6: bounded exponential growth, then a uniform
/// random pick in `[1, delay]`. We use system-time nanos as the entropy
/// source to keep the dep allowlist tight (no `rand`).
fn compute_backoff_delay(initial_ms: u64, max_ms: u64, attempt: u32) -> Duration {
    let bounded_attempt = attempt.min(31);
    let cap = initial_ms
        .saturating_mul(1u64 << bounded_attempt)
        .min(max_ms.max(initial_ms))
        .max(1);
    let jittered = (jitter_seed() % cap) + 1;
    Duration::from_millis(jittered)
}

fn jitter_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1)
        .wrapping_add(1)
}

enum AttemptOutcome {
    Ok(serde_json::Value),
    Retry(ArgoError),
    Fatal(ArgoError),
}

async fn classify_attempt(resp: Result<reqwest::Response, reqwest::Error>) -> AttemptOutcome {
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            let retryable = e.is_timeout() || e.is_connect();
            let err = ArgoError::Provider(format!("reqwest send: {e}"));
            return if retryable {
                AttemptOutcome::Retry(err)
            } else {
                AttemptOutcome::Fatal(err)
            };
        }
    };
    let status = resp.status();
    if status.is_success() {
        return match resp.json::<serde_json::Value>().await {
            Ok(v) => AttemptOutcome::Ok(v),
            Err(e) => {
                AttemptOutcome::Fatal(ArgoError::Provider(format!("openai response decode: {e}")))
            }
        };
    }
    let body_text = truncate_for_error(resp.text().await.unwrap_or_default());
    let err = ArgoError::Provider(format!("openai HTTP {status}: {body_text}"));
    if status.as_u16() == 429 || status.is_server_error() {
        AttemptOutcome::Retry(err)
    } else {
        AttemptOutcome::Fatal(err)
    }
}

/// Truncate an HTTP error body to [`ERROR_BODY_CAP`] bytes (issue #5 /
/// F-7). UTF-8-safe: we step backwards to a char boundary.
pub(super) fn truncate_for_error(s: String) -> String {
    if s.len() <= ERROR_BODY_CAP {
        return s;
    }
    let mut end = ERROR_BODY_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let truncated_bytes = s.len() - end;
    format!("{}... [truncated {truncated_bytes} bytes]", &s[..end])
}

//! Manifest-driven `Transport` for the Anthropic Messages API.
//!
//! Auth: `x-api-key: <key>` + the required `anthropic-version: 2023-06-01`
//! header. Streaming uses the typed event sequence from
//! <https://docs.anthropic.com/en/api/messages-streaming>.
mod stream;
mod wire;

#[cfg(test)]
mod tests;

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
use stream::StreamState;
use wire::{assemble_response, build_request_body, AnthropicResponse};

/// Anthropic API version pinned per the docs.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// HTTP header carrying the API version.
const ANTHROPIC_VERSION_HEADER: HeaderName = HeaderName::from_static("anthropic-version");

/// HTTP header carrying the idempotency key.
const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

/// Cap for the body text we attach to `ArgoError::Provider` on non-2xx.
const ERROR_BODY_CAP: usize = 4096;

/// Manifest-driven Anthropic transport.
///
/// ```
/// # fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{AnthropicTransport, ProviderManifest, Transport};
/// let toml = include_str!("../../../../manifests/providers/anthropic.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let transport = AnthropicTransport::from_manifest(manifest, "sk-ant-test")?;
/// let _: &str = transport.id();
/// # Ok(()) }
/// ```
pub struct AnthropicTransport {
    manifest: ProviderManifest,
    api_key: String,
    client: Client,
    capabilities: CapabilitySet,
}

impl std::fmt::Debug for AnthropicTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicTransport")
            .field("provider_id", &self.manifest.provider.id)
            .field("base_url", &self.manifest.provider.base_url)
            .field("api_key", &"<redacted>")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl AnthropicTransport {
    /// Build from a parsed manifest + a resolved API key.
    pub fn from_manifest(manifest: ProviderManifest, api_key: &str) -> ArgoResult<Self> {
        if manifest.provider.family != ProviderFamily::Anthropic {
            return Err(ArgoError::Provider(format!(
                "AnthropicTransport requires family=anthropic; got {:?}",
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

    /// Borrowed manifest.
    #[must_use]
    pub fn manifest(&self) -> &ProviderManifest {
        &self.manifest
    }

    fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.manifest.provider.base_url, self.manifest.transport.chat_path
        )
    }

    fn auth_header_value(&self) -> String {
        self.manifest
            .auth
            .header_format
            .replace("{key}", &self.api_key)
    }
}

#[async_trait]
impl Transport for AnthropicTransport {
    fn id(&self) -> &str {
        &self.manifest.provider.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let body = build_request_body(&self.manifest, req)?;
        let raw = retry_post_json(self, &body).await?;
        let parsed: AnthropicResponse = serde_json::from_value(raw.clone())
            .map_err(|e| ArgoError::Provider(format!("anthropic response parse: {e}")))?;
        let mut resp = assemble_response(
            parsed,
            raw.clone(),
            &self.manifest.provider.id,
            self.manifest.resolve_model_id(&req.model),
        );
        // Prefer the manifest-declared extractor if it agrees; otherwise the
        // wire-typed assembler already populated tool_calls. extract_tool_calls
        // is the cross-family hook for cost-router / replay parity.
        if matches!(
            self.manifest.tool_calls.format,
            crate::manifest::ToolCallFormat::AnthropicToolUse
        ) {
            let extracted = extract_tool_calls(self.manifest.tool_calls.format, &raw)?;
            if !extracted.is_empty() {
                resp.tool_calls = extracted;
            }
        }
        Ok(resp)
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
            .header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .header(IDEMPOTENCY_KEY, RequestId::new().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("reqwest send (stream): {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = truncate_for_error(resp.text().await.unwrap_or_default());
            return Err(ArgoError::Provider(format!(
                "anthropic HTTP {status}: {text}"
            )));
        }
        Ok(stream_response_body(resp).boxed())
    }
}

async fn retry_post_json(
    t: &AnthropicTransport,
    body: &serde_json::Value,
) -> ArgoResult<serde_json::Value> {
    let idem = RequestId::new().to_string();
    let mut attempt: u32 = 0;
    loop {
        let resp = t
            .client
            .post(t.endpoint())
            .header(t.manifest.auth.header_name.as_str(), t.auth_header_value())
            .header(ANTHROPIC_VERSION_HEADER, ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .header(IDEMPOTENCY_KEY, idem.as_str())
            .json(body)
            .send()
            .await;
        let outcome = classify_attempt(resp).await;
        match outcome {
            AttemptOutcome::Ok(v) => return Ok(v),
            AttemptOutcome::Fatal(e) => return Err(e),
            AttemptOutcome::Retry(e) => {
                if attempt >= t.manifest.transport.max_retries {
                    return Err(e);
                }
                let delay = compute_backoff_delay(
                    t.manifest.transport.initial_backoff_ms,
                    t.manifest.transport.max_backoff_ms,
                    attempt,
                );
                tracing::warn!(attempt, delay_ms = delay.as_millis() as u64, error = %e, "anthropic retry");
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
            }
        }
    }
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
            Err(e) => AttemptOutcome::Fatal(ArgoError::Provider(format!(
                "anthropic response decode: {e}"
            ))),
        };
    }
    let body_text = truncate_for_error(resp.text().await.unwrap_or_default());
    let err = ArgoError::Provider(format!("anthropic HTTP {status}: {body_text}"));
    if status.as_u16() == 429 || status.is_server_error() {
        AttemptOutcome::Retry(err)
    } else {
        AttemptOutcome::Fatal(err)
    }
}

fn compute_backoff_delay(initial_ms: u64, max_ms: u64, attempt: u32) -> Duration {
    let bounded_attempt = attempt.min(31);
    let cap = initial_ms
        .saturating_mul(1u64 << bounded_attempt)
        .min(max_ms.max(initial_ms))
        .max(1);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1)
        .wrapping_add(1);
    Duration::from_millis((nanos % cap) + 1)
}

fn truncate_for_error(s: String) -> String {
    if s.len() <= ERROR_BODY_CAP {
        return s;
    }
    let mut end = ERROR_BODY_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = s.len() - end;
    format!("{}... [truncated {truncated} bytes]", &s[..end])
}

fn stream_response_body(
    resp: reqwest::Response,
) -> impl futures::Stream<Item = ArgoResult<ProviderResponseDelta>> + Send + 'static {
    fstream::unfold(StreamRunner::new(resp), step_stream)
}

struct StreamRunner {
    resp: reqwest::Response,
    buf: SseBuffer,
    state: StreamState,
    pending: std::collections::VecDeque<ArgoResult<ProviderResponseDelta>>,
    done: bool,
}

impl StreamRunner {
    fn new(resp: reqwest::Response) -> Self {
        Self {
            resp,
            buf: SseBuffer::new(),
            state: StreamState::new(),
            pending: std::collections::VecDeque::new(),
            done: false,
        }
    }
}

async fn step_stream(
    mut s: StreamRunner,
) -> Option<(ArgoResult<ProviderResponseDelta>, StreamRunner)> {
    if let Some(item) = s.pending.pop_front() {
        return Some((item, s));
    }
    if s.done || s.state.done {
        return None;
    }
    loop {
        if let Some(event) = s.buf.take_event() {
            match stream::event_to_deltas(&mut s.state, &event) {
                Ok(deltas) => {
                    for d in deltas {
                        s.pending.push_back(Ok(d));
                    }
                }
                Err(e) => s.pending.push_back(Err(e)),
            }
            if let Some(item) = s.pending.pop_front() {
                return Some((item, s));
            }
            if s.state.done {
                return None;
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
                    Err(ArgoError::Provider(format!("anthropic SSE chunk: {e}"))),
                    s,
                ));
            }
        }
    }
}

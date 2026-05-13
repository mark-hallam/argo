//! Streaming-aware retry wrapper (`mission-streaming-aware-retry`,
//! MISSION_BACKLOG #249).
//!
//! When a streaming LLM call fails mid-stream — a network blip, a
//! transient 5xx, a 429 — naïve retry restarts from the start and
//! discards every chunk received so far. This module wraps an
//! attempt factory in a state machine that buffers received chunks
//! and, on a transient failure, retries with the buffered assistant
//! text pre-filled as an `Assistant` message at the tail of the
//! conversation. The provider treats it as a continuation:
//! Anthropic's [assistant prefill] and OpenAI's
//! "append-assistant-then-continue" pattern both pick up cleanly
//! mid-sentence.
//!
//! [assistant prefill]: https://docs.anthropic.com/en/api/messages#assistant-prefill
//!
//! Scope (v1): buffered content is **text only**
//! ([`ProviderResponseDelta::TextChunk`]). `ToolCallDelta` fragments
//! and `ReasoningChunk` deltas are NOT preserved across retries —
//! per the brief, content-deduplication is out of scope and the
//! model is expected to pick up cleanly from the prefix.
//!
//! Retryable errors: HTTP 5xx, HTTP 429, and transport-level network
//! / SSE-chunk failures. Non-retryable: HTTP 4xx (other than 429),
//! schema errors, capability denials, invariants. Non-retryable
//! errors surface unchanged.
//!
//! Every retry emits a structured `tracing::warn!` with
//! `kind = "runtime.streaming_retry"`, attempt index, computed
//! delay, the source error, and buffered byte count. Operators tail
//! these via the existing tracing → log-shipping pipeline.
//!
//! [`RetryStreamTransport<T>`] wraps any [`Transport`] and applies
//! the retry policy to [`Transport::complete_stream`] only.
//! Non-streaming `complete` calls fall through unchanged — their
//! retry policy lives in the manifest-driven transport.
//!
//! ```
//! # async fn example<T: argo_provider::Transport>(inner: T) {
//! use argo_provider::streaming_retry::{RetryStreamTransport, RetryConfig};
//! let cfg = RetryConfig::default();
//! let _wrapped = RetryStreamTransport::new(inner, cfg);
//! # }
//! ```
use crate::transport::{ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport};
use argo_types::{
    ArgoError, ArgoResult, CapabilitySet, ContentBlock, Message, MessageId, MessageRole,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::future::BoxFuture;
use futures::stream::{self as fstream, BoxStream, StreamExt};
use std::sync::Arc;
use std::time::Duration;

/// Operator-tunable knobs for the streaming-retry wrapper.
///
/// Mirrors the `[runtime.streaming_retry]` TOML block in
/// `argo-config`; the consuming code converts the config struct into
/// this type via `RetryConfig::from(cfg)`.
///
/// ```
/// use argo_provider::streaming_retry::RetryConfig;
/// let cfg = RetryConfig::default();
/// assert!(cfg.enabled);
/// assert_eq!(cfg.max_attempts, 3);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    /// Master enable. When `false`, the wrapper is a pass-through.
    pub enabled: bool,
    /// Maximum number of retries after the initial attempt. `3` means
    /// up to 4 total tries (initial + 3) before
    /// [`ArgoError::StreamingRetryExhausted`] surfaces.
    pub max_attempts: u32,
    /// Base backoff in milliseconds. Doubled per subsequent retry,
    /// capped internally so misconfigured values can't stall the
    /// stream indefinitely.
    pub backoff_initial_ms: u64,
}

/// Hard ceiling on the per-retry sleep, regardless of
/// `backoff_initial_ms * 2^attempt`. 30s keeps even pathological
/// misconfiguration from holding a stream open indefinitely.
const BACKOFF_MAX_MS: u64 = 30_000;

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            backoff_initial_ms: 1000,
        }
    }
}

/// A boxed factory that opens a fresh streaming attempt for the given
/// request. The wrapper calls this once per attempt — initial plus
/// each retry — so the closure must be re-entrant.
pub type AttemptFactory = Arc<
    dyn Fn(
            ProviderRequest,
        )
            -> BoxFuture<'static, ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>>>
        + Send
        + Sync,
>;

/// Return the retry-wrapped streaming-output for `req`.
///
/// The returned stream forwards chunks from the current attempt live
/// to the consumer (so first-token latency is preserved across the
/// happy path) while internally buffering text for continuation on
/// failure. On a retryable mid-stream error the wrapper sleeps for
/// `backoff_initial_ms * 2^(attempt-1)` (capped at 30s), re-opens
/// the call with the buffered text appended as an `Assistant`
/// message, and continues forwarding deltas from the new stream.
/// Consumers therefore observe one continuous stream: live attempt-1
/// deltas, then live attempt-2 continuation, and so on.
///
/// After `max_attempts` retries the wrapper emits a single final
/// `Err(ArgoError::StreamingRetryExhausted { partial_content })` and
/// terminates the consumer stream.
pub fn retry_streaming(
    req: ProviderRequest,
    config: RetryConfig,
    attempt: AttemptFactory,
) -> BoxStream<'static, ArgoResult<ProviderResponseDelta>> {
    if !config.enabled {
        // Disabled: open one attempt and forward verbatim. If the open
        // itself fails, surface the error as a single Err item.
        let fut = (attempt)(req);
        let s = fstream::once(fut)
            .flat_map(|res| match res {
                Ok(stream) => stream,
                Err(e) => Box::pin(fstream::once(async move { Err(e) }))
                    as BoxStream<'static, ArgoResult<ProviderResponseDelta>>,
            })
            .boxed();
        return s;
    }
    let state = State {
        req,
        config,
        attempt,
        buffered: String::new(),
        current: None,
        retries_used: 0,
        finished: false,
    };
    fstream::unfold(state, step).boxed()
}

struct State {
    req: ProviderRequest,
    config: RetryConfig,
    attempt: AttemptFactory,
    /// Assistant text accumulated across attempts; used to build the
    /// continuation prefix on the next retry and as the
    /// `partial_content` payload on exhaustion.
    buffered: String,
    current: Option<BoxStream<'static, ArgoResult<ProviderResponseDelta>>>,
    /// Number of retries performed so far (not counting the initial
    /// attempt). Bounded by `config.max_attempts`.
    retries_used: u32,
    finished: bool,
}

async fn step(mut state: State) -> Option<(ArgoResult<ProviderResponseDelta>, State)> {
    if state.finished {
        return None;
    }
    loop {
        if state.current.is_none() {
            let req = if state.buffered.is_empty() {
                state.req.clone()
            } else {
                augment_with_prefix(&state.req, &state.buffered)
            };
            match (state.attempt)(req).await {
                Ok(s) => {
                    state.current = Some(s);
                }
                Err(err) => match maybe_schedule_retry(&mut state, &err).await {
                    RetryDecision::Retried => continue,
                    RetryDecision::Terminal => {
                        state.finished = true;
                        return Some((Err(make_terminal_err(err, &mut state.buffered)), state));
                    }
                },
            }
        }

        let s = match state.current.as_mut() {
            Some(s) => s,
            None => unreachable!("opened above"),
        };
        match s.next().await {
            Some(Ok(delta)) => {
                if let ProviderResponseDelta::TextChunk(chunk) = &delta {
                    state.buffered.push_str(chunk);
                }
                return Some((Ok(delta), state));
            }
            Some(Err(err)) => {
                state.current = None;
                match maybe_schedule_retry(&mut state, &err).await {
                    RetryDecision::Retried => continue,
                    RetryDecision::Terminal => {
                        state.finished = true;
                        return Some((Err(make_terminal_err(err, &mut state.buffered)), state));
                    }
                }
            }
            None => {
                state.finished = true;
                return None;
            }
        }
    }
}

enum RetryDecision {
    Retried,
    Terminal,
}

async fn maybe_schedule_retry(state: &mut State, err: &ArgoError) -> RetryDecision {
    if !is_retryable(err) || state.retries_used >= state.config.max_attempts {
        return RetryDecision::Terminal;
    }
    state.retries_used = state.retries_used.saturating_add(1);
    let delay = compute_backoff(state.config.backoff_initial_ms, state.retries_used);
    tracing::warn!(
        kind = "runtime.streaming_retry",
        attempt = state.retries_used,
        max_attempts = state.config.max_attempts,
        delay_ms = delay.as_millis() as u64,
        buffered_bytes = state.buffered.len(),
        error = %err,
        "streaming retry"
    );
    tokio::time::sleep(delay).await;
    RetryDecision::Retried
}

fn make_terminal_err(source: ArgoError, buffered: &mut String) -> ArgoError {
    if is_retryable(&source) {
        ArgoError::StreamingRetryExhausted {
            partial_content: std::mem::take(buffered),
        }
    } else {
        source
    }
}

/// Classify an error as a retryable transient failure. Mirrors the
/// circuit-breaker classifier ([`crate::circuit::is_breaker_failure`])
/// but additionally counts HTTP 429 as retryable — the breaker
/// deliberately ignores 429 (it isn't a server-health signal), but
/// the rate-limit is exactly the case mid-stream retry-with-backoff
/// is designed to ride out.
fn is_retryable(err: &ArgoError) -> bool {
    let msg = match err {
        ArgoError::Provider(m) => m.as_str(),
        _ => return false,
    };
    if let Some(status) = parse_http_status(msg) {
        return status == 429 || (500..600).contains(&status);
    }
    // Transport-level network/SSE failures format as "reqwest send: …"
    // or "<provider> SSE chunk: …" — both are transient.
    msg.starts_with("reqwest send") || msg.contains("SSE chunk")
}

fn parse_http_status(msg: &str) -> Option<u16> {
    let i = msg.find(" HTTP ")?;
    let rest = &msg[i + " HTTP ".len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse::<u16>().ok()
}

fn compute_backoff(initial_ms: u64, attempt: u32) -> Duration {
    let bounded = attempt.min(20).saturating_sub(1);
    let raw = initial_ms.saturating_mul(1u64 << bounded);
    Duration::from_millis(raw.clamp(1, BACKOFF_MAX_MS))
}

fn augment_with_prefix(req: &ProviderRequest, prefix: &str) -> ProviderRequest {
    let mut out = req.clone();
    out.messages.push(Message {
        id: MessageId::new(),
        // Reuse an existing session id when we can; otherwise mint a
        // fresh one. Transports today only read `role` + `content` off
        // the message for wire encoding, so the session id is opaque.
        session_id: out
            .messages
            .last()
            .map(|m| m.session_id)
            .unwrap_or_default(),
        role: MessageRole::Assistant,
        content: vec![ContentBlock::Text {
            text: prefix.to_string(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    });
    out
}

/// `Transport` wrapper that applies the retry policy to
/// [`Transport::complete_stream`]. Non-streaming `complete` calls
/// fall through to the inner transport unchanged — their retry
/// policy lives in the manifest-driven transport, not here.
pub struct RetryStreamTransport<T: Transport> {
    inner: Arc<T>,
    config: RetryConfig,
}

impl<T: Transport> RetryStreamTransport<T> {
    /// Wrap an inner transport with the streaming-retry policy.
    pub fn new(inner: T, config: RetryConfig) -> Self {
        Self {
            inner: Arc::new(inner),
            config,
        }
    }

    /// Borrowed inner transport (read-only).
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Borrowed config.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }
}

#[async_trait]
impl<T: Transport + 'static> Transport for RetryStreamTransport<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.inner.capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        // Non-streaming path: pass through. The wrapper's contract is
        // mid-stream retry only.
        self.inner.complete(req).await
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        let inner = self.inner.clone();
        let attempt: AttemptFactory = Arc::new(move |r: ProviderRequest| {
            let inner = inner.clone();
            Box::pin(async move { inner.complete_stream(&r).await })
        });
        Ok(retry_streaming(req.clone(), self.config, attempt))
    }
}

#[cfg(test)]
#[path = "streaming_retry_tests.rs"]
mod tests;

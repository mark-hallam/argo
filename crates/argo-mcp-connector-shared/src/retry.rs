//! Retry-with-exponential-backoff for MCP connector outbound calls.
//!
//! Connector calls fail transiently — DNS hiccups, upstream 5xx, transient
//! 429s, connection resets. Without retry these surface to the agent as if
//! the tool were broken. `retry_with_policy` absorbs them: it re-invokes
//! the operation up to `max_attempts` times, sleeping
//! `initial_delay_ms * multiplier^attempt + jitter` between tries (capped
//! at `max_delay_ms`). The jitter is critical for thundering-herd avoidance
//! when many concurrent calls all retry after the same upstream blip.
//!
//! ```no_run
//! use argo_mcp_connector_shared::retry::{
//!     default_http_policy, is_http_retriable, retry_with_policy,
//! };
//! use argo_mcp::{McpError, McpResult};
//!
//! async fn call_upstream() -> McpResult<()> { Ok(()) }
//!
//! # async fn doc() -> McpResult<()> {
//! let policy = default_http_policy();
//! retry_with_policy(&policy, is_http_retriable, || async {
//!     call_upstream().await
//! }).await
//! # }
//! ```
use argo_mcp::McpError;
use std::future::Future;
use std::time::Duration;

/// Configuration for [`retry_with_policy`].
///
/// Defaults are conservative — 3 attempts × 2x multiplier ≈ 3.75s max wall
/// clock before giving up. Operators that need tighter or looser bounds can
/// override fields or use [`RetryPolicy::tightened`] / [`RetryPolicy::loose`].
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Total attempts including the first try. Must be ≥ 1.
    pub max_attempts: u32,
    /// Delay applied before the *first* retry (attempt index 1).
    pub initial_delay_ms: u64,
    /// Multiplier applied to delay per attempt: `initial * multiplier^attempt`.
    pub multiplier: f64,
    /// Hard cap on per-sleep delay (after multiplier, before jitter).
    pub max_delay_ms: u64,
    /// Uniform jitter in `[0, jitter_ms)` added to each sleep. Set to 0 to
    /// disable jitter (mostly useful in tests).
    pub jitter_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 250,
            multiplier: 2.0,
            max_delay_ms: 5_000,
            jitter_ms: 100,
        }
    }
}

impl RetryPolicy {
    /// Tighter policy for latency-sensitive calls: 2 attempts, 100ms base.
    #[must_use]
    pub fn tightened() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 100,
            multiplier: 2.0,
            max_delay_ms: 1_000,
            jitter_ms: 50,
        }
    }

    /// Looser policy for slow-but-eventually-consistent upstreams.
    #[must_use]
    pub fn loose() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            multiplier: 2.0,
            max_delay_ms: 10_000,
            jitter_ms: 250,
        }
    }
}

/// HTTP-flavored default — identical to `RetryPolicy::default()` today.
#[must_use]
pub fn default_http_policy() -> RetryPolicy {
    RetryPolicy::default()
}

/// Classifier for HTTP-shaped MCP errors.
///
/// Returns `true` for:
/// - `McpError::Io` — covers connection resets, DNS, TLS handshake hiccups.
/// - `McpError::Timeout` — per-call deadline; worth one more try.
/// - `McpError::Rpc { code, .. }` where `code` is 429 or any 5xx — upstream
///   asking us to back off or signalling a server-side fault.
///
/// 4xx other than 429 is non-retriable (request is malformed; retrying will
/// fail the same way). Spawn / Decode / Crash / Shutdown / Initialize errors
/// are protocol-level and not transport-level — retrying is unsafe.
#[must_use]
pub fn is_http_retriable(err: &McpError) -> bool {
    match err {
        McpError::Io(_) | McpError::Timeout(_) => true,
        McpError::Rpc { code, .. } => *code == 429 || (500..=599).contains(code),
        McpError::Spawn(_)
        | McpError::Initialize(_)
        | McpError::Crash(_)
        | McpError::Decode(_)
        | McpError::Shutdown => false,
    }
}

/// Classifier for tonic/gRPC-shaped MCP errors.
///
/// Retries on JSON-RPC error codes 14 (`Unavailable`), 4 (`DeadlineExceeded`),
/// and 8 (`ResourceExhausted`) — the standard gRPC status codes that
/// indicate a transient server-side or rate-limit condition. Other codes
/// (e.g., 3 `InvalidArgument`, 7 `PermissionDenied`) are not retried.
/// Transport-level `Io` and `Timeout` are also retried.
#[must_use]
pub fn is_tonic_retriable(err: &McpError) -> bool {
    match err {
        McpError::Io(_) | McpError::Timeout(_) => true,
        McpError::Rpc { code, .. } => matches!(*code, 4 | 8 | 14),
        McpError::Spawn(_)
        | McpError::Initialize(_)
        | McpError::Crash(_)
        | McpError::Decode(_)
        | McpError::Shutdown => false,
    }
}

/// Compute the sleep before retry `attempt` (1-indexed, i.e. the delay
/// applied between the first try and the first retry uses `attempt = 1`).
///
/// `jitter_value` is the resolved random value in `[0, policy.jitter_ms)`;
/// it is passed in (rather than sampled inside) to keep the function pure
/// and easy to test deterministically.
#[must_use]
pub fn backoff_delay_ms(policy: &RetryPolicy, attempt: u32, jitter_value: u64) -> u64 {
    let exp = policy.multiplier.powi(attempt.saturating_sub(1) as i32);
    let scaled = (policy.initial_delay_ms as f64 * exp).min(policy.max_delay_ms as f64);
    let base = scaled.max(0.0) as u64;
    let jitter = if policy.jitter_ms == 0 {
        0
    } else {
        jitter_value % policy.jitter_ms
    };
    base.saturating_add(jitter)
}

/// Cheap per-call jitter source. Not cryptographically random — fine for
/// thundering-herd smoothing where any spread suffices.
fn sample_jitter_value() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()))
        .unwrap_or(0)
}

/// Run `op` under the retry policy. Returns the first successful result, or
/// the last error if all attempts fail or an error is classified as
/// non-retriable.
///
/// The `is_retriable` callback decides whether a failure is worth another
/// try. Returning `false` short-circuits to that error immediately.
///
/// `op` is `FnMut`: each retry calls the closure again to construct a fresh
/// future. Connectors typically pass `|| async { self.inner_call().await }`.
pub async fn retry_with_policy<F, Fut, T, E>(
    policy: &RetryPolicy,
    is_retriable: fn(&E) -> bool,
    mut op: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let max_attempts = policy.max_attempts.max(1);
    let mut attempt: u32 = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts || !is_retriable(&e) {
                    return Err(e);
                }
                let delay = backoff_delay_ms(policy, attempt, sample_jitter_value());
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}

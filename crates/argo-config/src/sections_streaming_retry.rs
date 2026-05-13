//! `[runtime.streaming_retry]` operator knobs.
//!
//! Defaults are inline `const`s rather than `defaults.rs` constants
//! because they're scoped to this single section and have no
//! cross-section reuse — keeping them local makes the public API of
//! `defaults` strictly smaller.
use serde::{Deserialize, Serialize};

/// `[runtime.streaming_retry]` — operator-tunable knobs for the
/// streaming-aware retry wrapper (`argo_provider::streaming_retry`).
///
/// When a streaming LLM call fails mid-stream (network blip, 5xx,
/// 429), the wrapper buffers chunks received so far and retries with
/// the buffered text pre-filled into an assistant message (the
/// `assistant_prefix` pattern supported by Anthropic and OpenAI). The
/// model continues from where it left off instead of restarting.
///
/// Defaults are conservative: enabled by default with 3 retries on
/// exponential backoff (1s/2s/4s). After the budget is exhausted the
/// wrapper returns [`argo_types::ArgoError::StreamingRetryExhausted`]
/// with the partial content as a recoverable hint. Operators with
/// pathologically failing providers should lower `max_attempts` to
/// avoid burning tokens on the same prompt.
///
/// ```
/// use argo_config::StreamingRetryConfig;
/// let cfg = StreamingRetryConfig::default();
/// assert!(cfg.enabled);
/// assert_eq!(cfg.max_attempts, 3);
/// assert_eq!(cfg.backoff_initial_ms, 1000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamingRetryConfig {
    /// Master enable. When `false`, transports skip the retry wrapper
    /// and behave exactly as before (mid-stream failures propagate
    /// unchanged). On by default.
    pub enabled: bool,
    /// Maximum number of retries after the initial attempt. The first
    /// attempt is not counted, so `max_attempts = 3` means up to four
    /// total tries before [`argo_types::ArgoError::StreamingRetryExhausted`]
    /// surfaces. Set to `0` to disable retries entirely while still
    /// preserving the buffer-on-failure observability path.
    pub max_attempts: u32,
    /// Base backoff in milliseconds. The wrapper doubles this for
    /// each subsequent retry (1s / 2s / 4s at the default), capped
    /// by an internal ceiling so a misconfigured value can't stall
    /// the stream indefinitely.
    pub backoff_initial_ms: u64,
}

/// Default for [`StreamingRetryConfig::enabled`].
pub const DEFAULT_STREAMING_RETRY_ENABLED: bool = true;
/// Default for [`StreamingRetryConfig::max_attempts`].
pub const DEFAULT_STREAMING_RETRY_MAX_ATTEMPTS: u32 = 3;
/// Default for [`StreamingRetryConfig::backoff_initial_ms`].
pub const DEFAULT_STREAMING_RETRY_BACKOFF_INITIAL_MS: u64 = 1000;

impl Default for StreamingRetryConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_STREAMING_RETRY_ENABLED,
            max_attempts: DEFAULT_STREAMING_RETRY_MAX_ATTEMPTS,
            backoff_initial_ms: DEFAULT_STREAMING_RETRY_BACKOFF_INITIAL_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_streaming_retry_matches_brief() {
        let cfg = StreamingRetryConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_attempts, 3);
        assert_eq!(cfg.backoff_initial_ms, 1000);
    }

    #[test]
    fn runtime_section_parses_from_toml() {
        use crate::sections_context::RuntimeConfig;

        let toml_src = r#"
[runtime.streaming_retry]
enabled = false
max_attempts = 5
backoff_initial_ms = 250
"#;
        #[derive(Debug, serde::Deserialize)]
        struct Wrapper {
            runtime: RuntimeConfig,
        }
        let w: Wrapper = toml::from_str(toml_src).unwrap();
        assert!(!w.runtime.streaming_retry.enabled);
        assert_eq!(w.runtime.streaming_retry.max_attempts, 5);
        assert_eq!(w.runtime.streaming_retry.backoff_initial_ms, 250);
    }

    #[test]
    fn empty_toml_yields_defaults() {
        use crate::sections_context::RuntimeConfig;

        #[derive(Debug, serde::Deserialize, Default)]
        #[serde(default)]
        struct Wrapper {
            runtime: RuntimeConfig,
        }
        let w: Wrapper = toml::from_str("").unwrap();
        assert_eq!(
            w.runtime.streaming_retry.max_attempts,
            DEFAULT_STREAMING_RETRY_MAX_ATTEMPTS
        );
    }
}

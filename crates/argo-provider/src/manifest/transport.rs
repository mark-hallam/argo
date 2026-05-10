//! `[transport]` block of the provider manifest.
//!
//! Wire transport configuration: streaming mode, retry/backoff, timeouts,
//! HTTP paths, and adaptive heartbeat policy. Re-exported from
//! `manifest::mod` so callers continue to import these names from
//! `argo_provider::manifest`.
use serde::{Deserialize, Serialize};

/// `[transport]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSpec {
    /// Streaming mode.
    pub streaming: StreamingMode,
    /// Path appended to `base_url` for chat-completion calls.
    #[serde(default = "default_chat_path")]
    pub chat_path: String,
    /// Path for `/v1/models` discovery.
    #[serde(default = "default_models_path")]
    pub models_path: String,
    /// TCP connect timeout (ms).
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// Read timeout (ms) — overridden per-turn by adaptive heartbeat policy
    /// (§6.4) when `heartbeat_event` fires.
    #[serde(default = "default_read_timeout")]
    pub read_timeout_ms: u64,
    /// Max retries for retryable status codes.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Initial exponential-backoff delay (ms).
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    /// Cap for exponential backoff (ms).
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
    /// SSE event name that resets the heartbeat timer (NEW v0.2).
    #[serde(default)]
    pub heartbeat_event: Option<String>,
    /// Heartbeat grace window (ms) — adaptive timeout extension.
    #[serde(default)]
    pub heartbeat_grace_ms: Option<u64>,
    /// Whether chunked HTTP keep-alive ticks reset the timer.
    #[serde(default)]
    pub keepalive_chunks_reset: bool,
}

fn default_chat_path() -> String {
    "/v1/chat/completions".to_string()
}
fn default_models_path() -> String {
    "/v1/models".to_string()
}
fn default_connect_timeout() -> u64 {
    5_000
}
fn default_read_timeout() -> u64 {
    60_000
}
fn default_max_retries() -> u32 {
    3
}
fn default_initial_backoff() -> u64 {
    200
}
fn default_max_backoff() -> u64 {
    4_000
}

/// SSE / non-stream toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamingMode {
    /// Server-Sent Events streaming.
    Sse,
    /// No streaming — response delivered as a single JSON body.
    None,
}

//! `[runtime.context_compress]` — operator-tunable knobs for the
//! `argo-runtime::context_compress` module
//! (`mission-context-window-aware-compression`, MISSION_BACKLOG §8
//! #247 Amendment §5.8).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::ContextCompressConfig`.
use serde::{Deserialize, Serialize};

/// Default trigger threshold (% of model context window) at which
/// compression fires.
pub const DEFAULT_CONTEXT_COMPRESS_TRIGGER_PCT: u32 = 80;
/// Default number of trailing user turns kept verbatim.
pub const DEFAULT_CONTEXT_COMPRESS_KEEP_LAST_TURNS: u32 = 10;
/// Default cap on the generated summary's token count.
pub const DEFAULT_CONTEXT_COMPRESS_SUMMARY_MAX_TOKENS: u32 = 500;

/// `[runtime.context_compress]` — operator-tunable knobs for the
/// `argo-runtime::context_compress` module
/// (`mission-context-window-aware-compression`, MISSION_BACKLOG §8
/// #247 Amendment §5.8).
///
/// The runtime module is the source of truth for the in-memory
/// representation; this struct mirrors only the values the operator
/// can override at the TOML layer. Default values match
/// `argo_runtime::context_compress::CompressionConfig::default()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextCompressConfig {
    /// Master enable. When `false`, the runtime never invokes the
    /// compressor regardless of token-count.
    pub enabled: bool,
    /// Compression fires when running token-count crosses
    /// `trigger_pct% * model.context_window_tokens`. Aggressive
    /// operators set this lower (e.g. `70`) to compress earlier.
    pub trigger_pct: u32,
    /// Trailing user turns kept verbatim. Must be `> 0`; the
    /// runtime rejects a `0` value so the most-recent user prompt
    /// is never lost to the summariser.
    pub keep_last_turns: u32,
    /// Cap on the generated summary's token count. Passed through to
    /// the summariser; the runtime does not enforce it post-hoc.
    pub summary_max_tokens: u32,
}

impl Default for ContextCompressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_pct: DEFAULT_CONTEXT_COMPRESS_TRIGGER_PCT,
            keep_last_turns: DEFAULT_CONTEXT_COMPRESS_KEEP_LAST_TURNS,
            summary_max_tokens: DEFAULT_CONTEXT_COMPRESS_SUMMARY_MAX_TOKENS,
        }
    }
}

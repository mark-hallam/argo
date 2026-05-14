//! `[runtime.prompt_cache_affinity]` — sticky-session affinity for
//! the Anthropic prompt cache (`mission-prompt-cache-session-affinity`).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::PromptCacheAffinityConfig`.
use serde::{Deserialize, Serialize};

/// `[runtime.prompt_cache_affinity]` — sticky-session affinity for
/// the Anthropic prompt cache (`mission-prompt-cache-session-affinity`).
///
/// Defaults to enabled because the warm cache is a free latency +
/// cost win; operators should not need to opt in. `ttl_secs` of
/// 270 = Anthropic's 5-minute cache TTL minus 30s headroom for
/// clock skew + network jitter. Operators on dedicated infra with
/// tighter clock guarantees can raise the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptCacheAffinityConfig {
    /// Master enable. When `false`, the runtime mints a fresh
    /// SessionId every turn (the pre-brief behavior).
    pub enabled: bool,
    /// How many seconds an entry survives before
    /// `acquire_or_mint` mints fresh. 270s default.
    pub ttl_secs: u64,
}

impl Default for PromptCacheAffinityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_secs: 270,
        }
    }
}

//! `[runtime]` parent — aggregator for the `[runtime.*]` config
//! sections. Each child config lives in its own sibling module
//! (`sections_context_compress`, `sections_action_throttle`,
//! `sections_sandbox_prewarm`, `sections_mcp_pool`,
//! `sections_prompt_cache`, `sections_worker_resource`); this file
//! owns only [`RuntimeConfig`] + [`VerticalLockConfig`] and the
//! `pub use` façade that keeps `argo_config::ContextCompressConfig`
//! (and friends) resolving against the historic path.
use serde::{Deserialize, Serialize};

use crate::sections_streaming_retry::StreamingRetryConfig;

pub use crate::sections_action_throttle::{
    ActionThrottleConfig, ActionThrottlePolicyConfig, DEFAULT_ACTION_THROTTLE_AUDIT_SECS,
};
pub use crate::sections_context_compress::ContextCompressConfig;
pub use crate::sections_mcp_pool::{
    McpPoolConfig, DEFAULT_MCP_POOL_IDLE_TTL_SECS, DEFAULT_MCP_POOL_MAX_CLIENTS,
};
pub use crate::sections_plugin_registry::{
    PluginManifestEntry, PluginRegistryConfig, DEFAULT_PLUGIN_REGISTRY_TIMEOUT_SECS,
};
pub use crate::sections_prompt_cache::PromptCacheAffinityConfig;
pub use crate::sections_sandbox_prewarm::SandboxPrewarmConfig;
pub use crate::sections_worker_resource::WorkerResourceConfig;

const DEFAULT_VERTICAL_LOCK_LEASE_SECS: u64 = 30;
const DEFAULT_VERTICAL_LOCK_ACQUIRE_TIMEOUT_SECS: u64 = 5;

/// `[runtime.vertical_lock]` — per-worker vertical advisory lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerticalLockConfig {
    /// Master enable. When `false`, dispatch wiring should skip the
    /// advisory lock and preserve existing behavior.
    pub enabled: bool,
    /// How long a held lock survives after a process crash.
    pub lease_secs: u64,
    /// How long a competing acquire waits before surfacing contention.
    pub acquire_timeout_secs: u64,
}

impl Default for VerticalLockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lease_secs: DEFAULT_VERTICAL_LOCK_LEASE_SECS,
            acquire_timeout_secs: DEFAULT_VERTICAL_LOCK_ACQUIRE_TIMEOUT_SECS,
        }
    }
}

/// `[runtime]` — parent for runtime-side TOML knobs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// `[runtime.streaming_retry]` — streaming-aware provider retry.
    pub streaming_retry: StreamingRetryConfig,
    /// `[runtime.context_compress]`.
    pub context_compress: ContextCompressConfig,
    /// `[runtime.action_throttle]` + per-action sub-tables.
    pub action_throttle: ActionThrottleConfig,
    /// `[runtime.sandbox_prewarm]` — E2B pool burst pre-warming.
    pub sandbox_prewarm: SandboxPrewarmConfig,
    /// `[runtime.mcp_pool]` — warm MCP subprocess pool sizing.
    pub mcp_pool: McpPoolConfig,
    /// `[runtime.prompt_cache_affinity]` — sticky-session affinity
    /// for the Anthropic prompt cache.
    pub prompt_cache_affinity: PromptCacheAffinityConfig,
    /// `[runtime.worker_resource_cage]` — per-worker cgroup-v2
    /// memory + CPU caps.
    pub worker_resource_cage: WorkerResourceConfig,
    /// `[runtime.plugin_registry]` — configured plugin subprocesses.
    pub plugin_registry: PluginRegistryConfig,
    /// `[runtime.vertical_lock]` — per-tenant/vertical/worker mutex.
    pub vertical_lock: VerticalLockConfig,
}

#[cfg(test)]
#[path = "sections_context_tests.rs"]
mod tests;

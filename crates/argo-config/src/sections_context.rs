//! `[runtime.*]` config sections that share the runtime parent.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::sections_streaming_retry::StreamingRetryConfig;

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

/// Default cumulative-wait threshold (seconds) past which a
/// dispatch emits a `runtime.action_throttled` audit row. Mirrors
/// [`argo_runtime::action_throttle::DEFAULT_AUDIT_THRESHOLD`].
pub const DEFAULT_ACTION_THROTTLE_AUDIT_SECS: u32 = 10;

/// `[runtime.action_throttle.<action_type>]` — operator-defined
/// per-action-type rate-limit policy. Parsed off the TOML and
/// mapped to `argo_runtime::action_throttle::ThrottlePolicy` at
/// registry construction time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionThrottlePolicyConfig {
    /// Average sustained dispatch rate (tokens / sec). `0` or
    /// negative is treated as misconfigured and the runtime
    /// silently skips installing the bucket.
    pub tokens_per_sec: f32,
    /// Maximum burst depth (tokens). `0` is treated as
    /// misconfigured.
    pub burst: u32,
}

impl Default for ActionThrottlePolicyConfig {
    fn default() -> Self {
        Self {
            tokens_per_sec: 1.0,
            burst: 1,
        }
    }
}

/// `[runtime.action_throttle]` — master enable + per-action-type
/// policy map (`mission-per-action-type-throttling`, MISSION_BACKLOG
/// §8 #254).
///
/// Disabled by default for backward compatibility — the runtime
/// dispatch path is zero-overhead until an operator sets `enabled
/// = true`. Per-action policies live in nested
/// `[runtime.action_throttle.policies.<action_type>]` blocks. The
/// `.policies` sub-key (rather than `[runtime.action_throttle.<action_type>]`
/// directly) sidesteps a serde-flatten foot-gun where bool / u32
/// scalars at the same level as a `HashMap<String, struct>` collide
/// during deserialisation; the operator UX is otherwise identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionThrottleConfig {
    /// Master enable. When `false`, the runtime never installs
    /// the gate regardless of configured policies.
    pub enabled: bool,
    /// Cumulative-wait threshold (seconds) past which a dispatch
    /// emits a `runtime.action_throttled` audit row.
    pub audit_threshold_secs: u32,
    /// Per-action-type policy table. Each entry binds one TOML
    /// block `[runtime.action_throttle.policies.<action_type>]`.
    pub policies: HashMap<String, ActionThrottlePolicyConfig>,
}

impl Default for ActionThrottleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            audit_threshold_secs: DEFAULT_ACTION_THROTTLE_AUDIT_SECS,
            policies: HashMap::new(),
        }
    }
}

/// Default spawn-rate threshold for the burst pre-warm trigger.
pub const DEFAULT_SANDBOX_PREWARM_SPAWN_RATE_THRESHOLD: u32 = 3;
/// Default reservoir cap.
pub const DEFAULT_SANDBOX_PREWARM_MAX: u32 = 5;
/// Default sliding-window width for spawn-rate detection.
pub const DEFAULT_SANDBOX_PREWARM_WINDOW_SECS: u64 = 300;
/// Default extra sandboxes opened on each burst trigger.
pub const DEFAULT_SANDBOX_PREWARM_BURST_SIZE: u32 = 2;
/// Default idle TTL for pre-warmed sandboxes.
pub const DEFAULT_SANDBOX_PREWARM_IDLE_TTL_SECS: u64 = 600;

/// `[runtime.sandbox_prewarm]` — burst pre-warming for the E2B
/// sandbox pool (`mission-burst-pre-warming-sandbox`,
/// MISSION_BACKLOG §8 #255). Disabled by default so opting in is
/// always a deliberate operator action; pre-warming consumes
/// E2B-minutes even when the reservoir goes unused.
///
/// The in-memory shape lives in
/// `argo_executor_computer_use::sandbox_prewarm::SandboxPrewarmConfig`;
/// this struct mirrors the TOML-side knobs and is mapped at the
/// executor's bootstrap step. The split keeps `argo-config` free
/// of a build-time dependency on the executor crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxPrewarmConfig {
    /// Master enable. When `false`, the prewarmer is not
    /// constructed and the dispatch path stays zero-overhead.
    pub enabled: bool,
    /// Spawn-events-per-window above which a burst pre-warm
    /// fires. Strict greater-than: with `3`, 3 spawns/window does
    /// NOT trigger; 4 does.
    pub spawn_rate_threshold: u32,
    /// Reservoir cap. Excess pre-warmed sandboxes are closed
    /// rather than admitted.
    pub max_prewarm: u32,
    /// Sliding-window width (seconds) used by the spawn-rate
    /// trigger.
    pub window_seconds: u64,
    /// Extra sandboxes opened on each burst trigger.
    pub burst_size: u32,
    /// Pre-warmed sandbox idle TTL (seconds). Past this the
    /// reservoir reaper drops the entry. Mirrors the in-band pool
    /// idle TTL so the two layers stay aligned.
    pub idle_ttl_seconds: u64,
    /// Operator-active hours (`0..=23`). Empty means "every hour
    /// is active" (no quiet hours configured). Non-empty is the
    /// inverse of the operator's quiet-hours block — the hourly
    /// pre-warm trigger fires only when `now.hour()` matches an
    /// entry.
    pub active_hours: Vec<u8>,
}

impl Default for SandboxPrewarmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            spawn_rate_threshold: DEFAULT_SANDBOX_PREWARM_SPAWN_RATE_THRESHOLD,
            max_prewarm: DEFAULT_SANDBOX_PREWARM_MAX,
            window_seconds: DEFAULT_SANDBOX_PREWARM_WINDOW_SECS,
            burst_size: DEFAULT_SANDBOX_PREWARM_BURST_SIZE,
            idle_ttl_seconds: DEFAULT_SANDBOX_PREWARM_IDLE_TTL_SECS,
            active_hours: Vec::new(),
        }
    }
}

/// Default LRU capacity of the MCP-client connection pool. Mirrors
/// `argo_mcp::DEFAULT_POOL_MAX_CLIENTS`.
pub const DEFAULT_MCP_POOL_MAX_CLIENTS: u32 = 32;
/// Default idle-TTL for the MCP-client pool, in seconds. Mirrors
/// `argo_mcp::DEFAULT_POOL_IDLE_TTL_SECS`.
pub const DEFAULT_MCP_POOL_IDLE_TTL_SECS: u64 = 600;

/// `[runtime.mcp_pool]` — operator-tunable knobs for the warm MCP
/// subprocess pool (`mission-mcp-connection-pooling`,
/// MISSION_BACKLOG §8 #252). The pool itself lives in
/// `argo_mcp::pool`; this struct mirrors the TOML-side knobs and is
/// mapped at AgentDeps wiring time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct McpPoolConfig {
    /// Hard cap on cached MCP clients per pool. New acquisitions
    /// past this evict the LRU entry.
    pub max_clients: u32,
    /// Seconds an entry can sit untouched before the idle reaper
    /// removes it on the next tick.
    pub idle_ttl_secs: u64,
}

impl Default for McpPoolConfig {
    fn default() -> Self {
        Self {
            max_clients: DEFAULT_MCP_POOL_MAX_CLIENTS,
            idle_ttl_secs: DEFAULT_MCP_POOL_IDLE_TTL_SECS,
        }
    }
}

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

/// Default memory cap (MiB) for the per-worker cgroup-v2 cage.
/// Mirrors `argo_runtime::worker_resource_cage::DEFAULT_MEMORY_MAX_MIB`.
pub const DEFAULT_WORKER_RESOURCE_MEMORY_MAX_MIB: u64 = 1024;
/// Default `cpu.weight` for the per-worker cgroup-v2 cage. Cgroup-v2
/// neutral weight; mirrors
/// `argo_runtime::worker_resource_cage::DEFAULT_CPU_WEIGHT`.
pub const DEFAULT_WORKER_RESOURCE_CPU_WEIGHT: u32 = 100;

/// `[runtime.worker_resource_cage]` — per-worker cgroup-v2 memory +
/// CPU caps (`mission-cgroup-per-worker-resource-limits`,
/// MISSION_BACKLOG §8 #236). Mirrors the in-memory shape in
/// `argo_runtime::worker_resource_cage::WorkerResourceConfig`; the
/// orchestrator maps this struct to the runtime one at worker-spawn
/// time. Split here (rather than reusing the runtime type directly)
/// keeps `argo-config` free of a build-time dependency on
/// `argo-runtime`.
///
/// Defaults to enabled with a 1 GiB memory cap + neutral
/// `cpu.weight=100`. On non-Linux hosts the runtime returns
/// `CageStatus::NotSupported` regardless of these values, so
/// default-enabled is safe for macOS dev workflows; the runbook
/// (`docs/skycot/runbook/worker-resource-cage.md`) covers
/// Docker/Lima for operators who need confinement off-Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerResourceConfig {
    /// Master enable. When `false`, the runtime returns
    /// `NotSupported { reason: "disabled by config" }` on any host
    /// and the worker runs unconfined.
    pub enabled: bool,
    /// Hard memory cap per worker, in MiB. Written into
    /// `memory.max`; breach triggers a kernel OOM kill that the
    /// `oom_restart` handler downstream observes.
    pub memory_max_mib: u64,
    /// CPU weight per worker. cgroup-v2 range is `1..=10_000`; the
    /// runtime clamps out-of-range values. `100` is the neutral
    /// weight.
    pub cpu_weight: u32,
}

impl Default for WorkerResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_max_mib: DEFAULT_WORKER_RESOURCE_MEMORY_MAX_MIB,
            cpu_weight: DEFAULT_WORKER_RESOURCE_CPU_WEIGHT,
        }
    }
}

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
    /// `[runtime.vertical_lock]` — per-tenant/vertical/worker mutex.
    pub vertical_lock: VerticalLockConfig,
}

#[cfg(test)]
#[path = "sections_context_tests.rs"]
mod tests;

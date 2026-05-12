//! `[runtime.*]` config sections. Extracted into its own sibling
//! file because [`crate::sections`] sits at the 500-LOC ceiling.
//! Re-exported through [`crate`] so callers see the same flat
//! surface as the other section structs.
//!
//! Currently holds:
//!
//! - `[runtime.context_compress]` — context-window-aware
//!   compression knobs (`mission-context-window-aware-compression`).
//! - `[runtime.action_throttle]` + `[runtime.action_throttle.<action_type>]`
//!   — per-action-type token-bucket throttling knobs
//!   (`mission-per-action-type-throttling`).
//! - `[runtime.sandbox_prewarm]` — E2B sandbox pre-warm reservoir
//!   knobs (`mission-burst-pre-warming-sandbox`).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// `[runtime]` — parent for runtime-side TOML knobs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// `[runtime.context_compress]`.
    pub context_compress: ContextCompressConfig,
    /// `[runtime.action_throttle]` + per-action sub-tables.
    pub action_throttle: ActionThrottleConfig,
    /// `[runtime.sandbox_prewarm]` — E2B pool burst pre-warming.
    pub sandbox_prewarm: SandboxPrewarmConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_runtime_constants() {
        let cfg = ContextCompressConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.trigger_pct, 80);
        assert_eq!(cfg.keep_last_turns, 10);
        assert_eq!(cfg.summary_max_tokens, 500);
    }

    #[test]
    fn parses_from_toml() {
        let body = r#"
[context_compress]
enabled = false
trigger_pct = 70
keep_last_turns = 5
summary_max_tokens = 800
"#;
        let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
        assert!(!cfg.context_compress.enabled);
        assert_eq!(cfg.context_compress.trigger_pct, 70);
        assert_eq!(cfg.context_compress.keep_last_turns, 5);
        assert_eq!(cfg.context_compress.summary_max_tokens, 800);
    }

    #[test]
    fn empty_toml_yields_defaults() {
        let cfg: RuntimeConfig = toml::from_str("").expect("parse");
        assert_eq!(cfg.context_compress, ContextCompressConfig::default());
    }

    #[test]
    fn action_throttle_defaults_off() {
        let cfg = ActionThrottleConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.audit_threshold_secs, 10);
        assert!(cfg.policies.is_empty());
    }

    #[test]
    fn sandbox_prewarm_defaults_disabled() {
        let cfg = SandboxPrewarmConfig::default();
        assert!(!cfg.enabled, "pre-warm must be off by default");
        assert_eq!(cfg.spawn_rate_threshold, 3);
        assert_eq!(cfg.max_prewarm, 5);
        assert_eq!(cfg.window_seconds, 300);
        assert_eq!(cfg.burst_size, 2);
        assert_eq!(cfg.idle_ttl_seconds, 600);
        assert!(cfg.active_hours.is_empty());
    }

    #[test]
    fn sandbox_prewarm_parses_from_toml() {
        let body = r#"
[sandbox_prewarm]
enabled = true
spawn_rate_threshold = 5
max_prewarm = 7
window_seconds = 120
burst_size = 3
idle_ttl_seconds = 900
active_hours = [9, 10, 11, 14, 15, 16]
"#;
        let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
        assert!(cfg.sandbox_prewarm.enabled);
        assert_eq!(cfg.sandbox_prewarm.spawn_rate_threshold, 5);
        assert_eq!(cfg.sandbox_prewarm.max_prewarm, 7);
        assert_eq!(cfg.sandbox_prewarm.window_seconds, 120);
        assert_eq!(cfg.sandbox_prewarm.burst_size, 3);
        assert_eq!(cfg.sandbox_prewarm.idle_ttl_seconds, 900);
        assert_eq!(cfg.sandbox_prewarm.active_hours, vec![9, 10, 11, 14, 15, 16]);
    }

    #[test]
    fn action_throttle_parses_from_toml() {
        let body = r#"
[action_throttle]
enabled = true
audit_threshold_secs = 20

[action_throttle.policies.send_email]
tokens_per_sec = 5.0
burst = 10

[action_throttle.policies.post_listing]
tokens_per_sec = 2.0
burst = 3
"#;
        let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
        assert!(cfg.action_throttle.enabled);
        assert_eq!(cfg.action_throttle.audit_threshold_secs, 20);
        let email = &cfg.action_throttle.policies["send_email"];
        assert!((email.tokens_per_sec - 5.0).abs() < 1e-6);
        assert_eq!(email.burst, 10);
        let listing = &cfg.action_throttle.policies["post_listing"];
        assert!((listing.tokens_per_sec - 2.0).abs() < 1e-6);
        assert_eq!(listing.burst, 3);
    }
}

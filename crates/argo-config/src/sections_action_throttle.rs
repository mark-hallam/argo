//! `[runtime.action_throttle]` — master enable + per-action-type
//! policy map (`mission-per-action-type-throttling`, MISSION_BACKLOG
//! §8 #254).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::ActionThrottleConfig`.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

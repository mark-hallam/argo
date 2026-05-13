//! Provider-routing / model-evaluation config sections: `[routing]`,
//! `[routing.tiered]`, `[judge]`, `[forge]`. Extracted from
//! [`crate::sections`] to keep that module under the 500-LOC ceiling.
//! Re-exported through `sections` so the public API is unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};

/// `[routing]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingConfig {
    /// `fixed` | `tiered` | `policy_plugin`.
    pub policy: String,
    /// `[routing.tiered]` (when `policy = "tiered"`).
    pub tiered: Option<RoutingTieredConfig>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            policy: defaults::DEFAULT_ROUTING_POLICY.into(),
            tiered: None,
        }
    }
}

/// `[routing.tiered]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingTieredConfig {
    /// Classifier provider:model.
    pub classifier: String,
    /// Simple tier provider:model.
    pub simple: String,
    /// Medium tier provider:model.
    pub medium: String,
    /// Complex tier provider:model.
    pub complex: String,
    /// Classifier-latency budget (ms).
    pub classify_threshold_ms: u32,
    /// Fall back to default model if classification fails.
    pub fallback_to_default_on_error: bool,
}

impl Default for RoutingTieredConfig {
    fn default() -> Self {
        Self {
            classifier: String::new(),
            simple: String::new(),
            medium: String::new(),
            complex: String::new(),
            classify_threshold_ms: defaults::DEFAULT_CLASSIFY_THRESHOLD_MS,
            fallback_to_default_on_error: true,
        }
    }
}

/// `[judge]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeConfig {
    /// Master enable.
    pub enabled: bool,
    /// Judge model.
    pub model: String,
    /// Rubric name (default rubric or workspace path).
    pub rubric: String,
    /// `partial` | `failure` | `success`.
    pub verdict_on_error: String,
    /// Persist evidence hashes.
    pub log_evidence_hashes: bool,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::DEFAULT_JUDGE_ENABLED,
            model: String::new(),
            rubric: defaults::DEFAULT_JUDGE_RUBRIC.into(),
            verdict_on_error: defaults::DEFAULT_JUDGE_VERDICT_ON_ERROR.into(),
            log_evidence_hashes: true,
        }
    }
}

/// `[forge]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ForgeConfig {
    /// Default off (explicit opt-in).
    pub enabled: bool,
    /// Optimizer model.
    pub optimizer_model: String,
    /// Min number of examples required to start.
    pub default_examples_min: u32,
    /// Default iteration cap.
    pub default_iterations: u32,
    /// Per-run budget cap (USD micros).
    pub budget_per_run_usd_micros: u64,
    /// Auto-promote threshold (final_score in `[0, 1]`).
    pub auto_promote_threshold: f32,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            optimizer_model: String::new(),
            default_examples_min: defaults::DEFAULT_FORGE_EXAMPLES_MIN,
            default_iterations: defaults::DEFAULT_FORGE_ITERATIONS,
            budget_per_run_usd_micros: defaults::DEFAULT_FORGE_BUDGET_PER_RUN_USD_MICROS,
            auto_promote_threshold: defaults::DEFAULT_FORGE_AUTO_PROMOTE_THRESHOLD,
        }
    }
}

//! Provider-routing / model-evaluation config sections: `[routing]`,
//! `[routing.tiered]`, `[judge]`, `[forge]`. Extracted from
//! [`crate::sections`] to keep that module under the 500-LOC ceiling.
//! Re-exported through `sections` so the public API is unchanged.
use crate::defaults;
use argo_types::{ArgoError, ArgoResult, ProviderModel, RoutingPolicy, TieredRouting};
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

impl RoutingTieredConfig {
    /// Convert the string `provider:model` tier table into the runtime
    /// [`TieredRouting`] policy the agent loop applies per session.
    ///
    /// Each field must be `<provider>:<model>` (e.g.
    /// `claude-code-cli:claude-sonnet-4-6`). The conversion is the
    /// glue that lets an `argo.toml` `[routing.tiered]` block drive
    /// the cost-tier router — Sonnet floor for `simple`/`medium`,
    /// Opus for `complex`, Sonnet (never Haiku) as the classifier.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when any tier value is missing the
    /// `:` separator or has an empty provider/model half.
    ///
    /// # Examples
    /// ```
    /// use argo_config::RoutingTieredConfig;
    /// let cfg = RoutingTieredConfig {
    ///     classifier: "claude-code-cli:claude-sonnet-4-6".into(),
    ///     simple: "claude-code-cli:claude-sonnet-4-6".into(),
    ///     medium: "claude-code-cli:claude-sonnet-4-6".into(),
    ///     complex: "claude-code-cli:claude-opus-4-8".into(),
    ///     classify_threshold_ms: 120,
    ///     fallback_to_default_on_error: true,
    /// };
    /// let policy = cfg.to_routing_policy().expect("valid tiers");
    /// # let _ = policy;
    /// ```
    pub fn to_routing_policy(&self) -> ArgoResult<RoutingPolicy> {
        Ok(RoutingPolicy::Tiered(self.to_tiered_routing()?))
    }

    /// Build the [`TieredRouting`] from the parsed tier strings.
    ///
    /// # Errors
    /// Same as [`Self::to_routing_policy`].
    pub fn to_tiered_routing(&self) -> ArgoResult<TieredRouting> {
        Ok(TieredRouting {
            classifier_model: parse_provider_model("classifier", &self.classifier)?,
            simple: parse_provider_model("simple", &self.simple)?,
            medium: parse_provider_model("medium", &self.medium)?,
            complex: parse_provider_model("complex", &self.complex)?,
            classify_threshold_ms: u64::from(self.classify_threshold_ms),
            fallback_to_default_on_error: self.fallback_to_default_on_error,
        })
    }
}

/// Parse a `<provider>:<model>` tier string into a [`ProviderModel`].
/// Both halves must be non-empty. The model id may itself contain `:`
/// is not expected (model ids are dotted/dashed), so we split on the
/// first `:` only.
fn parse_provider_model(tier: &str, raw: &str) -> ArgoResult<ProviderModel> {
    let (provider, model) = raw.split_once(':').ok_or_else(|| {
        ArgoError::Schema(format!(
            "[routing.tiered].{tier} must be `<provider>:<model>` (e.g. \
             `claude-code-cli:claude-sonnet-4-6`); got `{raw}`"
        ))
    })?;
    let provider = provider.trim();
    let model = model.trim();
    if provider.is_empty() || model.is_empty() {
        return Err(ArgoError::Schema(format!(
            "[routing.tiered].{tier} has an empty provider or model half: `{raw}`"
        )));
    }
    Ok(ProviderModel {
        provider: provider.to_string(),
        model: model.to_string(),
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_tiered() -> RoutingTieredConfig {
        RoutingTieredConfig {
            classifier: "claude-code-cli:claude-sonnet-4-6".into(),
            simple: "claude-code-cli:claude-sonnet-4-6".into(),
            medium: "claude-code-cli:claude-sonnet-4-6".into(),
            complex: "claude-code-cli:claude-opus-4-8".into(),
            classify_threshold_ms: 120,
            fallback_to_default_on_error: true,
        }
    }

    #[test]
    fn dev_config_maps_sonnet_floor_and_opus_complex_no_haiku() {
        let routing = dev_tiered().to_tiered_routing().expect("valid tiers");
        // Sonnet floor: classifier + simple + medium all Sonnet.
        assert_eq!(routing.classifier_model.provider, "claude-code-cli");
        assert_eq!(routing.classifier_model.model, "claude-sonnet-4-6");
        assert_eq!(routing.simple.model, "claude-sonnet-4-6");
        assert_eq!(routing.medium.model, "claude-sonnet-4-6");
        // Only complex rises to Opus.
        assert_eq!(routing.complex.model, "claude-opus-4-8");
        assert_eq!(routing.classify_threshold_ms, 120);
        // Hard floor: Haiku must appear nowhere in the dev routing path.
        for m in [
            &routing.classifier_model,
            &routing.simple,
            &routing.medium,
            &routing.complex,
        ] {
            assert!(
                !m.model.contains("haiku"),
                "Haiku must never appear in dev tiered routing; saw {}",
                m.model
            );
        }
    }

    #[test]
    fn to_routing_policy_wraps_tiered() {
        let policy = dev_tiered().to_routing_policy().expect("valid");
        assert!(matches!(policy, RoutingPolicy::Tiered(_)));
    }

    #[test]
    fn rejects_tier_without_colon_separator() {
        let mut cfg = dev_tiered();
        cfg.complex = "claude-opus-4-8".into(); // missing `provider:`
        let err = cfg.to_tiered_routing().unwrap_err();
        assert!(
            format!("{err}").contains("complex"),
            "error must name the offending tier; got {err}"
        );
    }

    #[test]
    fn rejects_empty_model_half() {
        let mut cfg = dev_tiered();
        cfg.simple = "claude-code-cli:".into();
        let err = cfg.to_tiered_routing().unwrap_err();
        assert!(format!("{err}").contains("empty"), "got {err}");
    }
}

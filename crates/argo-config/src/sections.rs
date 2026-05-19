//! `[…]` section structs that make up [`crate::ArgoConfig`].
//!
//! Each struct binds one TOML section, owns its own `Default` (so
//! defaults stay co-located with the struct), and — where it
//! translates to an in-memory primitive used elsewhere — exposes a
//! single `to_*` / `*_policy` method (e.g. [`BudgetConfig::to_cost_ceilings`]).
//!
//! The actual struct definitions live in sibling `sections_*` modules
//! (one per concern, each ≤ 250 LOC) so this file stays under the
//! 500-LOC ceiling. Every public type is re-exported here so callers'
//! `use argo_config::sections::ProviderSection;` imports stay unchanged.
//!
//! Sibling modules:
//! - [`crate::sections_budget`] — `[budget]`, `[delegate]`, `[parallel]`
//! - [`crate::sections_core`] — `[core]`, `[tenant]`, `[loop]`, `[plugin]`
//! - [`crate::sections_auth`] — `[auth]`
//! - [`crate::sections_telemetry`] — `[telemetry]`, `[telemetry.log_shipping]`
//! - [`crate::sections_audit`] — `[audit]`
//! - [`crate::sections_routing`] — `[routing]`, `[routing.tiered]`, `[judge]`, `[forge]`
//! - [`crate::sections_gateway`] — `[gateway]`, `[gateway.rate_limit]`
//! - [`crate::sections_planner`] — `[planner]`, `[planner.bias]`
//! - [`crate::sections_channels`] — `[channels]`, `[channels.teams]`, `[channels.matrix]`
//! - [`crate::sections_executor`] — `[document_csv]`, `[document_gsuite]`
//! - [`crate::sections_security`] — `[security]`, `[security.encryption]`, `[security.hipaa]`
//! - [`crate::sections_siem`] — `[siem]` (Splunk HEC / Elastic `_bulk` / Datadog Logs dispatcher)
//! - [`crate::sections_sso`] — `[sso]`, `[sso.oidc.*]`, `[sso.saml.*]`
//! - [`crate::sections_storage`] — `[storage]`, `[storage.tenants.*]`
use serde::{Deserialize, Serialize};

pub use crate::sections_audit::AuditConfig;
pub use crate::sections_auth::AuthConfig;
pub use crate::sections_budget::{BudgetConfig, DelegateConfig, ParallelConfig};
pub use crate::sections_channels::{
    ChannelsConfig, MatrixTomlConfig, TeamsTomlConfig, VoiceTomlConfig, WebInboxTomlConfig,
};
pub use crate::sections_core::{CoreConfig, LoopConfig, PluginConfig, TenantConfig};
pub use crate::sections_executor::{
    CodeExecutorConfig, DocumentCsvConfig, DocumentGsuiteConfig, DEFAULT_CODE_EXECUTOR_PROFILE,
};
pub use crate::sections_gateway::{
    GatewayBehindProxyConfig, GatewayConfig, GatewayRequestAuditConfig,
    GatewaySecurityHeadersConfig,
};
pub use crate::sections_planner::{PlannerBiasConfig, PlannerConfig};
pub use crate::sections_routing::{ForgeConfig, JudgeConfig, RoutingConfig, RoutingTieredConfig};
pub use crate::sections_security::{
    IpAllowlistConfig, SecurityConfig, SecurityEncryptionConfig, SecurityHipaaConfig,
};
pub use crate::sections_siem::SiemConfig;
pub use crate::sections_sso::{SsoConfig, SsoOidcTenant, SsoSamlTenant};
pub use crate::sections_storage::{StorageConfig, StorageTenantConfig};
pub use crate::sections_telemetry::{
    TelemetryConfig, TelemetryPrometheusCardinalityCapConfig, TelemetryPrometheusConfig,
};

/// `[skill_learning]` — autonomous per-worker skill-learning cron.
///
/// Enabled by default so a gateway process starts the shipped
/// learning loop unless an operator explicitly disables it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillLearningConfig {
    /// Spawn the per-worker learning cron from gateway boot.
    pub enabled: bool,
    /// Tick cadence in seconds.
    pub interval_secs: u64,
    /// Optional single tenant UUID to restrict cron spawning.
    pub tenant_filter: Option<String>,
}

impl Default for SkillLearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 3_600,
            tenant_filter: None,
        }
    }
}

#[cfg(test)]
mod skill_learning_config_tests {
    #[test]
    fn skill_learning_toml_parses() {
        let cfg = crate::ArgoConfig::from_toml(
            "[skill_learning]\nenabled = false\ninterval_secs = 5\ntenant_filter = \"00000000-0000-0000-0000-000000000001\"\n",
        )
        .unwrap();
        assert!(!cfg.skill_learning.enabled);
        assert_eq!(cfg.skill_learning.interval_secs, 5);
        assert_eq!(
            cfg.skill_learning.tenant_filter.as_deref(),
            Some("00000000-0000-0000-0000-000000000001")
        );
    }
}

/// `[skill_library]` — prompt-time skill library controls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillLibraryConfig {
    /// `[skill_library.relevance]` selector policy.
    pub relevance: SkillLibraryRelevanceConfig,
}

/// `[skill_library.relevance]` — marketplace prompt-selection caps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillLibraryRelevanceConfig {
    /// Maximum marketplace skills rendered into a single prompt.
    pub max_marketplace_skills: usize,
    /// Minimum marketplace trust label (`verified` by default).
    pub min_trust_grade: String,
}

impl Default for SkillLibraryRelevanceConfig {
    fn default() -> Self {
        Self {
            max_marketplace_skills: 5,
            min_trust_grade: "verified".to_string(),
        }
    }
}

/// `[anomaly]` — session-level worker anomaly detector. Defaults are
/// observe-only; `auto_pause = true` opts into stopping the work loop
/// when the runtime score crosses `threshold`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnomalyConfig {
    /// Pause the worker loop when a score crosses `threshold`.
    pub auto_pause: bool,
    /// Score threshold in `[0.0, 1.0]`.
    pub threshold: f64,
    /// Near-identical step outputs before the loop is considered stuck.
    pub repeated_step_limit: u32,
    /// Step latency cap that contributes a spike score.
    pub max_step_latency_ms: u64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            auto_pause: false,
            threshold: 0.85,
            repeated_step_limit: 3,
            max_step_latency_ms: 120_000,
        }
    }
}

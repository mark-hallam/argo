//! `[…]` section structs that make up [`crate::ArgoConfig`].
//!
//! Each struct binds one TOML section, owns its own `Default` (so
//! defaults stay co-located with the struct), and — where it
//! translates to an in-memory primitive used elsewhere — exposes a
//! single `to_*` / `*_policy` method (e.g. [`BudgetConfig::to_cost_ceilings`]).
use crate::defaults;
use argo_core::RestartPolicy;
use argo_types::CostCeilings;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[core]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Argo home directory; supports `~` expansion.
    pub home: String,
    /// Working workspace path.
    pub workspace: String,
    /// Profile name.
    pub profile: String,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            home: defaults::DEFAULT_HOME.into(),
            workspace: defaults::DEFAULT_WORKSPACE.into(),
            profile: defaults::DEFAULT_PROFILE.into(),
        }
    }
}

impl CoreConfig {
    /// Resolve `home` to an absolute path, expanding `~`.
    #[must_use]
    pub fn resolved_home(&self) -> PathBuf {
        expand_tilde(&self.home)
    }
}

/// `[tenant]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TenantConfig {
    /// `single` or `multi`.
    pub mode: String,
    /// Tenant id used in single-tenant mode.
    pub default_id: String,
    /// `none`, `oidc`, or `header`.
    pub auth_provider: String,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            mode: defaults::DEFAULT_TENANT_MODE.into(),
            default_id: defaults::DEFAULT_TENANT_ID.into(),
            auth_provider: defaults::DEFAULT_AUTH_PROVIDER.into(),
        }
    }
}

/// `[telemetry]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryConfig {
    /// Master enable.
    pub enabled: bool,
    /// `stdout`, `otlp`, or `none`.
    pub exporter: String,
    /// Exporter endpoint (e.g. OTLP HTTP URL).
    pub endpoint: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::DEFAULT_TELEMETRY_ENABLED,
            exporter: defaults::DEFAULT_TELEMETRY_EXPORTER.into(),
            endpoint: String::new(),
        }
    }
}

/// `[audit]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    /// Master enable.
    pub enabled: bool,
    /// Hash algo for the chain (`sha256`).
    pub chain_algo: String,
    /// Export format (`ndjson`).
    pub export_format: String,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            chain_algo: defaults::DEFAULT_AUDIT_CHAIN_ALGO.into(),
            export_format: defaults::DEFAULT_AUDIT_EXPORT_FORMAT.into(),
        }
    }
}

/// `[budget]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    /// Default iteration cap.
    pub default_iterations: u32,
    /// Default token-in cap.
    pub default_tokens_in: u64,
    /// Default token-out cap.
    pub default_tokens_out: u64,
    /// Default wallclock cap (ms).
    pub default_wallclock_ms: u64,
    /// Default spend cap.
    pub default_spend_usd_micros: u64,
    /// Per-session hard cap.
    pub default_spend_per_session_usd_micros: u64,
    /// Per-day hard cap.
    pub default_spend_per_day_usd_micros: u64,
    /// Per-month hard cap.
    pub default_spend_per_month_usd_micros: u64,
    /// Notification thresholds.
    pub notify_at_pct: Vec<u8>,
    /// `halt` | `prompt_user` | `auto_continue`.
    pub on_max_iterations: String,
    /// Hard cap on auto-continue extensions.
    pub auto_continue_max_extensions: u32,
    /// How many iterations each extension grants.
    pub auto_continue_extension_size: u32,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            default_iterations: defaults::DEFAULT_BUDGET_ITERATIONS,
            default_tokens_in: 0,
            default_tokens_out: 0,
            default_wallclock_ms: 0,
            default_spend_usd_micros: 0,
            default_spend_per_session_usd_micros: defaults::DEFAULT_SPEND_PER_SESSION_USD_MICROS,
            default_spend_per_day_usd_micros: defaults::DEFAULT_SPEND_PER_DAY_USD_MICROS,
            default_spend_per_month_usd_micros: defaults::DEFAULT_SPEND_PER_MONTH_USD_MICROS,
            notify_at_pct: defaults::DEFAULT_NOTIFY_AT_PCT.to_vec(),
            on_max_iterations: defaults::DEFAULT_ON_MAX_ITERATIONS.into(),
            auto_continue_max_extensions: defaults::DEFAULT_AUTO_CONTINUE_MAX_EXTENSIONS,
            auto_continue_extension_size: defaults::DEFAULT_AUTO_CONTINUE_EXTENSION_SIZE,
        }
    }
}

impl BudgetConfig {
    /// Bind the `[budget]` block to a [`CostCeilings`] (item C).
    /// `0` for any of the per-scope caps disables that scope.
    #[must_use]
    pub fn to_cost_ceilings(&self) -> CostCeilings {
        CostCeilings {
            per_session_usd_micros: nonzero(self.default_spend_per_session_usd_micros),
            per_day_usd_micros: nonzero(self.default_spend_per_day_usd_micros),
            per_month_usd_micros: nonzero(self.default_spend_per_month_usd_micros),
            notify_at_pct: self.notify_at_pct.clone(),
        }
    }
}

fn nonzero(v: u64) -> Option<u64> {
    if v == 0 {
        None
    } else {
        Some(v)
    }
}

/// `[delegate]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DelegateConfig {
    /// Max nesting depth.
    pub max_spawn_depth: u32,
    /// Cap on concurrent siblings.
    pub max_concurrent_children: u32,
    /// Default child iteration cap.
    pub child_default_iterations: u32,
    /// `explicit` | `share`.
    pub budget_inheritance: String,
    /// Inherit fallback chain (Amendment 7).
    pub fallback_chain_inheritance: bool,
}

impl Default for DelegateConfig {
    fn default() -> Self {
        Self {
            max_spawn_depth: defaults::DEFAULT_MAX_SPAWN_DEPTH,
            max_concurrent_children: defaults::DEFAULT_MAX_CONCURRENT_CHILDREN,
            child_default_iterations: defaults::DEFAULT_CHILD_ITERATIONS,
            budget_inheritance: "explicit".into(),
            fallback_chain_inheritance: true,
        }
    }
}

/// `[parallel]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParallelConfig {
    /// Max parallel tool dispatches.
    pub max_parallel_tools: u32,
    /// Force serial for destructive commands.
    pub destructive_command_serial: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: defaults::DEFAULT_MAX_PARALLEL_TOOLS,
            destructive_command_serial: true,
        }
    }
}

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

/// `[gateway]` (top-level shape; nested adapter configs are
/// deserialized into raw values for the gateway crate to interpret).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayConfig {
    /// Master enable.
    pub enabled: bool,
    /// Adapters to spin up.
    pub adapters: Vec<String>,
    /// Bind host. Default `127.0.0.1`.
    pub host: String,
    /// Bind port. Default [`crate::defaults::DEFAULT_GATEWAY_PORT`].
    pub port: u16,
    /// Public-facing URL clients use to reach this gateway.
    /// Default [`crate::defaults::DEFAULT_GATEWAY_URL`].
    pub url: String,
    /// Orchestrator duty scheduler tick interval (seconds).
    /// Default [`crate::defaults::DEFAULT_DUTY_SCHEDULER_SECS`].
    pub duty_scheduler_secs: u64,
    /// Handoff walker sweep interval (seconds).
    /// Default [`crate::defaults::DEFAULT_HANDOFF_SWEEP_SECS`].
    pub handoff_sweep_secs: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            adapters: Vec::new(),
            host: defaults::DEFAULT_GATEWAY_HOST.into(),
            port: defaults::DEFAULT_GATEWAY_PORT,
            url: defaults::DEFAULT_GATEWAY_URL.into(),
            duty_scheduler_secs: defaults::DEFAULT_DUTY_SCHEDULER_SECS,
            handoff_sweep_secs: defaults::DEFAULT_HANDOFF_SWEEP_SECS,
        }
    }
}

/// `[loop]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoopConfig {
    /// Restart policy (Amendment 13).
    pub on_restart: String,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            on_restart: defaults::DEFAULT_ON_RESTART.into(),
        }
    }
}

impl LoopConfig {
    /// Bind `on_restart` to [`RestartPolicy`]. Unrecognized values
    /// fall back to `prompt_user` (the spec default).
    #[must_use]
    pub fn restart_policy(&self) -> RestartPolicy {
        match self.on_restart.as_str() {
            "auto_resume" => RestartPolicy::AutoResume,
            "discard" => RestartPolicy::Discard,
            _ => RestartPolicy::PromptUser,
        }
    }
}

/// `[plugin]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    /// Plugins to launch on startup.
    pub auto_load: Vec<String>,
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    } else if s == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(s)
}

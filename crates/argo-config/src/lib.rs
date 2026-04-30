//! `argo-config` — `argo.toml` loader.
//!
//! Loads the master config (§3) and binds the v0.2 sections to the
//! in-memory primitives:
//!
//! - `[budget]` → [`argo_types::CostCeilings`] (item C, hard $$$
//!   ceilings) plus the iteration / auto-continue policy.
//! - `[loop.on_restart]` → [`argo_core::RestartPolicy`]
//!   (Amendment 13).
//! - `[routing.policy]` → routing tier name; the actual
//!   `TieredRouting` struct is built upstream from the
//!   `[routing.tiered]` block when policy = `"tiered"`.
//!
//! Provider / gateway / agent / plugin sections are deserialized
//! into typed sub-structs but their values are pass-through; the
//! consuming crates (argo-provider, argo-gateway, argo-core)
//! interpret them.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use argo_core::RestartPolicy;
use argo_types::{ArgoError, ArgoResult, CostCeilings};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level config — every section is optional so partial configs
/// (e.g., a workspace `argo.toml` overriding only `[budget]`) load
/// against the shipped defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArgoConfig {
    /// `[core]`.
    pub core: CoreConfig,
    /// `[tenant]`.
    pub tenant: TenantConfig,
    /// `[telemetry]`.
    pub telemetry: TelemetryConfig,
    /// `[audit]`.
    pub audit: AuditConfig,
    /// `[budget]`.
    pub budget: BudgetConfig,
    /// `[delegate]`.
    pub delegate: DelegateConfig,
    /// `[parallel]`.
    pub parallel: ParallelConfig,
    /// `[routing]`.
    pub routing: RoutingConfig,
    /// `[judge]`.
    pub judge: JudgeConfig,
    /// `[forge]`.
    pub forge: ForgeConfig,
    /// `[gateway]`.
    pub gateway: GatewayConfig,
    /// `[loop]`.
    #[serde(rename = "loop")]
    pub loop_: LoopConfig,
    /// `[plugin]`.
    pub plugin: PluginConfig,
}

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
            home: "~/.argo".into(),
            workspace: ".".into(),
            profile: "default".into(),
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
            mode: "single".into(),
            default_id: "default".into(),
            auth_provider: "none".into(),
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
            enabled: true,
            exporter: "stdout".into(),
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
            chain_algo: "sha256".into(),
            export_format: "ndjson".into(),
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
            default_iterations: 60,
            default_tokens_in: 0,
            default_tokens_out: 0,
            default_wallclock_ms: 0,
            default_spend_usd_micros: 0,
            default_spend_per_session_usd_micros: 1_000_000,
            default_spend_per_day_usd_micros: 10_000_000,
            default_spend_per_month_usd_micros: 100_000_000,
            notify_at_pct: vec![80, 95],
            on_max_iterations: "prompt_user".into(),
            auto_continue_max_extensions: 3,
            auto_continue_extension_size: 30,
        }
    }
}

impl BudgetConfig {
    /// Bind the `[budget]` block to a [`CostCeilings`] (item C).
    /// `0` for any of the per-scope caps disables that scope (the
    /// spec uses `0` as "unset" for the optional `[budget]` fields
    /// like `default_spend_usd_micros`).
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
            max_spawn_depth: 1,
            max_concurrent_children: 4,
            child_default_iterations: 30,
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
            max_parallel_tools: 8,
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
            policy: "fixed".into(),
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
            classify_threshold_ms: 80,
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
            enabled: true,
            model: String::new(),
            rubric: "default".into(),
            verdict_on_error: "partial".into(),
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
            default_examples_min: 3,
            default_iterations: 12,
            budget_per_run_usd_micros: 5_000_000,
            auto_promote_threshold: 0.92,
        }
    }
}

/// `[gateway]` (top-level shape; nested adapter configs are
/// deserialized into raw values for the gateway crate to interpret).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayConfig {
    /// Master enable.
    pub enabled: bool,
    /// Adapters to spin up.
    pub adapters: Vec<String>,
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
            on_restart: "prompt_user".into(),
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

impl ArgoConfig {
    /// Load `argo.toml` from `path`. Missing file ⇒ shipped defaults.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on read or parse failure.
    pub fn load(path: &Path) -> ArgoResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
        let cfg: Self = toml::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
        Ok(cfg)
    }
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

#[cfg(test)]
mod tests;

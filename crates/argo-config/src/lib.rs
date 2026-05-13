//! `argo-config` — workspace-level config loader.
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
//!
//! # Loading
//!
//! Prefer [`load`] (or [`load_from`] in tests) — both honour the
//! layered precedence env > project-local TOML > user-level TOML >
//! built-ins. [`ArgoConfig::load`] is the legacy single-path
//! variant retained for callers that already located the file
//! themselves.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

/// `[budget.ceiling]` TOML section — hard per-session / per-day /
/// per-month USD ceilings consumed by
/// `argo-provider::spend_gate::SpendGate` (Stream
/// `mission-llm-budget-hard-ceiling`).
pub mod budget_ceiling;
pub mod cred;
/// OS-keyring backend for [`cred`] (Stream
/// `mission-system-keyring-integration`). See module docs for service
/// naming and availability semantics.
pub mod cred_keyring;
pub mod defaults;
pub mod loader;
/// `[worker_pool]` TOML section — worker-pool elasticity recommender
/// bounds (`mission-worker-pool-elasticity`).
pub mod worker_pool;
/// Per-provider TOML knobs parsed off `[provider.<name>]` (currently
/// the circuit-breaker overrides — `BreakerSection` /
/// `ProvidersConfig`).
pub mod provider;
mod sandbox;
mod sections;
mod sections_audit;
mod sections_budget;
mod sections_channels;
mod sections_context;
mod sections_core;
mod sections_executor;
mod sections_gateway;
mod sections_routing;
mod sections_security;
mod sections_telemetry;
pub mod store;
pub mod tui;

pub use loader::{load, load_from};
pub use provider::{BreakerSection, ProviderEntry, ProvidersConfig};
pub use sandbox::SandboxConfig;
pub use sections::{
    AuditConfig, BudgetConfig, ChannelsConfig, CoreConfig, DelegateConfig, DocumentCsvConfig,
    ForgeConfig, GatewayConfig, GatewayRateLimitConfig, GatewayRateLimitTenantConfig, JudgeConfig,
    LoopConfig, MatrixTomlConfig, ParallelConfig, PluginConfig, RoutingConfig, RoutingTieredConfig,
    SecurityConfig, SecurityEncryptionConfig, SecurityHipaaConfig, TeamsTomlConfig, TelemetryConfig,
    TenantConfig,
};
pub use sections_context::{
    ActionThrottleConfig, ActionThrottlePolicyConfig, ContextCompressConfig, RuntimeConfig,
    SandboxPrewarmConfig, DEFAULT_ACTION_THROTTLE_AUDIT_SECS,
};
pub use store::{StoreConfig, WalTuningSection};
pub use tui::{TuiConfig, TuiThemeConfig};
pub use worker_pool::{WorkerPoolConfig, WorkerPoolElasticityConfig};

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    /// `[store]` (SQLite-backed session store knobs).
    pub store: StoreConfig,
    /// `[tui]` (interactive TUI surface).
    pub tui: TuiConfig,
    /// `[sandbox]` (sandbox runtime — named-image map etc.).
    pub sandbox: SandboxConfig,
    /// `[document_csv]` (RFC 4180 CSV executor — output_root etc.).
    pub document_csv: DocumentCsvConfig,
    /// `[channels]` (per-channel TOML config blocks).
    pub channels: ChannelsConfig,
    /// `[worker_pool]` — pool elasticity recommender bounds.
    pub worker_pool: WorkerPoolConfig,
    /// `[runtime]` — runtime-side knobs (currently
    /// `[runtime.context_compress]`).
    pub runtime: RuntimeConfig,
    /// `[security]` — at-rest encryption and compliance posture.
    pub security: SecurityConfig,
    /// Bookkeeping for which fields a particular layer explicitly
    /// set. Skipped at serde so TOML parses round-trip cleanly.
    #[serde(skip)]
    pub(crate) overrides: loader::ArgoConfigOverrides,
}


impl ArgoConfig {
    /// Load `argo.toml` from `path`. Missing file ⇒ shipped defaults.
    /// Single-path loader retained for callers that already located
    /// the TOML; new code should prefer [`load`].
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

#[cfg(test)]
mod tests;

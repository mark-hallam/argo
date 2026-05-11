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

pub mod cred;
pub mod defaults;
pub mod loader;
mod sandbox;
mod sections;
mod sections_budget;
pub mod tui;

pub use loader::{load, load_from};
pub use sandbox::SandboxConfig;
pub use sections::{
    AuditConfig, BudgetConfig, CoreConfig, DelegateConfig, ForgeConfig, GatewayConfig, JudgeConfig,
    LoopConfig, ParallelConfig, PluginConfig, RoutingConfig, RoutingTieredConfig, TelemetryConfig,
    TenantConfig,
};
pub use tui::{TuiConfig, TuiThemeConfig};

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
    /// `[tui]` (interactive TUI surface).
    pub tui: TuiConfig,
    /// `[sandbox]` (sandbox runtime — named-image map etc.).
    pub sandbox: SandboxConfig,
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

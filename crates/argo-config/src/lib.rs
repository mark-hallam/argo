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
mod sections_planner;
mod sections_routing;
mod sections_security;
mod sections_streaming_retry;
mod sections_telemetry;
pub mod store;
pub mod tui;
/// `[worker_pool]` TOML section — worker-pool elasticity recommender
/// bounds (`mission-worker-pool-elasticity`).
pub mod worker_pool;

pub use loader::{load, load_from};
pub use provider::{BreakerSection, ProviderEntry, ProvidersConfig};
pub use sandbox::SandboxConfig;
pub use sections::{
    AuditConfig, BudgetConfig, ChannelsConfig, CoreConfig, DelegateConfig, DocumentCsvConfig,
    ForgeConfig, GatewayBehindProxyConfig, GatewayConfig, GatewayRateLimitConfig,
    GatewayRateLimitTenantConfig, IpAllowlistConfig, JudgeConfig, LoopConfig, MatrixTomlConfig,
    ParallelConfig, PlannerBiasConfig, PlannerConfig, PluginConfig, RoutingConfig,
    RoutingTieredConfig, SecurityConfig, SecurityEncryptionConfig, SecurityHipaaConfig,
    TeamsTomlConfig, TelemetryConfig, TenantConfig,
};
pub use sections_context::{
    ActionThrottleConfig, ActionThrottlePolicyConfig, ContextCompressConfig, McpPoolConfig,
    PromptCacheAffinityConfig, RuntimeConfig, SandboxPrewarmConfig,
    DEFAULT_ACTION_THROTTLE_AUDIT_SECS, DEFAULT_MCP_POOL_IDLE_TTL_SECS,
    DEFAULT_MCP_POOL_MAX_CLIENTS,
};
pub use sections_streaming_retry::{
    StreamingRetryConfig, DEFAULT_STREAMING_RETRY_BACKOFF_INITIAL_MS,
    DEFAULT_STREAMING_RETRY_ENABLED, DEFAULT_STREAMING_RETRY_MAX_ATTEMPTS,
};
pub use store::{StoreConfig, WalTuningSection};
pub use tui::{TuiConfig, TuiThemeConfig};
pub use worker_pool::{WorkerPoolConfig, WorkerPoolElasticityConfig};

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Sentinel tenant id stamped on every pre-migration `audit_record`
/// row by migration 63. The default workspace tenant unions this
/// bucket into its read scope so historical audit visibility is
/// preserved without an env-flag flip. [`workspace_tenant_id`] never
/// returns this value — operators running real multi-tenant
/// deployments migrate legacy rows to real tenant ids via the
/// follow-up CLI brief (`mission-audit-legacy-tenant-migrate-cli`).
pub const LEGACY_AUDIT_TENANT_SENTINEL: &str = "__legacy__";

/// Resolve the workspace's stable tenant id (Stream
/// `mission-enterprise-cross-tenant-audit`).
///
/// Precedence:
/// 1. `[tenant] id = "..."` in `<workspace>/argo.toml` if non-empty
///    and not the shipped default (`default`).
/// 2. `ws-<16hex>` derived from a SHA-256 of the workspace root path
///    so fresh single-tenant deployments self-name without operator
///    intervention. Stable across runs on the same path.
///
/// Never returns the [`LEGACY_AUDIT_TENANT_SENTINEL`] (`__legacy__`)
/// — that value belongs exclusively to pre-migration rows.
///
/// ```
/// use argo_config::workspace_tenant_id;
/// use std::path::Path;
/// let id = workspace_tenant_id(Path::new("/some/path"));
/// assert!(id.starts_with("ws-"));
/// assert_ne!(id, "__legacy__");
/// ```
#[must_use]
pub fn workspace_tenant_id(workspace_root: &Path) -> String {
    let cfg_path = workspace_root.join("argo.toml");
    if let Ok(cfg) = ArgoConfig::load(&cfg_path) {
        let explicit = cfg.tenant.default_id.trim();
        if !explicit.is_empty()
            && explicit != defaults::DEFAULT_TENANT_ID
            && explicit != LEGACY_AUDIT_TENANT_SENTINEL
        {
            return explicit.to_string();
        }
    }
    let mut h = Sha256::new();
    h.update(workspace_root.to_string_lossy().as_bytes());
    let digest = h.finalize();
    format!("ws-{}", hex::encode(&digest[..8]))
}

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
    /// `[runtime.context_compress]` and
    /// `[runtime.streaming_retry]`).
    pub runtime: RuntimeConfig,
    /// `[planner]` — planner bias knobs
    /// (`mission-planner-bias-from-template`).
    pub planner: PlannerConfig,
    /// `[security]` — at-rest encryption, compliance posture, and
    /// gateway defence-in-depth gates.
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
#[path = "lib_tests.rs"]
mod tenant_id_tests;
#[cfg(test)]
mod tests;

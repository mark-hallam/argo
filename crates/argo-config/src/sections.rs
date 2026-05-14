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

pub use crate::sections_audit::AuditConfig;
pub use crate::sections_budget::{BudgetConfig, DelegateConfig, ParallelConfig};
pub use crate::sections_channels::{ChannelsConfig, MatrixTomlConfig, TeamsTomlConfig};
pub use crate::sections_core::{CoreConfig, LoopConfig, PluginConfig, TenantConfig};
pub use crate::sections_executor::{DocumentCsvConfig, DocumentGsuiteConfig};
pub use crate::sections_gateway::{
    GatewayBehindProxyConfig, GatewayConfig, GatewayRateLimitConfig, GatewayRateLimitTenantConfig,
    GatewayRequestAuditConfig,
};
pub use crate::sections_planner::{PlannerBiasConfig, PlannerConfig};
pub use crate::sections_routing::{ForgeConfig, JudgeConfig, RoutingConfig, RoutingTieredConfig};
pub use crate::sections_security::{
    IpAllowlistConfig, SecurityConfig, SecurityEncryptionConfig, SecurityHipaaConfig,
};
pub use crate::sections_siem::SiemConfig;
pub use crate::sections_sso::{SsoConfig, SsoOidcTenant, SsoSamlTenant};
pub use crate::sections_telemetry::TelemetryConfig;

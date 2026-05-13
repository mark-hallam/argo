//! Per-provider configuration knobs (currently just the
//! `[provider.<name>.breaker]` block).
//!
//! TOML shape:
//!
//! ```toml
//! [provider.anthropic.breaker]
//! failure_threshold = 5
//! half_open_probe_interval_ms = 30000
//! ```
//!
//! Missing block → all fields fall back to `Default` (matches the
//! `argo-provider::circuit::BreakerConfig::default()` values).
//! Translation into `argo_provider::BreakerConfig` happens upstream
//! (typically in `argo-cli`) so this crate stays free of an
//! `argo-provider` dependency.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Default consecutive failures before the breaker trips Closed→Open
/// (mirrors `argo_provider::circuit::DEFAULT_FAILURE_THRESHOLD`).
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
/// Default base cooldown / half-open probe interval in milliseconds
/// (mirrors `argo_provider::circuit::DEFAULT_COOLDOWN_SECS` × 1000).
pub const DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS: u64 = 30_000;

/// One provider's `[provider.<name>.breaker]` knobs.
///
/// Both fields are optional; missing values resolve to the
/// `argo_provider::circuit::BreakerConfig::default()` constants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakerSection {
    /// Consecutive HTTP-5xx / network failures that trip the
    /// breaker. Must be > 0 if specified.
    pub failure_threshold: Option<u32>,
    /// Cooldown between Open and the half-open probe attempt,
    /// in milliseconds. Must be > 0 if specified.
    pub half_open_probe_interval_ms: Option<u64>,
}

impl BreakerSection {
    /// Effective failure threshold (default if unset).
    #[must_use]
    pub fn resolved_failure_threshold(&self) -> u32 {
        self.failure_threshold.unwrap_or(DEFAULT_FAILURE_THRESHOLD)
    }

    /// Effective half-open probe interval (default if unset).
    #[must_use]
    pub fn resolved_half_open_probe_interval(&self) -> Duration {
        Duration::from_millis(
            self.half_open_probe_interval_ms
                .unwrap_or(DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS),
        )
    }
}

/// One provider's full `[provider.<name>]` block. Only the `breaker`
/// child is wired for now; future per-provider knobs (retries,
/// custom timeouts, …) belong here so the parser surface grows
/// additively.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// Optional breaker overrides.
    pub breaker: Option<BreakerSection>,
}

impl ProviderEntry {
    /// The breaker block, or a fresh default if the operator didn't
    /// declare `[provider.<name>.breaker]`.
    #[must_use]
    pub fn breaker_or_default(&self) -> BreakerSection {
        self.breaker.clone().unwrap_or_default()
    }
}

/// Top-level `[provider.<name>]` registry parsed off an `argo.toml`
/// body. The map key is the provider id (e.g. `"anthropic"`, matching
/// `manifests/providers/anthropic.toml`'s `[provider].id`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProvidersConfig {
    /// `<provider-id> → entry`.
    pub providers: HashMap<String, ProviderEntry>,
}

impl ProvidersConfig {
    /// Lookup by provider id. Missing → fresh default entry.
    #[must_use]
    pub fn entry_or_default(&self, provider_id: &str) -> ProviderEntry {
        self.providers.get(provider_id).cloned().unwrap_or_default()
    }

    /// Parse just the `[provider.*]` tables out of an `argo.toml`
    /// body. Foreign top-level keys are ignored — callers should run
    /// this in addition to (not instead of) the full `ArgoConfig`
    /// loader.
    pub fn parse_from_argo_toml(body: &str) -> ArgoResult<Self> {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default)]
            provider: HashMap<String, ProviderEntry>,
        }
        let wrapper: Wrapper = toml::from_str(body)
            .map_err(|e| ArgoError::Provider(format!("parse argo.toml [provider.*]: {e}")))?;
        let cfg = Self {
            providers: wrapper.provider,
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Hard-reject zero values for breaker knobs (zero threshold or
    /// zero cooldown would either spin or never trip — neither is
    /// useful and both surface as silent operator surprises).
    fn validate(&self) -> ArgoResult<()> {
        for (id, entry) in &self.providers {
            if let Some(b) = &entry.breaker {
                if let Some(0) = b.failure_threshold {
                    return Err(ArgoError::Provider(format!(
                        "[provider.{id}.breaker].failure_threshold must be > 0"
                    )));
                }
                if let Some(0) = b.half_open_probe_interval_ms {
                    return Err(ArgoError::Provider(format!(
                        "[provider.{id}.breaker].half_open_probe_interval_ms must be > 0"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;

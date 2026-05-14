//! Shared anomaly alert payloads.

use crate::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Event-source anomaly class.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyKind {
    /// Audit chain verification found tampering or broken linkage.
    ChainBreak,
    /// Provider circuit breaker transitioned to open.
    ProviderStorm,
    /// A hard budget ceiling blocked work.
    BudgetCeiling,
    /// Gateway startup health check failed before bind.
    GatewayBootFail,
    /// Extension point for process-local emitters.
    Custom(String),
}

impl std::fmt::Display for AnomalyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChainBreak => f.write_str("chain_break"),
            Self::ProviderStorm => f.write_str("provider_storm"),
            Self::BudgetCeiling => f.write_str("budget_ceiling"),
            Self::GatewayBootFail => f.write_str("gateway_boot_fail"),
            Self::Custom(s) => f.write_str(s),
        }
    }
}

impl AnomalyKind {
    /// Parse a TOML routing key.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "chain_break" => Self::ChainBreak,
            "provider_storm" => Self::ProviderStorm,
            "budget_ceiling" => Self::BudgetCeiling,
            "gateway_boot_fail" => Self::GatewayBootFail,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Anomaly severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Operator-visible warning.
    Warn,
    /// Critical page-worthy event.
    Critical,
}

impl Severity {
    /// Parse config value, defaulting unknown strings to `Warn`.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "critical" => Self::Critical,
            _ => Self::Warn,
        }
    }
}

/// Alert payload emitted by anomaly sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalyAlert {
    /// Event-source anomaly kind.
    pub kind: AnomalyKind,
    /// Severity used by dispatch thresholding.
    pub severity: Severity,
    /// Human-readable operator message.
    pub message: String,
    /// Structured context carried into channel payloads.
    pub context: BTreeMap<String, String>,
}

/// Async dispatch surface shared by all anomaly emitters.
#[async_trait]
pub trait AnomalyDispatch: Send + Sync {
    /// Dispatch `alert` to configured channels.
    async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()>;
}

//! Shared anomaly alert payloads.

use crate::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Event-source anomaly class.
///
/// # Examples
///
/// ```
/// use argo_types::AnomalyKind;
/// let kind = AnomalyKind::ChainBreak;
/// assert_eq!(kind.to_string(), "chain_break");
/// let custom = AnomalyKind::Custom("disk_full".into());
/// assert_eq!(custom.to_string(), "disk_full");
/// ```
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
    ///
    /// Unknown strings round-trip through [`AnomalyKind::Custom`].
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_types::AnomalyKind;
    /// assert_eq!(AnomalyKind::parse("chain_break"), AnomalyKind::ChainBreak);
    /// assert_eq!(AnomalyKind::parse("unrecognized"), AnomalyKind::Custom("unrecognized".into()));
    /// ```
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
///
/// # Examples
///
/// ```
/// use argo_types::Severity;
/// assert!(Severity::Critical > Severity::Warn);
/// let json = serde_json::to_string(&Severity::Critical).unwrap();
/// assert_eq!(json, "\"critical\"");
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_types::Severity;
    /// assert_eq!(Severity::parse("critical"), Severity::Critical);
    /// assert_eq!(Severity::parse("typo"), Severity::Warn);
    /// ```
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "critical" => Self::Critical,
            _ => Self::Warn,
        }
    }
}

/// Alert payload emitted by anomaly sources.
///
/// # Examples
///
/// ```
/// use argo_types::{AnomalyAlert, AnomalyKind, Severity};
/// use std::collections::BTreeMap;
/// let alert = AnomalyAlert {
///     kind: AnomalyKind::BudgetCeiling,
///     severity: Severity::Critical,
///     message: "per-day cap reached".into(),
///     context: BTreeMap::from([("tenant".into(), "acme".into())]),
/// };
/// assert_eq!(alert.context["tenant"], "acme");
/// ```
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
///
/// # Examples
///
/// A no-op dispatch impl used in tests:
///
/// ```
/// use argo_types::{AnomalyAlert, AnomalyDispatch, ArgoResult};
/// use async_trait::async_trait;
///
/// struct NullDispatch;
///
/// #[async_trait]
/// impl AnomalyDispatch for NullDispatch {
///     async fn dispatch(&self, _alert: AnomalyAlert) -> ArgoResult<()> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait AnomalyDispatch: Send + Sync {
    /// Dispatch `alert` to configured channels.
    async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()>;
}
